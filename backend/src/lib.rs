#[macro_use]
extern crate rocket;

pub mod db;
pub mod routes;
pub mod auth;
pub mod events;
pub mod rate_limit;

pub type DbPool = std::sync::Mutex<rusqlite::Connection>;

pub fn create_rocket(conn: rusqlite::Connection) -> rocket::Rocket<rocket::Build> {
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
        .manage(std::sync::Mutex::new(conn))
        .manage(routes::RateLimiters {
            blog_creation: blog_limiter,
            comment_creation: comment_limiter,
        })
        .manage(events::EventBus::new())
        .attach(cors)
        .mount("/api/v1", routes![
            routes::health,
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
            routes::rss_feed,
            routes::json_feed,
            routes::search_posts,
            routes::preview_markdown,
            routes::blog_event_stream,
        ])
        .mount("/", routes![routes::llms_txt])
        .register("/", catchers![routes::not_found, routes::internal_error, routes::unauthorized, routes::too_many_requests])
}
