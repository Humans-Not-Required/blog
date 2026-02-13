use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::State;
use serde::{Deserialize, Serialize};

use crate::auth::{BlogToken, generate_key, hash_key};
use crate::rate_limit::{ClientIp, RateLimiter};
use crate::semantic::SemanticIndex;
use crate::DbPool;

type ContentResult = Result<(Status, (rocket::http::ContentType, String)), (Status, Json<ApiError>)>;

pub struct RateLimiters {
    pub blog_creation: RateLimiter,
    pub comment_creation: RateLimiter,
}

// ─── Models ───

#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
}

fn err(status: Status, msg: &str, code: &str) -> (Status, Json<ApiError>) {
    (status, Json(ApiError { error: msg.to_string(), code: code.to_string() }))
}

fn db_err(msg: &str) -> (Status, Json<ApiError>) {
    err(Status::InternalServerError, msg, "DB_ERROR")
}

#[derive(Serialize)]
pub struct BlogResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_public: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize)]
pub struct BlogCreated {
    pub id: String,
    pub name: String,
    pub manage_key: String,
    pub view_url: String,
    pub manage_url: String,
    pub api_base: String,
}

#[derive(Serialize)]
pub struct PostResponse {
    pub id: String,
    pub blog_id: String,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub content_html: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub status: String,
    pub published_at: Option<String>,
    pub author_name: String,
    pub created_at: String,
    pub updated_at: String,
    pub comment_count: i64,
    pub word_count: u64,
    pub reading_time_minutes: u32,
    pub view_count: i64,
    pub is_pinned: bool,
}

fn compute_word_count(markdown: &str) -> u64 {
    markdown.split_whitespace().count() as u64
}

fn compute_reading_time(word_count: u64) -> u32 {
    // 200 words per minute, minimum 1 minute
    ((word_count as f64 / 200.0).ceil() as u32).max(1)
}

#[derive(Serialize)]
pub struct CommentResponse {
    pub id: String,
    pub post_id: String,
    pub author_name: String,
    pub content: String,
    pub created_at: String,
}

// ─── Request bodies ───

#[derive(Deserialize)]
pub struct CreateBlogReq {
    pub name: String,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateBlogReq {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_public: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreatePostReq {
    pub title: String,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub author_name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePostReq {
    pub title: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
    pub author_name: Option<String>,
    pub slug: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateCommentReq {
    pub author_name: String,
    pub content: String,
}

// ─── Helpers ───

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn render_markdown(md: &str) -> String {
    use pulldown_cmark::{Parser, Options, html};
    let parser = Parser::new_ext(md, Options::all());
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn verify_blog_key(conn: &rusqlite::Connection, blog_id: &str, token: &BlogToken) -> Result<(), (Status, Json<ApiError>)> {
    let hash: String = conn
        .query_row("SELECT manage_key_hash FROM blogs WHERE id = ?1", [blog_id], |r| r.get(0))
        .map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))?;
    if hash_key(&token.0) != hash {
        return Err(err(Status::Unauthorized, "Invalid manage key", "UNAUTHORIZED"));
    }
    Ok(())
}

// ─── Routes ───

#[get("/health")]
pub fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok", "version": "0.1.0"}))
}

#[get("/openapi.json")]
pub fn openapi() -> (rocket::http::ContentType, &'static str) {
    (rocket::http::ContentType::JSON, include_str!("../openapi.json"))
}

#[get("/llms.txt")]
pub fn llms_txt() -> (Status, (rocket::http::ContentType, String)) {
    (Status::Ok, (rocket::http::ContentType::Plain,
        "# Blog Platform API\n\
         > API-first blogging platform for AI agents\n\n\
         ## Base URL\n\
         /api/v1\n\n\
         ## Endpoints\n\
         - POST /blogs - Create a blog (returns manage_key)\n\
         - GET /blogs - List public blogs\n\
         - GET /blogs/:id - Get blog\n\
         - GET /blogs/:id/posts - List posts\n\
         - GET /blogs/:id/posts/:slug - Get post\n\
         - POST /blogs/:id/posts - Create post (auth required)\n\
         - GET /blogs/:id/feed.rss - RSS feed\n\
         - GET /blogs/:id/feed.json - JSON feed\n\n\
         ## Auth\n\
         Bearer token, X-API-Key header, or ?key= query param\n".to_string()
    ))
}

#[post("/blogs", format = "json", data = "<req>")]
pub fn create_blog(req: Json<CreateBlogReq>, client_ip: ClientIp, limiters: &State<RateLimiters>, db: &State<DbPool>) -> Result<(Status, Json<BlogCreated>), (Status, Json<ApiError>)> {
    let rl = limiters.blog_creation.check_default(&client_ip.0);
    if !rl.allowed {
        return Err(err(Status::TooManyRequests, &format!("Rate limit exceeded. Try again in {} seconds", rl.reset_secs), "RATE_LIMIT_EXCEEDED"));
    }
    let name = req.name.trim();
    if name.is_empty() {
        return Err(err(Status::UnprocessableEntity, "Name is required", "VALIDATION_ERROR"));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let key = generate_key("blog");
    let hash = hash_key(&key);
    let desc = req.description.as_deref().unwrap_or("");
    let is_public = req.is_public.unwrap_or(false) as i32;

    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO blogs (id, name, description, manage_key_hash, is_public) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, name, desc, hash, is_public],
    ).map_err(|e| db_err(&e.to_string()))?;

    Ok((Status::Created, Json(BlogCreated {
        id: id.clone(),
        name: name.to_string(),
        manage_key: key,
        view_url: format!("/blog/{}", id),
        manage_url: format!("/blog/{}?key=<manage_key>", id),
        api_base: format!("/api/v1/blogs/{}", id),
    })))
}

#[get("/blogs")]
pub fn list_blogs(db: &State<DbPool>) -> Result<Json<Vec<BlogResponse>>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, description, is_public, created_at, updated_at FROM blogs WHERE is_public = 1 ORDER BY created_at DESC"
    ).map_err(|e| db_err(&e.to_string()))?;

    let blogs = stmt.query_map([], |row| {
        Ok(BlogResponse {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            is_public: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(blogs))
}

#[get("/blogs/<blog_id>")]
pub fn get_blog(blog_id: &str, db: &State<DbPool>) -> Result<Json<BlogResponse>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT id, name, description, is_public, created_at, updated_at FROM blogs WHERE id = ?1",
        [blog_id],
        |row| Ok(BlogResponse {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            is_public: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }),
    ).map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))
    .map(Json)
}

