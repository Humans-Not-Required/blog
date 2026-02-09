use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::broadcast;

const CHANNEL_CAPACITY: usize = 256;

/// A blog-level event broadcast system.
pub struct EventBus {
    channels: Mutex<HashMap<String, broadcast::Sender<BlogEvent>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlogEvent {
    pub event: String,
    pub blog_id: String,
    pub data: serde_json::Value,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }

    pub fn subscribe(&self, blog_id: &str) -> broadcast::Receiver<BlogEvent> {
        let mut channels = self.channels.lock().unwrap();
        let sender = channels
            .entry(blog_id.to_string())
            .or_insert_with(|| broadcast::channel(CHANNEL_CAPACITY).0);
        sender.subscribe()
    }

    pub fn emit(&self, event: BlogEvent) {
        let channels = self.channels.lock().unwrap();
        if let Some(sender) = channels.get(&event.blog_id) {
            let _: Result<usize, _> = sender.send(event);
        }
    }
}
