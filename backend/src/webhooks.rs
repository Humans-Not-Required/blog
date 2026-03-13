use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use hmac::{Hmac, Mac};

use crate::DbPool;

type HmacSha256 = Hmac<Sha256>;

/// Valid webhook event types
pub const VALID_EVENTS: &[&str] = &[
    "post.published",
    "post.updated",
    "post.deleted",
    "comment.created",
    "post.reacted",
];

// ─── Models ───

#[derive(Debug, Serialize, Clone)]
pub struct WebhookResponse {
    pub id: String,
    pub blog_id: String,
    pub url: String,
    pub events: Vec<String>,
    pub is_active: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct WebhookDeliveryResponse {
    pub id: i64,
    pub webhook_id: String,
    pub event: String,
    pub status_code: Option<i32>,
    pub success: bool,
    pub error: Option<String>,
    pub delivered_at: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct WebhookPayload {
    pub event: String,
    pub blog_id: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CreateWebhookReq {
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
}

// ─── Validation ───

pub fn validate_events(events: &[String]) -> Result<(), String> {
    if events.is_empty() {
        return Err("At least one event is required".to_string());
    }
    for e in events {
        if !VALID_EVENTS.contains(&e.as_str()) {
            return Err(format!("Invalid event: '{}'. Valid events: {}", e, VALID_EVENTS.join(", ")));
        }
    }
    Ok(())
}

pub fn validate_url(url: &str) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    if url.len() > 2048 {
        return Err("URL must be under 2048 characters".to_string());
    }
    Ok(())
}

// ─── HMAC Signing ───

pub fn sign_payload(payload: &[u8], secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(payload);
    let result = mac.finalize();
    hex::encode(result.into_bytes())
}

// ─── DB Operations ───

pub fn create_webhook(conn: &Connection, id: &str, blog_id: &str, url: &str, events: &[String], secret: Option<&str>) -> Result<WebhookResponse, String> {
    let events_json = serde_json::to_string(events).unwrap_or_else(|_| "[]".to_string());
    conn.execute(
        "INSERT INTO webhooks (id, blog_id, url, events, secret) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![id, blog_id, url, events_json, secret],
    ).map_err(|e| format!("Failed to create webhook: {}", e))?;

    Ok(WebhookResponse {
        id: id.to_string(),
        blog_id: blog_id.to_string(),
        url: url.to_string(),
        events: events.to_vec(),
        is_active: true,
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub fn list_webhooks(conn: &Connection, blog_id: &str) -> Vec<WebhookResponse> {
    let mut stmt = conn.prepare(
        "SELECT id, blog_id, url, events, is_active, created_at FROM webhooks WHERE blog_id = ?1 ORDER BY created_at DESC"
    ).unwrap();
    stmt.query_map([blog_id], |row| {
        let events_str: String = row.get(3)?;
        let events: Vec<String> = serde_json::from_str(&events_str).unwrap_or_default();
        Ok(WebhookResponse {
            id: row.get(0)?,
            blog_id: row.get(1)?,
            url: row.get(2)?,
            events,
            is_active: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
        })
    }).unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

pub fn get_webhook(conn: &Connection, webhook_id: &str, blog_id: &str) -> Option<WebhookResponse> {
    conn.query_row(
        "SELECT id, blog_id, url, events, is_active, created_at FROM webhooks WHERE id = ?1 AND blog_id = ?2",
        [webhook_id, blog_id],
        |row| {
            let events_str: String = row.get(3)?;
            let events: Vec<String> = serde_json::from_str(&events_str).unwrap_or_default();
            Ok(WebhookResponse {
                id: row.get(0)?,
                blog_id: row.get(1)?,
                url: row.get(2)?,
                events,
                is_active: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
            })
        },
    ).ok()
}

pub fn delete_webhook(conn: &Connection, webhook_id: &str, blog_id: &str) -> bool {
    let deleted = conn.execute(
        "DELETE FROM webhooks WHERE id = ?1 AND blog_id = ?2",
        [webhook_id, blog_id],
    ).unwrap_or(0);
    deleted > 0
}

pub fn count_webhooks(conn: &Connection, blog_id: &str) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM webhooks WHERE blog_id = ?1",
        [blog_id],
        |row| row.get(0),
    ).unwrap_or(0)
}

/// Get all active webhooks for a blog that subscribe to a specific event
pub fn get_matching_webhooks(conn: &Connection, blog_id: &str, event: &str) -> Vec<(String, String, Option<String>)> {
    let mut stmt = conn.prepare(
        "SELECT id, url, events, secret FROM webhooks WHERE blog_id = ?1 AND is_active = 1"
    ).unwrap();
    stmt.query_map([blog_id], |row| {
        let id: String = row.get(0)?;
        let url: String = row.get(1)?;
        let events_str: String = row.get(2)?;
        let secret: Option<String> = row.get(3)?;
        Ok((id, url, events_str, secret))
    }).unwrap()
    .filter_map(|r| r.ok())
    .filter(|(_, _, events_str, _)| {
        let events: Vec<String> = serde_json::from_str(events_str).unwrap_or_default();
        events.contains(&event.to_string())
    })
    .map(|(id, url, _, secret)| (id, url, secret))
    .collect()
}

pub fn list_deliveries(conn: &Connection, webhook_id: &str, blog_id: &str, limit: i64) -> Vec<WebhookDeliveryResponse> {
    let mut stmt = conn.prepare(
        "SELECT d.id, d.webhook_id, d.event, d.status_code, d.success, d.error, d.delivered_at
         FROM webhook_deliveries d
         JOIN webhooks w ON d.webhook_id = w.id
         WHERE d.webhook_id = ?1 AND w.blog_id = ?2
         ORDER BY d.delivered_at DESC
         LIMIT ?3"
    ).unwrap();
    stmt.query_map(rusqlite::params![webhook_id, blog_id, limit], |row| {
        Ok(WebhookDeliveryResponse {
            id: row.get(0)?,
            webhook_id: row.get(1)?,
            event: row.get(2)?,
            status_code: row.get(3)?,
            success: row.get::<_, i32>(4)? != 0,
            error: row.get(5)?,
            delivered_at: row.get(6)?,
        })
    }).unwrap()
    .filter_map(|r| r.ok())
    .collect()
}

// ─── Async Delivery ───

/// Fire webhooks for an event. Non-blocking — spawns async tasks.
pub fn fire_webhooks(db: &DbPool, blog_id: &str, event: &str, data: serde_json::Value) {
    let conn = db.lock().unwrap_or_else(|e| e.into_inner());
    let matching = get_matching_webhooks(&conn, blog_id, event);
    drop(conn);

    if matching.is_empty() {
        return;
    }

    let payload = WebhookPayload {
        event: event.to_string(),
        blog_id: blog_id.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        data,
    };

    let payload_json = match serde_json::to_vec(&payload) {
        Ok(j) => j,
        Err(_) => return,
    };

    for (_, url, secret) in matching {
        let payload_bytes = payload_json.clone();
        let evt = event.to_string();
        let signature = secret.as_deref().map(|s| sign_payload(&payload_bytes, s));
        let client = reqwest::Client::new();

        tokio::spawn(async move {
            let mut req = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("X-Webhook-Event", &evt)
                .timeout(std::time::Duration::from_secs(10))
                .body(payload_bytes);

            if let Some(sig) = signature {
                req = req.header("X-Webhook-Signature", format!("sha256={}", sig));
            }

            // Fire and forget — v1 has no delivery recording
            let _result = req.send().await;
        });
    }
}
