use rusqlite::Connection;
use crate::semantic::{SemanticIndex, PostData};

pub fn initialize(conn: &Connection) {
    conn.execute_batch("PRAGMA journal_mode=WAL;").ok();
    conn.execute_batch("PRAGMA foreign_keys=ON;").ok();

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS blogs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT DEFAULT '',
            manage_key_hash TEXT NOT NULL,
            is_public INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS posts (
            id TEXT PRIMARY KEY,
            blog_id TEXT NOT NULL REFERENCES blogs(id),
            title TEXT NOT NULL,
            slug TEXT NOT NULL,
            content TEXT NOT NULL DEFAULT '',
            content_html TEXT NOT NULL DEFAULT '',
            summary TEXT DEFAULT '',
            tags TEXT DEFAULT '[]',
            status TEXT DEFAULT 'draft',
            published_at TEXT,
            author_name TEXT DEFAULT '',
            metadata TEXT DEFAULT '{}',
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            UNIQUE(blog_id, slug)
        );

        CREATE TABLE IF NOT EXISTS comments (
            id TEXT PRIMARY KEY,
            post_id TEXT NOT NULL REFERENCES posts(id),
            author_name TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_posts_blog_id ON posts(blog_id);
        CREATE INDEX IF NOT EXISTS idx_posts_slug ON posts(blog_id, slug);
        CREATE INDEX IF NOT EXISTS idx_posts_status ON posts(status);
        CREATE INDEX IF NOT EXISTS idx_comments_post_id ON comments(post_id);

        CREATE TABLE IF NOT EXISTS post_views (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            viewer_ip TEXT DEFAULT '',
            user_agent TEXT DEFAULT '',
            viewed_at TEXT DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_post_views_post_id ON post_views(post_id);
        CREATE INDEX IF NOT EXISTS idx_post_views_viewed_at ON post_views(viewed_at);
        CREATE INDEX IF NOT EXISTS idx_post_views_composite ON post_views(post_id, viewed_at);
        ",
    )
    .expect("Failed to initialize database");

    // Migration: add is_pinned column to posts
    let has_pinned: bool = conn
        .prepare("SELECT is_pinned FROM posts LIMIT 0")
        .is_ok();
    if !has_pinned {
        conn.execute_batch("ALTER TABLE posts ADD COLUMN is_pinned INTEGER DEFAULT 0;")
            .ok();
    }

    // Migration: add scheduled_at column to posts
    let has_scheduled_at: bool = conn
        .prepare("SELECT scheduled_at FROM posts LIMIT 0")
        .is_ok();
    if !has_scheduled_at {
        conn.execute_batch("ALTER TABLE posts ADD COLUMN scheduled_at TEXT;")
            .ok();
    }

    // FTS5 full-text search index
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS posts_fts USING fts5(
            post_id UNINDEXED,
            blog_id UNINDEXED,
            title,
            content,
            tags,
            summary,
            author_name,
            tokenize='porter unicode61'
        );",
    )
    .expect("Failed to create FTS5 table");

    // Rebuild FTS index from existing posts (idempotent — clears and repopulates)
    rebuild_fts_index(conn);
    initialize_webhooks(conn);
    initialize_reactions(conn);
}

/// Rebuild the FTS5 index from the posts table. Called on startup.
pub fn rebuild_fts_index(conn: &Connection) {
    conn.execute("DELETE FROM posts_fts", []).ok();
    conn.execute_batch(
        "INSERT INTO posts_fts (post_id, blog_id, title, content, tags, summary, author_name)
         SELECT id, blog_id, title, content, tags, summary, author_name FROM posts
         WHERE status = 'published';",
    )
    .ok();
}

/// Upsert a post into the FTS index (call after create/update/publish).
pub fn upsert_fts(conn: &Connection, post_id: &str) {
    // Remove old entry
    conn.execute("DELETE FROM posts_fts WHERE post_id = ?1", [post_id])
        .ok();
    // Insert if published
    conn.execute(
        "INSERT INTO posts_fts (post_id, blog_id, title, content, tags, summary, author_name)
         SELECT id, blog_id, title, content, tags, summary, author_name FROM posts
         WHERE id = ?1 AND status = 'published'",
        [post_id],
    )
    .ok();
}

/// Remove a post from the FTS index (call after delete).
pub fn delete_fts(conn: &Connection, post_id: &str) {
    conn.execute("DELETE FROM posts_fts WHERE post_id = ?1", [post_id])
        .ok();
}

// ─── Semantic Index Helpers ───