#[patch("/blogs/<blog_id>", format = "json", data = "<req>")]
pub fn update_blog(blog_id: &str, req: Json<UpdateBlogReq>, token: BlogToken, db: &State<DbPool>) -> Result<Json<BlogResponse>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    let current = conn.query_row(
        "SELECT name, description, is_public FROM blogs WHERE id = ?1", [blog_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?)),
    ).map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))?;

    let name = req.name.as_deref().unwrap_or(&current.0);
    let desc = req.description.as_deref().unwrap_or(&current.1);
    let is_public = req.is_public.map(|b| b as i32).unwrap_or(current.2);

    conn.execute(
        "UPDATE blogs SET name = ?1, description = ?2, is_public = ?3, updated_at = datetime('now') WHERE id = ?4",
        rusqlite::params![name, desc, is_public, blog_id],
    ).map_err(|e| db_err(&e.to_string()))?;

    conn.query_row(
        "SELECT id, name, description, is_public, created_at, updated_at FROM blogs WHERE id = ?1",
        [blog_id],
        |row| Ok(BlogResponse {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            is_public: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        }),
    ).map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))
    .map(Json)
}

#[post("/blogs/<blog_id>/posts", format = "json", data = "<req>")]
pub fn create_post(blog_id: &str, req: Json<CreatePostReq>, token: BlogToken, db: &State<DbPool>, sem: &State<SemanticIndex>) -> Result<(Status, Json<PostResponse>), (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    let title = req.title.trim();
    if title.is_empty() {
        return Err(err(Status::UnprocessableEntity, "Title is required", "VALIDATION_ERROR"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let slug = req.slug.as_deref().map(slugify).unwrap_or_else(|| slugify(title));
    let content = req.content.as_deref().unwrap_or("");
    let content_html = render_markdown(content);
    let summary = req.summary.as_deref().unwrap_or("");
    let tags = serde_json::to_string(&req.tags.as_deref().unwrap_or(&[])).unwrap_or_else(|_| "[]".to_string());
    let status = req.status.as_deref().unwrap_or("draft");
    let author_name = req.author_name.as_deref().unwrap_or("");
    let published_at: Option<String> = if status == "published" { Some(chrono::Utc::now().to_rfc3339()) } else { None };

    conn.execute(
        "INSERT INTO posts (id, blog_id, title, slug, content, content_html, summary, tags, status, published_at, author_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        rusqlite::params![id, blog_id, title, slug, content, content_html, summary, tags, status, published_at, author_name],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            err(Status::Conflict, "A post with this slug already exists", "SLUG_CONFLICT")
        } else {
            db_err(&e.to_string())
        }
    })?;

    // Update FTS + semantic index
    crate::db::upsert_fts(&conn, &id);
    crate::db::upsert_semantic(&conn, &id, sem);

    let post = query_post(&conn, &id)?;
    Ok((Status::Created, Json(post)))
}

fn query_post(conn: &rusqlite::Connection, post_id: &str) -> Result<PostResponse, (Status, Json<ApiError>)> {
    conn.query_row(
        "SELECT p.id, p.blog_id, p.title, p.slug, p.content, p.content_html, p.summary, p.tags, p.status, p.published_at, p.author_name, p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as comment_count,
                (SELECT COUNT(*) FROM post_views v WHERE v.post_id = p.id) as view_count,
                COALESCE(p.is_pinned, 0) as is_pinned
         FROM posts p WHERE p.id = ?1",
        [post_id],
        |row| {
            let content: String = row.get(4)?;
            let wc = compute_word_count(&content);
            Ok(PostResponse {
                id: row.get(0)?,
                blog_id: row.get(1)?,
                title: row.get(2)?,
                slug: row.get(3)?,
                content,
                content_html: row.get(5)?,
                summary: row.get(6)?,
                tags: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                status: row.get(8)?,
                published_at: row.get(9)?,
                author_name: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                comment_count: row.get(13)?,
                word_count: wc,
                reading_time_minutes: compute_reading_time(wc),
                view_count: row.get(14)?,
                is_pinned: row.get::<_, i32>(15)? != 0,
            })
        },
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))
}

