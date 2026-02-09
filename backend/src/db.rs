use rusqlite::Connection;

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
