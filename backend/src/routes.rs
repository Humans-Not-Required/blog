use rocket::serde::json::Json;
use rocket::http::Status;
use rocket::State;
use serde::{Deserialize, Serialize};

use crate::auth::{BlogToken, generate_key, hash_key};
use crate::DbPool;

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

#[get("/llms.txt")]
pub fn llms_txt() -> (Status, (rocket::http::ContentType, String)) {
    (Status::Ok, (rocket::http::ContentType::Plain, format!(
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
         Bearer token, X-API-Key header, or ?key= query param\n"
    )))
}

#[post("/blogs", format = "json", data = "<req>")]
pub fn create_blog(req: Json<CreateBlogReq>, db: &State<DbPool>) -> Result<(Status, Json<BlogCreated>), (Status, Json<ApiError>)> {
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
pub fn create_post(blog_id: &str, req: Json<CreatePostReq>, token: BlogToken, db: &State<DbPool>) -> Result<(Status, Json<PostResponse>), (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    let title = req.title.trim();
    if title.is_empty() {
        return Err(err(Status::UnprocessableEntity, "Title is required", "VALIDATION_ERROR"));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let slug = req.slug.as_deref().map(|s| slugify(s)).unwrap_or_else(|| slugify(title));
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

    let post = query_post(&conn, &id)?;
    Ok((Status::Created, Json(post)))
}

fn query_post(conn: &rusqlite::Connection, post_id: &str) -> Result<PostResponse, (Status, Json<ApiError>)> {
    conn.query_row(
        "SELECT p.id, p.blog_id, p.title, p.slug, p.content, p.content_html, p.summary, p.tags, p.status, p.published_at, p.author_name, p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as comment_count
         FROM posts p WHERE p.id = ?1",
        [post_id],
        |row| Ok(PostResponse {
            id: row.get(0)?,
            blog_id: row.get(1)?,
            title: row.get(2)?,
            slug: row.get(3)?,
            content: row.get(4)?,
            content_html: row.get(5)?,
            summary: row.get(6)?,
            tags: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
            status: row.get(8)?,
            published_at: row.get(9)?,
            author_name: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
            comment_count: row.get(13)?,
        }),
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
                (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as comment_count
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

    sql.push_str(" ORDER BY p.published_at DESC NULLS LAST, p.created_at DESC");

    let lim = limit.unwrap_or(50).min(100).max(1);
    let off = offset.unwrap_or(0).max(0);
    params.push(Box::new(lim));
    sql.push_str(&format!(" LIMIT ?{}", params.len()));
    params.push(Box::new(off));
    sql.push_str(&format!(" OFFSET ?{}", params.len()));

    let mut stmt = conn.prepare(&sql).map_err(|e| db_err(&e.to_string()))?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let posts = stmt.query_map(param_refs.as_slice(), |row| {
        Ok(PostResponse {
            id: row.get(0)?,
            blog_id: row.get(1)?,
            title: row.get(2)?,
            slug: row.get(3)?,
            content: row.get(4)?,
            content_html: row.get(5)?,
            summary: row.get(6)?,
            tags: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
            status: row.get(8)?,
            published_at: row.get(9)?,
            author_name: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
            comment_count: row.get(13)?,
        })
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(posts))
}

#[get("/blogs/<blog_id>/posts/<slug>", rank = 2)]
pub fn get_post_by_slug(blog_id: &str, slug: &str, db: &State<DbPool>) -> Result<Json<PostResponse>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    conn.query_row(
        "SELECT p.id, p.blog_id, p.title, p.slug, p.content, p.content_html, p.summary, p.tags, p.status, p.published_at, p.author_name, p.created_at, p.updated_at,
                (SELECT COUNT(*) FROM comments c WHERE c.post_id = p.id) as comment_count
         FROM posts p WHERE p.blog_id = ?1 AND p.slug = ?2",
        rusqlite::params![blog_id, slug],
        |row| Ok(PostResponse {
            id: row.get(0)?,
            blog_id: row.get(1)?,
            title: row.get(2)?,
            slug: row.get(3)?,
            content: row.get(4)?,
            content_html: row.get(5)?,
            summary: row.get(6)?,
            tags: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or_default(),
            status: row.get(8)?,
            published_at: row.get(9)?,
            author_name: row.get(10)?,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
            comment_count: row.get(13)?,
        }),
    ).map_err(|_| err(Status::NotFound, "Post not found", "NOT_FOUND"))
    .map(Json)
}

#[patch("/blogs/<blog_id>/posts/<post_id>", format = "json", data = "<req>")]
pub fn update_post(blog_id: &str, post_id: &str, req: Json<UpdatePostReq>, token: BlogToken, db: &State<DbPool>) -> Result<Json<PostResponse>, (Status, Json<ApiError>)> {
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
    let slug = req.slug.as_deref().map(|s| slugify(s)).unwrap_or(current.1);
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

    let post = query_post(&conn, post_id)?;
    Ok(Json(post))
}

#[delete("/blogs/<blog_id>/posts/<post_id>")]
pub fn delete_post(blog_id: &str, post_id: &str, token: BlogToken, db: &State<DbPool>) -> Result<Json<serde_json::Value>, (Status, Json<ApiError>)> {
    let conn = db.lock().unwrap();
    verify_blog_key(&conn, blog_id, &token)?;

    // Delete comments first
    conn.execute("DELETE FROM comments WHERE post_id = ?1", [post_id]).ok();
    let deleted = conn.execute(
        "DELETE FROM posts WHERE id = ?1 AND blog_id = ?2",
        rusqlite::params![post_id, blog_id],
    ).map_err(|e| db_err(&e.to_string()))?;

    if deleted == 0 {
        return Err(err(Status::NotFound, "Post not found", "NOT_FOUND"));
    }
    Ok(Json(serde_json::json!({"deleted": true})))
}

#[post("/blogs/<blog_id>/posts/<post_id>/comments", format = "json", data = "<req>")]
pub fn create_comment(blog_id: &str, post_id: &str, req: Json<CreateCommentReq>, db: &State<DbPool>) -> Result<(Status, Json<CommentResponse>), (Status, Json<ApiError>)> {
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

// ─── RSS Feed ───

#[get("/blogs/<blog_id>/feed.rss")]
pub fn rss_feed(blog_id: &str, db: &State<DbPool>) -> Result<(Status, (rocket::http::ContentType, String)), (Status, Json<ApiError>)> {
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

// ─── Search ───

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
}

#[get("/search?<q>&<limit>&<offset>")]
pub fn search_posts(q: &str, limit: Option<i64>, offset: Option<i64>, db: &State<DbPool>) -> Result<Json<Vec<SearchResult>>, (Status, Json<ApiError>)> {
    if q.trim().is_empty() {
        return Err(err(Status::BadRequest, "Query parameter 'q' is required", "VALIDATION_ERROR"));
    }
    let conn = db.lock().unwrap();
    let pattern = format!("%{}%", q.trim());
    let lim = limit.unwrap_or(20).min(100).max(1);
    let off = offset.unwrap_or(0).max(0);

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
        })
    }).map_err(|e| db_err(&e.to_string()))?
    .filter_map(|r| r.ok())
    .collect();

    Ok(Json(results))
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

#[catch(500)]
pub fn internal_error() -> Json<ApiError> {
    Json(ApiError { error: "Internal server error".to_string(), code: "INTERNAL_ERROR".to_string() })
}