#[get("/blogs/<blog_id>/posts?<tag>&<limit>&<offset>")]
pub fn list_posts(blog_id: &str, tag: Option<&str>, limit: Option<i64>, offset: Option<i64>, token: Option<BlogToken>, db: &State<DbPool>) -> Result<Json<Vec<PostResponse>>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    // Check blog exists
    conn.query_row("SELECT 1 FROM blogs WHERE id = ?1", [blog_id], |_| Ok(()))
        .map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))?;

    let has_manage = token.map(|t| hash_key(&t.0) == conn.query_row("SELECT manage_key_hash FROM blogs WHERE id = ?1", [blog_id], |r| r.get::<_, String>(0)).unwrap_or_default()).unwrap_or(false);

    let mut sql = String::from(
        "SELECT p.id, p.blog_id, p.title, p.slug, p.content, p.content_html, p.summary, p.tags, p.status, p.published_at, p.author_name, p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as comment_count,
                (SELECT COUNT(*) FROM post_views v WHERE v.post_id = p.id) as view_count,
                COALESCE(p.is_pinned, 0) as is_pinned
         FROM posts p WHERE p.blog_id = ?1"
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(blog_id.to_string())];

    if !has_manage {
        sql.push_str(" AND p.status = 'published'");
    }

    if let Some(t) = tag {
        params.push(Box::new(format!("%\"{}\"%" , t)));
        sql.push_str(&format!(" AND p.tags LIKE ?{}", params.len()));
    }

    sql.push_str(" ORDER BY COALESCE(p.is_pinned, 0) DESC, p.published_at DESC NULLS LAST, p.created_at DESC");

    let lim = limit.unwrap_or(50).clamp(1, 100);
    let off = offset.unwrap_or(0).max(0);
    params.push(Box::new(lim));
    sql.push_str(&format!(" LIMIT ?{}", params.len()));
    params.push(Box::new(off));
    sql.push_str(&format!(" OFFSET ?{}", params.len()));

    let mut stmt = conn.prepare(&sql).map_err(|e| db_err(&e.to_string()))?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let posts = stmt.query_map(param_refs.as_slice(), |row| {
        let content: String = row.get(4)?;
        let wc = compute_word_count(&content);
        Ok(PostResponse {
            id: row.get(0)?,
            blog_id: row.get(1)?,
            title: row.get(2)?,
            slug: row.get(3)?,
            content,
            content_html: row.get(5)?,
            summary: row.get(6)?,
            tags: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
            status: row.get(8)?,
            published_at: row.get(9)?,
            author_name: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
            comment_count: row.get(13)?,
            word_count: wc,
            reading_time_minutes: compute_reading_time(wc),
            view_count: row.get(14)?,
            is_pinned: row.get::<_, i32>(15)? != 0,
        })
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(posts))
}

#[get("/blogs/<blog_id>/posts/<slug>", rank = 2)]
pub fn get_post_by_slug(blog_id: &str, slug: &str, client_ip: ClientIp, db: &State<DbPool>) -> Result<Json<PostResponse>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    let post = conn.query_row(
        "SELECT p.id, p.blog_id, p.title, p.slug, p.content, p.content_html, p.summary, p.tags, p.status, p.published_at, p.author_name, p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as comment_count,
                (SELECT COUNT(*) FROM post_views v WHERE v.post_id = p.id) as view_count,
                COALESCE(p.is_pinned, 0) as is_pinned
         FROM posts p WHERE p.blog_id = ?1 AND p.slug = ?2",
        rusqlite::params![blog_id, slug],
        |row| {
            let content: String = row.get(4)?;
            let wc = compute_word_count(&content);
            Ok(PostResponse {
                id: row.get(0)?,
                blog_id: row.get(1)?,
                title: row.get(2)?,
                slug: row.get(3)?,
                content,
                content_html: row.get(5)?,
                summary: row.get(6)?,
                tags: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
                status: row.get(8)?,
                published_at: row.get(9)?,
                author_name: row.get(10)?,
                created_at: row.get(11)?,
                updated_at: row.get(12)?,
                comment_count: row.get(13)?,
                word_count: wc,
                reading_time_minutes: compute_reading_time(wc),
                view_count: row.get(14)?,
                is_pinned: row.get::<_, i32>(15)? != 0,
            })
        },
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))?;

    // Record view (fire-and-forget, don't fail the request if this errors)
    let _ = conn.execute(
        "INSERT INTO post_views (post_id, viewer_ip) VALUES (?1, ?2)",
        rusqlite::params![post.id, client_ip.0],
    );

    Ok(Json(post))
}

#[patch("/blogs/<blog_id>/posts/<post_id>", format = "json", data = "<req>")]
pub fn update_post(blog_id: &str, post_id: &str, req: Json<UpdatePostReq>, token: BlogToken, db: &State<DbPool>, sem: &State<SemanticIndex>) -> Result<Json<PostResponse>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    // Get current post
    let current = conn.query_row(
        "SELECT title, slug, content, summary, tags, status, author_name, published_at FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, Option<String>>(7)?,
        )),
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))?;

    let title = req.title.as_deref().unwrap_or(&current.0);
    let slug = req.slug.as_deref().map(slugify).unwrap_or(current.1);
    let content = req.content.as_deref().unwrap_or(&current.2);
    let content_html = if req.content.is_some() { render_markdown(content) } else { String::new() };
    let summary = req.summary.as_deref().unwrap_or(&current.3);
    let tags = req.tags.as_ref().map(|t| serde_json::to_string(t).unwrap_or_else(|_| "[]".to_string())).unwrap_or(current.4);
    let new_status = req.status.as_deref().unwrap_or(&current.5);
    let author_name = req.author_name.as_deref().unwrap_or(&current.6);

    // Set published_at on first publish
    let published_at = if new_status == "published" && current.7.is_none() {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        current.7
    };

    // Re-render HTML if content changed
    let final_html = if req.content.is_some() { content_html } else {
        conn.query_row("SELECT content_html FROM posts WHERE id = ?1", [post_id], |r| r.get::<_, String>(0)).unwrap_or_default()
    };

    conn.execute(
        "UPDATE posts SET title=?1, slug=?2, content=?3, content_html=?4, summary=?5, tags=?6, status=?7, published_at=?8, author_name=?9, updated_at=datetime('now') WHERE id=?10 AND blog_id=?11",
        rusqlite::params![title, slug, content, final_html, summary, tags, new_status, published_at, author_name, post_id, blog_id],
    ).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            err(Status::Conflict, "A post with this slug already exists", "SLUG_CONFLICT")
        } else {
            db_err(&e.to_string())
        }
    })?;

    // Update FTS + semantic index
    crate::db::upsert_fts(&conn, post_id);
    crate::db::upsert_semantic(&conn, post_id, sem);

    let post = query_post(&conn, post_id)?;
    Ok(Json(post))
}

