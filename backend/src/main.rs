#[macro_use]
extern crate rocket;

use std::sync::Mutex;

mod db;
mod routes;
mod auth;
mod rate_limit;
mod semantic;

pub type DbPool = Mutex<rusqlite::Connection>;

/// Extension trait for DbPool to recover from mutex poison
pub trait DbPoolExt {
    fn conn(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection>;
}

impl DbPoolExt for DbPool {
    fn conn(&self) -> std::sync::MutexGuard<'_, rusqlite::Connection> {
        self.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[launch]
fn rocket() -> _ {
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "data/blog.db".to_string());
    std::fs::create_dir_all(std::path::Path::new(&db_path).parent().unwrap_or(std::path::Path::new("."))).ok();
    let conn = rusqlite::Connection::open(&db_path).expect("Failed to open database");
    db::initialize(&conn);

    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "static".to_string());

    let cors = rocket_cors::CorsOptions::default()
        .allowed_origins(rocket_cors::AllowedOrigins::all())
        .to_cors()
        .expect("CORS config");

    let blog_limit: u64 = std::env::var("BLOG_RATE_LIMIT")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(10);
    let comment_limit: u64 = std::env::var("COMMENT_RATE_LIMIT")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    let blog_limiter = rate_limit::RateLimiter::new(std::time::Duration::from_secs(3600), blog_limit);
    let comment_limiter = rate_limit::RateLimiter::new(std::time::Duration::from_secs(3600), comment_limit);

    // Build semantic search index from published posts
    let sem_index = semantic::SemanticIndex::new();
    db::rebuild_semantic_index(&conn, &sem_index);

    rocket::build()
        .manage(Mutex::new(conn))
        .manage(sem_index)
        .manage(routes::RateLimiters {
            blog_creation: blog_limiter,
            comment_creation: comment_limiter,
        })
        .attach(cors)
        .mount("/api/v1", routes![
            routes::health,
            routes::openapi,
            routes::create_blog,
            routes::list_blogs,
            routes::get_blog,
            routes::update_blog,
            routes::create_post,
            routes::list_posts,
            routes::get_post_by_slug,
            routes::update_post,
            routes::delete_post,
            routes::create_comment,
            routes::list_comments,
            routes::delete_comment,
            routes::pin_post,
            routes::unpin_post,
            routes::rss_feed,
            routes::json_feed,
            routes::search_posts,
            routes::semantic_search,
            routes::preview_markdown,
            routes::related_posts,
            routes::blog_stats,
            routes::export_markdown,
            routes::export_html,
            routes::export_nostr,
            routes::api_skills_skill_md,
        ])
        .mount("/", routes![routes::llms_txt, routes::skills_index, routes::skills_skill_md])
        .mount("/", rocket::fs::FileServer::from(static_dir).rank(20))
        .register("/", catchers![routes::not_found, routes::internal_error, routes::unauthorized, routes::too_many_requests])
}
