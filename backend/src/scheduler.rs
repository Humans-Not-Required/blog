use rocket::{Rocket, fairing::{Fairing, Info, Kind}};
use std::sync::Mutex;
use crate::db;
use crate::semantic::SemanticIndex;
use crate::webhooks;

/// Fairing that spawns a background task to publish scheduled posts.
pub struct PostScheduler;

#[rocket::async_trait]
impl Fairing for PostScheduler {
    fn info(&self) -> Info {
        Info {
            name: "Post Scheduler",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        let db_pool = rocket.state::<Mutex<rusqlite::Connection>>()
            .expect("DB pool not found")
            as *const Mutex<rusqlite::Connection>;
        let sem_index = rocket.state::<SemanticIndex>()
            .expect("Semantic index not found")
            as *const SemanticIndex;

        // SAFETY: The Rocket instance (and its managed state) outlives the spawned task
        // because Rocket only shuts down when the process exits.
        let db_ptr = db_pool as usize;
        let sem_ptr = sem_index as usize;

        tokio::spawn(async move {
            let db = unsafe { &*(db_ptr as *const Mutex<rusqlite::Connection>) };
            let sem = unsafe { &*(sem_ptr as *const SemanticIndex) };

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                run_scheduler(db, sem);
            }
        });
    }
}

/// Run a single scheduler tick: publish due posts and fire webhooks.
pub fn run_scheduler(db: &Mutex<rusqlite::Connection>, sem: &SemanticIndex) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let published = db::publish_scheduled_posts(&conn);

    for (post_id, _blog_id) in &published {
        // Update semantic index
        db::upsert_semantic(&conn, post_id, sem);
    }
    drop(conn);

    // Fire webhooks outside the lock
    for (post_id, blog_id) in &published {
        // Get post details for webhook payload
        let conn = db.lock().unwrap_or_else(|e| e.into_inner());
        let payload = conn.query_row(
            "SELECT title, slug, author_name, summary FROM posts WHERE id = ?1",
            [post_id.as_str()],
            |row| Ok(serde_json::json!({
                "post_id": post_id,
                "title": row.get::<_, String>(0)?,
                "slug": row.get::<_, String>(1)?,
                "author_name": row.get::<_, String>(2)?,
                "summary": row.get::<_, String>(3)?,
                "scheduled": true
            })),
        ).ok();
        drop(conn);

        if let Some(payload) = payload {
            webhooks::fire_webhooks(db, blog_id, "post.published", payload);
        }
    }
}