#[delete("/blogs/<blog_id>/posts/<post_id>")]
pub fn delete_post(blog_id: &str, post_id: &str, token: BlogToken, db: &State<DbPool>, sem: &State<SemanticIndex>) -> Result<Json<serde_json::Value>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    // Delete comments first, then post views
    conn.execute("DELETE FROM comments WHERE post_id = ?1", [post_id]).ok();
    conn.execute("DELETE FROM post_views WHERE post_id = ?1", [post_id]).ok();
    let deleted = conn.execute(
        "DELETE FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
    ).map_err(|e| db_err(&e.to_string()))?;

    if deleted == 0 {
        return Err(err(Status::NotFound, "Post not found", "NOT_FOUND"));
    }

    // Remove from FTS + semantic index
    crate::db::delete_fts(&conn, post_id);
    crate::db::delete_semantic(post_id, sem);

    Ok(Json(serde_json::json!({"deleted": true})))
}

#[post("/blogs/<blog_id>/posts/<post_id>/comments", format = "json", data = "<req>")]
pub fn create_comment(blog_id: &str, post_id: &str, req: Json<CreateCommentReq>, client_ip: ClientIp, limiters: &State<RateLimiters>, db: &State<DbPool>) -> Result<(Status, Json<CommentResponse>), (Status, Json<ApiError>)> {
    let rl = limiters.comment_creation.check_default(&client_ip.0);
    if !rl.allowed {
        return Err(err(Status::TooManyRequests, &format!("Rate limit exceeded. Try again in {} seconds", rl.reset_secs), "RATE_LIMIT_EXCEEDED"));
    }
    let conn = db.lock().unwrap();
    // Verify post exists and belongs to blog
    conn.query_row(
        "SELECT 1 FROM posts WHERE id = ?1 AND blog_id = ?2 AND status = 'published'",
        rusqlite::params![post_id, blog_id],
        |_| Ok(()),
    ).map_err(|_| err(Status::NotFound, "Post not found or not published", "NOT_FOUND"))?;

    let author = req.author_name.trim();
    let content = req.content.trim();
    if author.is_empty() || content.is_empty() {
        return Err(err(Status::UnprocessableEntity, "Author name and content are required", "VALIDATION_ERROR"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO comments (id, post_id, author_name, content) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![id, post_id, author, content],
    ).map_err(|e| db_err(&e.to_string()))?;

    let comment = conn.query_row(
        "SELECT id, post_id, author_name, content, created_at FROM comments WHERE id = ?1",
        [&id],
        |row| Ok(CommentResponse {
            id: row.get(0)?,
            post_id: row.get(1)?,
            author_name: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        }),
    ).map_err(|e| db_err(&e.to_string()))?;

    Ok((Status::Created, Json(comment)))
}

#[get("/blogs/<blog_id>/posts/<post_id>/comments")]
pub fn list_comments(blog_id: &str, post_id: &str, db: &State<DbPool>) -> Result<Json<Vec<CommentResponse>>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    // Verify post exists
    conn.query_row(
        "SELECT 1 FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
        |_| Ok(()),
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))?;

    let mut stmt = conn.prepare(
        "SELECT id, post_id, author_name, content, created_at FROM comments WHERE post_id = ?1 ORDER BY created_at ASC"
    ).map_err(|e| db_err(&e.to_string()))?;

    let comments = stmt.query_map([post_id], |row| {
        Ok(CommentResponse {
            id: row.get(0)?,
            post_id: row.get(1)?,
            author_name: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        })
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(comments))
}

// ─── Comment Moderation ───

#[delete("/blogs/<blog_id>/posts/<post_id>/comments/<comment_id>")]
pub fn delete_comment(blog_id: &str, post_id: &str, comment_id: &str, token: BlogToken, db: &State<DbPool>) -> Result<Json<serde_json::Value>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    // Verify post belongs to blog
    conn.query_row(
        "SELECT 1 FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
        |_| Ok(()),
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))?;

    // Verify comment exists and belongs to post
    conn.query_row(
        "SELECT 1 FROM comments WHERE id = ?1 AND post_id = ?2",
        rusqlite::params![comment_id, post_id],
        |_| Ok(()),
    ).map_err(|_| err(Status::NotFound, "Comment not found", "NOT_FOUND"))?;

    conn.execute("DELETE FROM comments WHERE id = ?1", [comment_id])
        .map_err(|e| db_err(&e.to_string()))?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

// ─── Post Pinning ───

#[post("/blogs/<blog_id>/posts/<post_id>/pin")]
pub fn pin_post(blog_id: &str, post_id: &str, token: BlogToken, db: &State<DbPool>) -> Result<Json<PostResponse>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    // Verify post exists and belongs to blog
    conn.query_row(
        "SELECT 1 FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
        |_| Ok(()),
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))?;

    conn.execute(
        "UPDATE posts SET is_pinned = 1, updated_at = datetime('now') WHERE id = ?1",
        [post_id],
    ).map_err(|e| db_err(&e.to_string()))?;

    query_post(&conn, post_id).map(Json)
}