/// Rebuild the semantic TF-IDF index from all published posts. Called on startup.
pub fn rebuild_semantic_index(conn: &Connection, index: &SemanticIndex) {
    let mut stmt = conn.prepare(
        "SELECT id, blog_id, title, content, tags, summary FROM posts WHERE status = 'published'"
    ).expect("prepare semantic index query");

    let posts: Vec<PostData> = stmt.query_map([], |row| {
        Ok(PostData {
            post_id: row.get(0)?,
            blog_id: row.get(1)?,
            title: row.get(2)?,
            content: row.get(3)?,
            tags: row.get(4)?,
            summary: row.get(5)?,
        })
    }).expect("query semantic posts")
    .filter_map(|r| r.ok())
    .collect();

    index.rebuild(posts);
}

/// Upsert a post into the semantic index. Call after create/update/publish.
pub fn upsert_semantic(conn: &Connection, post_id: &str, index: &SemanticIndex) {
    // Check if post exists and is published
    let result = conn.query_row(
        "SELECT id, blog_id, title, content, tags, summary FROM posts WHERE id = ?1 AND status = 'published'",
        [post_id],
        |row| Ok(PostData {
            post_id: row.get(0)?,
            blog_id: row.get(1)?,
            title: row.get(2)?,
            content: row.get(3)?,
            tags: row.get(4)?,
            summary: row.get(5)?,
        }),
    );
    match result {
        Ok(post) => index.upsert(post),
        Err(_) => index.remove(post_id), // Not published — remove from index
    }
}

/// Remove a post from the semantic index.
pub fn delete_semantic(post_id: &str, index: &SemanticIndex) {
    index.remove(post_id);
}

// ─── Webhooks ───

pub fn initialize_webhooks(conn: &Connection) {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS webhooks (
            id TEXT PRIMARY KEY,
            blog_id TEXT NOT NULL REFERENCES blogs(id) ON DELETE CASCADE,
            url TEXT NOT NULL,
            events TEXT NOT NULL DEFAULT '[]',
            secret TEXT DEFAULT NULL,
            is_active INTEGER DEFAULT 1,
            created_at TEXT DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_webhooks_blog_id ON webhooks(blog_id);

        CREATE TABLE IF NOT EXISTS webhook_deliveries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            webhook_id TEXT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
            event TEXT NOT NULL,
            status_code INTEGER,
            success INTEGER DEFAULT 0,
            error TEXT DEFAULT NULL,
            delivered_at TEXT DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_webhook_id ON webhook_deliveries(webhook_id);
        CREATE INDEX IF NOT EXISTS idx_webhook_deliveries_delivered_at ON webhook_deliveries(delivered_at);
        ",
    )
    .expect("Failed to initialize webhooks tables");
}

// ─── Post Scheduling ───

/// Publish all posts whose scheduled_at time has passed.
/// Returns a list of (post_id, blog_id) for each post that was published.
pub fn publish_scheduled_posts(conn: &Connection) -> Vec<(String, String)> {
    let now = chrono::Utc::now().to_rfc3339();

    // Find all scheduled posts whose time has arrived
    let mut stmt = conn.prepare(
        "SELECT id, blog_id, scheduled_at FROM posts WHERE status = 'scheduled' AND scheduled_at <= ?1"
    ).unwrap_or_else(|_| panic!("Failed to prepare scheduled posts query"));

    let due_posts: Vec<(String, String, String)> = stmt.query_map([&now], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    }).unwrap_or_else(|_| panic!("Failed to query scheduled posts"))
    .filter_map(|r| r.ok())
    .collect();

    let mut published = Vec::new();

    for (post_id, blog_id, scheduled_at) in &due_posts {
        // Transition: scheduled → published, set published_at to scheduled_at
        let updated = conn.execute(
            "UPDATE posts SET status = 'published', published_at = ?1, updated_at = datetime('now') WHERE id = ?2 AND status = 'scheduled'",
            rusqlite::params![scheduled_at, post_id],
        ).unwrap_or(0);

        if updated > 0 {
            // Update FTS index
            upsert_fts(conn, post_id);
            published.push((post_id.clone(), blog_id.clone()));
        }
    }

    published
}

// ─── Post Reactions ───

pub fn initialize_reactions(conn: &Connection) {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS post_reactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            emoji TEXT NOT NULL,
            client_ip TEXT DEFAULT '',
            created_at TEXT DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_post_reactions_post_id ON post_reactions(post_id);
        CREATE INDEX IF NOT EXISTS idx_post_reactions_composite ON post_reactions(post_id, emoji);
        ",
    )
    .expect("Failed to initialize reactions table");
}
