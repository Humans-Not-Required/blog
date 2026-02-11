#[macro_use]
extern crate rocket;

use std::sync::Mutex;

mod db;
mod routes;
mod auth;
mod rate_limit;

pub type DbPool = Mutex<rusqlite::Connection>;

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

    rocket::build()
        .manage(Mutex::new(conn))
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
            routes::preview_markdown,
            routes::related_posts,
            routes::blog_stats,
            routes::export_markdown,
            routes::export_html,
            routes::export_nostr,
        ])
        .mount("/", routes![routes::llms_txt])
        .mount("/", rocket::fs::FileServer::from(static_dir).rank(20))
        .register("/", catchers![routes::not_found, routes::internal_error, routes::unauthorized, routes::too_many_requests])
}
