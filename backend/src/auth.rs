use rocket::request::{FromRequest, Outcome, Request};
use sha2::{Digest, Sha256};

/// Extracts a blog manage key from Bearer token, X-API-Key header, or ?key= query param.
pub struct BlogToken(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BlogToken {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // 1. Authorization: Bearer <key>
        if let Some(auth) = req.headers().get_one("Authorization") {
            if let Some(key) = auth.strip_prefix("Bearer ") {
                let key = key.trim();
                if !key.is_empty() {
                    return Outcome::Success(BlogToken(key.to_string()));
                }
            }
        }
        // 2. X-API-Key header
        if let Some(key) = req.headers().get_one("X-API-Key") {
            let key = key.trim();
            if !key.is_empty() {
                return Outcome::Success(BlogToken(key.to_string()));
            }
        }
        // 3. ?key= query param
        if let Some(key) = req.query_value::<String>("key") {
            if let Ok(key) = key {
                let key = key.trim().to_string();
                if !key.is_empty() {
                    return Outcome::Success(BlogToken(key));
                }
            }
        }
        Outcome::Forward(rocket::http::Status::Unauthorized)
    }
}

pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_key(prefix: &str) -> String {
    format!("{}_{}", prefix, uuid::Uuid::new_v4().simple())
}