#[post("/blogs/<blog_id>/posts/<post_id>/unpin")]
pub fn unpin_post(blog_id: &str, post_id: &str, token: BlogToken, db: &State<DbPool>) -> Result<Json<PostResponse>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    conn.query_row(
        "SELECT 1 FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
        |_| Ok(()),
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))?;

    conn.execute(
        "UPDATE posts SET is_pinned = 0, updated_at = datetime('now') WHERE id = ?1",
        [post_id],
    ).map_err(|e| db_err(&e.to_string()))?;

    query_post(&conn, post_id).map(Json)
}

// ─── RSS Feed ───

#[get("/blogs/<blog_id>/feed.rss")]
pub fn rss_feed(blog_id: &str, db: &State<DbPool>) -> ContentResult {
    let conn = db.lock().unwrap();
    let blog = conn.query_row(
        "SELECT name, description FROM blogs WHERE id = ?1", [blog_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))?;

    let mut stmt = conn.prepare(
        "SELECT title, slug, summary, content_html, published_at, author_name FROM posts WHERE blog_id = ?1 AND status = 'published' ORDER BY published_at DESC LIMIT 50"
    ).map_err(|e| db_err(&e.to_string()))?;

    let items: Vec<String> = stmt.query_map([blog_id], |row| {
        let title: String = row.get(0)?;
        let slug: String = row.get(1)?;
        let summary: String = row.get(2)?;
        let content_html: String = row.get(3)?;
        let published_at: String = row.get::<_, Option<String>>(4)?.unwrap_or_default();
        let author: String = row.get(5)?;
        let desc = if summary.is_empty() { &content_html } else { &summary };
        Ok(format!(
            "<item><title><![CDATA[{}]]></title><link>/blog/{}/post/{}</link><description><![CDATA[{}]]></description><pubDate>{}</pubDate><author>{}</author></item>",
            title, blog_id, slug, desc, published_at, author
        ))
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
<title><![CDATA[{}]]></title>
<description><![CDATA[{}]]></description>
<link>/blog/{}</link>
{}
</channel>
</rss>"#,
        blog.0, blog.1, blog_id, items.join("\n")
    );

    Ok((Status::Ok, (rocket::http::ContentType::XML, rss)))
}

// ─── JSON Feed ───

#[get("/blogs/<blog_id>/feed.json")]
pub fn json_feed(blog_id: &str, db: &State<DbPool>) -> Result<Json<serde_json::Value>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    let blog = conn.query_row(
        "SELECT name, description FROM blogs WHERE id = ?1", [blog_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))?;

    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, content_html, published_at, author_name, tags FROM posts WHERE blog_id = ?1 AND status = 'published' ORDER BY published_at DESC LIMIT 50"
    ).map_err(|e| db_err(&e.to_string()))?;

    let items: Vec<serde_json::Value> = stmt.query_map([blog_id], |row| {
        let tags_str: String = row.get(7)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        Ok(serde_json::json!({
            "id": row.get::<_, String>(0)?,
            "title": row.get::<_, String>(1)?,
            "url": format!("/blog/{}/post/{}", blog_id, row.get::<_, String>(2)?),
            "summary": row.get::<_, String>(3)?,
            "content_html": row.get::<_, String>(4)?,
            "date_published": row.get::<_, Option<String>>(5)?,
            "authors": [{"name": row.get::<_, String>(6)?}],
            "tags": tags,
        }))
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(serde_json::json!({
        "version": "https://jsonfeed.org/version/1.1",
        "title": blog.0,
        "description": blog.1,
        "home_page_url": format!("/blog/{}", blog_id),
        "feed_url": format!("/api/v1/blogs/{}/feed.json", blog_id),
        "items": items,
    })))
}

// ─── Search (FTS5 full-text) ───

#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub blog_id: String,
    pub blog_name: String,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub author_name: String,
    pub published_at: Option<String>,
    pub snippet: Option<String>,
    pub rank: Option<f64>,
}

