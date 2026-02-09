#[macro_use]
extern crate rocket;

pub mod db;
pub mod routes;
pub mod auth;

pub type DbPool = std::sync::Mutex<rusqlite::Connection>;

pub fn create_rocket(conn: rusqlite::Connection) -> rocket::Rocket<rocket::Build> {
    rocket::build()
        .manage(std::sync::Mutex::new(conn))
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
        ])
        .mount("/", routes![routes::llms_txt])
        .register("/", catchers![routes::not_found, routes::internal_error, routes::unauthorized])
}