#[get("/search?<q>&<limit>&<offset>")]
pub fn search_posts(q: &str, limit: Option<i64>, offset: Option<i64>, db: &State<DbPool>) -> Result<Json<Vec<SearchResult>>, (Status, Json<ApiError>)> {
    if q.trim().is_empty() {
        return Err(err(Status::BadRequest, "Query parameter 'q' is required", "VALIDATION_ERROR"));
    }
    let conn = db.lock().unwrap();
    let lim = limit.unwrap_or(20).clamp(1, 100);
    let off = offset.unwrap_or(0).max(0);
    let query = q.trim().to_string();

    // Try FTS5 first — falls back to LIKE if FTS fails (e.g. syntax error in query)
    let fts_result: Result<Vec<SearchResult>, rusqlite::Error> = (|| {
        let mut stmt = conn.prepare(
            "SELECT f.post_id, f.blog_id, b.name, p.title, p.slug, p.summary, p.tags, p.author_name, p.published_at,
                    snippet(posts_fts, 3, '<mark>', '</mark>', '…', 40) as snip,
                    rank
             FROM posts_fts f
             JOIN posts p ON p.id = f.post_id
             JOIN blogs b ON b.id = f.blog_id
             WHERE posts_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2 OFFSET ?3"
        )?;

        let results: Vec<SearchResult> = stmt.query_map(rusqlite::params![query, lim, off], |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                blog_id: row.get(1)?,
                blog_name: row.get(2)?,
                title: row.get(3)?,
                slug: row.get(4)?,
                summary: row.get(5)?,
                tags: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                author_name: row.get(7)?,
                published_at: row.get(8)?,
                snippet: row.get(9)?,
                rank: row.get(10)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
        Ok(results)
    })();

    match fts_result {
        Ok(results) => Ok(Json(results)),
        Err(_) => {
            // Fallback to LIKE search for invalid FTS queries
            let pattern = format!("%{}%", query);
            let mut stmt = conn.prepare(
                "SELECT p.id, p.blog_id, b.name, p.title, p.slug, p.summary, p.tags, p.author_name, p.published_at
                 FROM posts p JOIN blogs b ON b.id = p.blog_id
                 WHERE p.status = 'published'
                   AND (p.title LIKE ?1 OR p.content LIKE ?1 OR p.tags LIKE ?1 OR p.author_name LIKE ?1)
                 ORDER BY p.published_at DESC NULLS LAST
                 LIMIT ?2 OFFSET ?3"
            ).map_err(|e| db_err(&e.to_string()))?;

            let results = stmt.query_map(rusqlite::params![pattern, lim, off], |row| {
                Ok(SearchResult {
                    id: row.get(0)?,
                    blog_id: row.get(1)?,
                    blog_name: row.get(2)?,
                    title: row.get(3)?,
                    slug: row.get(4)?,
                    summary: row.get(5)?,
                    tags: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or_default(),
                    author_name: row.get(7)?,
                    published_at: row.get(8)?,
                    snippet: None,
                    rank: None,
                })
            }).map_err(|e| db_err(&e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

            Ok(Json(results))
        }
    }
}

// ─── Semantic Search (TF-IDF + Cosine Similarity) ───

#[derive(Serialize)]
pub struct SemanticResult {
    pub id: String,
    pub blog_id: String,
    pub blog_name: String,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub author_name: String,
    pub published_at: Option<String>,
    pub similarity: f64,
}

#[get("/search/semantic?<q>&<limit>&<blog_id>")]
pub fn semantic_search(q: &str, limit: Option<usize>, blog_id: Option<&str>, db: &State<DbPool>, sem: &State<SemanticIndex>) -> Result<Json<Vec<SemanticResult>>, (Status, Json<ApiError>)> {
    if q.trim().is_empty() {
        return Err(err(Status::BadRequest, "Query parameter 'q' is required", "VALIDATION_ERROR"));
    }
    let lim = limit.unwrap_or(20).clamp(1, 100);

    let hits = match blog_id {
        Some(bid) => sem.search_blog(bid, q, lim),
        None => sem.search(q, lim),
    };

    if hits.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let conn = db.lock().unwrap();
    let mut results = Vec::new();
    for hit in hits {
        let row = conn.query_row(
            "SELECT p.title, p.slug, p.summary, p.tags, p.author_name, p.published_at, b.name
             FROM posts p JOIN blogs b ON b.id = p.blog_id
             WHERE p.id = ?1 AND p.status = 'published'",
            [&hit.post_id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
            )),
        );
        if let Ok((title, slug, summary, tags_str, author_name, published_at, blog_name)) = row {
            results.push(SemanticResult {
                id: hit.post_id,
                blog_id: hit.blog_id,
                blog_name,
                title,
                slug,
                summary,
                tags: serde_json::from_str(&tags_str).unwrap_or_default(),
                author_name,
                published_at,
                similarity: (hit.similarity * 1000.0).round() / 1000.0,
            });
        }
    }

    Ok(Json(results))
}

// ─── Related Posts ───

#[derive(Serialize)]
pub struct RelatedPost {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub summary: String,
    pub tags: Vec<String>,
    pub author_name: String,
    pub published_at: Option<String>,
    pub reading_time_minutes: u32,
    pub score: f64,
}

fn title_words(title: &str) -> std::collections::HashSet<String> {
    static STOP_WORDS: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
        "of", "with", "by", "is", "it", "as", "be", "this", "that", "from",
        "was", "are", "were", "been", "has", "have", "had", "not", "no", "do",
        "does", "did", "will", "would", "can", "could", "should", "may", "might",
        "i", "we", "you", "he", "she", "they", "my", "your", "how", "what",
        "why", "when", "where", "which", "who",
    ];
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !STOP_WORDS.contains(w))
        .map(|w| w.to_string())
        .collect()
}

#[get("/blogs/<blog_id>/posts/<post_id>/related?<limit>")]
pub fn related_posts(blog_id: &str, post_id: &str, limit: Option<usize>, db: &State<DbPool>) -> Result<Json<Vec<RelatedPost>>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();

    // Get the source post
    let (src_tags_str, src_title): (String, String) = conn.query_row(
        "SELECT tags, title FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))?;

    let src_tags: Vec<String> = serde_json::from_str(&src_tags_str).unwrap_or_default();
    let src_words = title_words(&src_title);
    let max_results = limit.unwrap_or(5).clamp(1, 20);

    // Get all other published posts in the same blog
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, summary, tags, author_name, published_at, content
         FROM posts
         WHERE blog_id = ?1 AND id != ?2 AND status = 'published'
         ORDER BY published_at DESC NULLS LAST"
    ).map_err(|e| db_err(&e.to_string()))?;

    let mut candidates: Vec<RelatedPost> = stmt.query_map(rusqlite::params![blog_id, post_id], |row| {
        let id: String = row.get(0)?;
        let title: String = row.get(1)?;
        let slug: String = row.get(2)?;
        let summary: String = row.get(3)?;
        let tags_str: String = row.get(4)?;
        let author_name: String = row.get(5)?;
        let published_at: Option<String> = row.get(6)?;
        let content: String = row.get(7)?;

        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        let wc = compute_word_count(&content);

        // Score: shared tags (3 pts each) + title word overlap (1 pt each)
        let shared_tags = tags.iter().filter(|t| src_tags.contains(t)).count() as f64;
        let candidate_words = title_words(&title);
        let shared_words = src_words.intersection(&candidate_words).count() as f64;
        let score = shared_tags * 3.0 + shared_words;

        Ok(RelatedPost {
            id,
            title,
            slug,
            summary,
            tags,
            author_name,
            published_at,
            reading_time_minutes: compute_reading_time(wc),
            score,
        })
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .filter(|p| p.score > 0.0)
    .collect();

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(max_results);

    Ok(Json(candidates))
}

// ─── Blog Stats ───

#[derive(Serialize)]
pub struct BlogStats {
    pub blog_id: String,
    pub blog_name: String,
    pub total_posts: i64,
    pub published_posts: i64,
    pub total_comments: i64,
    pub total_views: i64,
    pub views_24h: i64,
    pub views_7d: i64,
    pub views_30d: i64,
    pub top_posts: Vec<PostViewSummary>,
}

#[derive(Serialize)]
pub struct PostViewSummary {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub view_count: i64,
    pub comment_count: i64,
}

#[get("/blogs/<blog_id>/stats")]
pub fn blog_stats(blog_id: &str, db: &State<DbPool>) -> Result<Json<BlogStats>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();

    let blog_name: String = conn.query_row(
        "SELECT name FROM blogs WHERE id = ?1", [blog_id], |r| r.get(0),
    ).map_err(|_| err(Status::NotFound, "Blog not found", "NOT_FOUND"))?;

    let total_posts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM posts WHERE blog_id = ?1", [blog_id], |r| r.get(0),
    ).unwrap_or(0);

    let published_posts: i64 = conn.query_row(
        "SELECT COUNT(*) FROM posts WHERE blog_id = ?1 AND status = 'published'", [blog_id], |r| r.get(0),
    ).unwrap_or(0);

    let total_comments: i64 = conn.query_row(
        "SELECT COUNT(*) FROM comments c JOIN posts p ON p.id = c.post_id WHERE p.blog_id = ?1",
        [blog_id], |r| r.get(0),
    ).unwrap_or(0);

    let total_views: i64 = conn.query_row(
        "SELECT COUNT(*) FROM post_views v JOIN posts p ON p.id = v.post_id WHERE p.blog_id = ?1",
        [blog_id], |r| r.get(0),
    ).unwrap_or(0);

    let views_24h: i64 = conn.query_row(
        "SELECT COUNT(*) FROM post_views v JOIN posts p ON p.id = v.post_id WHERE p.blog_id = ?1 AND v.viewed_at >= datetime('now', '-1 day')",
        [blog_id], |r| r.get(0),
    ).unwrap_or(0);

    let views_7d: i64 = conn.query_row(
        "SELECT COUNT(*) FROM post_views v JOIN posts p ON p.id = v.post_id WHERE p.blog_id = ?1 AND v.viewed_at >= datetime('now', '-7 days')",
        [blog_id], |r| r.get(0),
    ).unwrap_or(0);

    let views_30d: i64 = conn.query_row(
        "SELECT COUNT(*) FROM post_views v JOIN posts p ON p.id = v.post_id WHERE p.blog_id = ?1 AND v.viewed_at >= datetime('now', '-30 days')",
        [blog_id], |r| r.get(0),
    ).unwrap_or(0);

    // Top 10 posts by views
    let mut stmt = conn.prepare(
        "SELECT p.id, p.title, p.slug,
                (SELECT COUNT(*) FROM post_views v WHERE v.post_id = p.id) as vc,
                (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as cc
         FROM posts p
         WHERE p.blog_id = ?1 AND p.status = 'published'
         ORDER BY vc DESC
         LIMIT 10"
    ).map_err(|e| db_err(&e.to_string()))?;

    let top_posts = stmt.query_map([blog_id], |row| {
        Ok(PostViewSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            slug: row.get(2)?,
            view_count: row.get(3)?,
            comment_count: row.get(4)?,
        })
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(BlogStats {
        blog_id: blog_id.to_string(),
        blog_name,
        total_posts,
        published_posts,
        total_comments,
        total_views,
        views_24h,
        views_7d,
        views_30d,
        top_posts,
    }))
}

// ─── Preview ───

#[derive(Deserialize)]
pub struct PreviewReq {
    pub content: String,
}

#[derive(Serialize)]
pub struct PreviewResponse {
    pub html: String,
}

#[post("/preview", format = "json", data = "<req>")]
pub fn preview_markdown(req: Json<PreviewReq>) -> Json<PreviewResponse> {
    let html = render_markdown(&req.content);
    Json(PreviewResponse { html })
}

// ─── Export / Cross-posting ───

#[derive(Serialize)]
pub struct ExportMarkdown {
    pub title: String,
    pub slug: String,
    pub author_name: String,
    pub published_at: Option<String>,
    pub tags: Vec<String>,
    pub summary: String,
    pub frontmatter: String,
    pub content: String,
    pub full_document: String,
}

#[get("/blogs/<blog_id>/posts/<slug>/export/markdown", rank = 3)]
pub fn export_markdown(blog_id: &str, slug: &str, db: &State<DbPool>) -> Result<Json<ExportMarkdown>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    let post = conn.query_row(
        "SELECT title, slug, content, summary, tags, author_name, published_at FROM posts WHERE blog_id = ?1 AND slug = ?2 AND status = 'published'",
        rusqlite::params![blog_id, slug],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
        )),
    ).map_err(|_| err(Status::NotFound, "Post not found or not published", "NOT_FOUND"))?;

    let tags: Vec<String> = serde_json::from_str(&post.4).unwrap_or_default();
    let tags_line = if tags.is_empty() { String::new() } else { format!("tags: [{}]\n", tags.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ")) };

    let frontmatter = format!(
        "---\ntitle: \"{}\"\nauthor: \"{}\"\ndate: \"{}\"\n{}summary: \"{}\"\n---",
        post.0.replace('"', "\\\""),
        post.5.replace('"', "\\\""),
        post.6.as_deref().unwrap_or(""),
        tags_line,
        post.3.replace('"', "\\\""),
    );

    let full_document = format!("{}\n\n{}", frontmatter, post.2);

    Ok(Json(ExportMarkdown {
        title: post.0,
        slug: post.1,
        author_name: post.5,
        published_at: post.6,
        tags,
        summary: post.3,
        frontmatter,
        content: post.2,
        full_document,
    }))
}

#[get("/blogs/<blog_id>/posts/<slug>/export/html", rank = 4)]
pub fn export_html(blog_id: &str, slug: &str, db: &State<DbPool>) -> ContentResult {
    let conn = db.lock().unwrap();
    let post = conn.query_row(
        "SELECT title, content_html, summary, author_name, published_at, tags FROM posts WHERE blog_id = ?1 AND slug = ?2 AND status = 'published'",
        rusqlite::params![blog_id, slug],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, String>(5)?,
        )),
    ).map_err(|_| err(Status::NotFound, "Post not found or not published", "NOT_FOUND"))?;

    let tags: Vec<String> = serde_json::from_str(&post.5).unwrap_or_default();
    let tags_html = if tags.is_empty() { String::new() } else {
        format!("<div class=\"tags\">{}</div>", tags.iter().map(|t| format!("<span class=\"tag\">{}</span>", t)).collect::<Vec<_>>().join(" "))
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<meta name="description" content="{summary}">
<meta name="author" content="{author}">
<style>
body {{ max-width: 720px; margin: 0 auto; padding: 2rem; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; line-height: 1.6; color: #e2e8f0; background: #0f172a; }}
h1 {{ font-size: 2rem; margin-bottom: 0.5rem; color: #f1f5f9; }}
.meta {{ color: #94a3b8; margin-bottom: 2rem; }}
.tags {{ margin-top: 0.5rem; }} .tag {{ background: #1e293b; padding: 2px 8px; border-radius: 4px; font-size: 0.85rem; color: #7dd3fc; margin-right: 4px; }}
article {{ line-height: 1.8; }} article img {{ max-width: 100%; border-radius: 8px; }}
pre {{ background: #1e293b; padding: 1rem; border-radius: 8px; overflow-x: auto; }}
code {{ font-family: 'Fira Code', monospace; font-size: 0.9em; }} :not(pre) > code {{ background: #1e293b; padding: 2px 6px; border-radius: 4px; }}
a {{ color: #38bdf8; }}
blockquote {{ border-left: 3px solid #334155; margin-left: 0; padding-left: 1rem; color: #94a3b8; }}
</style>
</head>
<body>
<header>
<h1>{title}</h1>
<div class="meta">
{author_html}{date_html}
{tags_html}
</div>
</header>
<article>{content}</article>
</body>
</html>"#,
        title = post.0.replace('"', "&quot;"),
        summary = post.2.replace('"', "&quot;"),
        author = post.3.replace('"', "&quot;"),
        author_html = if post.3.is_empty() { String::new() } else { format!("By {} ", post.3) },
        date_html = post.4.as_ref().map(|d| format!("· {}", d)).unwrap_or_default(),
        tags_html = tags_html,
        content = post.1,
    );

    Ok((Status::Ok, (rocket::http::ContentType::HTML, html)))
}

/// Export as unsigned Nostr NIP-23 long-form content event template (kind 30023)
#[derive(Serialize)]
pub struct NostrExport {
    pub kind: u32,
    pub content: String,
    pub tags: Vec<Vec<String>>,
    pub note: String,
}

#[get("/blogs/<blog_id>/posts/<slug>/export/nostr", rank = 5)]
pub fn export_nostr(blog_id: &str, slug: &str, db: &State<DbPool>) -> Result<Json<NostrExport>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    let post = conn.query_row(
        "SELECT title, slug, content, summary, tags, author_name, published_at FROM posts WHERE blog_id = ?1 AND slug = ?2 AND status = 'published'",
        rusqlite::params![blog_id, slug],
        |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
        )),
    ).map_err(|_| err(Status::NotFound, "Post not found or not published", "NOT_FOUND"))?;

    let post_tags: Vec<String> = serde_json::from_str(&post.4).unwrap_or_default();

    let mut nostr_tags: Vec<Vec<String>> = vec![
        vec!["d".to_string(), post.1.clone()],
        vec!["title".to_string(), post.0],
    ];

    if !post.3.is_empty() {
        nostr_tags.push(vec!["summary".to_string(), post.3]);
    }

    if let Some(ref pa) = post.6 {
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(pa) {
            nostr_tags.push(vec!["published_at".to_string(), dt.timestamp().to_string()]);
        }
    }

    for tag in &post_tags {
        nostr_tags.push(vec!["t".to_string(), tag.clone()]);
    }

    Ok(Json(NostrExport {
        kind: 30023,
        content: post.2,
        tags: nostr_tags,
        note: "Unsigned NIP-23 event template. Sign with your Nostr key and publish to relays.".to_string(),
    }))
}

// ─── Catchers ───

#[catch(401)]
pub fn unauthorized() -> Json<ApiError> {
    Json(ApiError { error: "Unauthorized".to_string(), code: "UNAUTHORIZED".to_string() })
}

#[catch(404)]
pub fn not_found() -> Json<ApiError> {
    Json(ApiError { error: "Not found".to_string(), code: "NOT_FOUND".to_string() })
}

#[catch(429)]
pub fn too_many_requests() -> Json<ApiError> {
    Json(ApiError { error: "Too many requests".to_string(), code: "RATE_LIMIT_EXCEEDED".to_string() })
}

#[catch(500)]
pub fn internal_error() -> Json<ApiError> {
    Json(ApiError { error: "Internal server error".to_string(), code: "INTERNAL_ERROR".to_string() })
}
