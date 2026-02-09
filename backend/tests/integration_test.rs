use rocket::http::{ContentType, Header, Status};
use rocket::local::blocking::Client;
use blog::{create_rocket, db};

fn test_client() -> Client {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    db::initialize(&conn);
    let rocket = create_rocket(conn);
    Client::tracked(rocket).unwrap()
}

fn create_blog_helper(client: &Client, name: &str) -> (String, String) {
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(format!(r#"{{"name": "{}"}}"#, name))
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let body: serde_json::Value = resp.into_json().unwrap();
    (body["id"].as_str().unwrap().to_string(), body["manage_key"].as_str().unwrap().to_string())
}

#[test]
fn test_health() {
    let client = test_client();
    let resp = client.get("/api/v1/health").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["status"], "ok");
}

#[test]
fn test_create_blog() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Test Blog");
    assert!(!id.is_empty());
    assert!(key.starts_with("blog_"));
}

#[test]
fn test_create_blog_empty_name() {
    let client = test_client();
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(r#"{"name": ""}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::UnprocessableEntity);
}

#[test]
fn test_get_blog() {
    let client = test_client();
    let (id, _) = create_blog_helper(&client, "My Blog");
    let resp = client.get(format!("/api/v1/blogs/{}", id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["name"], "My Blog");
}

#[test]
fn test_update_blog_requires_auth() {
    let client = test_client();
    let (id, _) = create_blog_helper(&client, "Blog");
    let resp = client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON)
        .body(r#"{"name": "Updated"}"#)
        .dispatch();
    // No auth → forward → 404
    assert!(resp.status() == Status::NotFound || resp.status() == Status::Unauthorized);
}

#[test]
fn test_update_blog_with_auth() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let resp = client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"name": "Updated Blog"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["name"], "Updated Blog");
}

#[test]
fn test_list_blogs_public_only() {
    let client = test_client();
    create_blog_helper(&client, "Private Blog");
    // Create a public blog
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(r#"{"name": "Public Blog", "is_public": true}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    let resp = client.get("/api/v1/blogs").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let blogs = body.as_array().unwrap();
    assert_eq!(blogs.len(), 1);
    assert_eq!(blogs[0]["name"], "Public Blog");
}

#[test]
fn test_create_post() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Hello World", "content": "Hello **bold**.", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["title"], "Hello World");
    assert_eq!(body["slug"], "hello-world");
    assert_eq!(body["status"], "published");
    assert!(body["content_html"].as_str().unwrap().contains("bold"));
    assert!(body["published_at"].as_str().is_some());
}

#[test]
fn test_create_post_no_auth() {
    let client = test_client();
    let (id, _) = create_blog_helper(&client, "Blog");
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .body(r#"{"title": "Nope"}"#)
        .dispatch();
    assert!(resp.status() == Status::NotFound || resp.status() == Status::Unauthorized);
}

#[test]
fn test_list_posts_hides_drafts() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create draft
    client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Draft Post"}"#)
        .dispatch();

    // Create published
    client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Published Post", "status": "published"}"#)
        .dispatch();

    // Without auth: only published
    let resp = client.get(format!("/api/v1/blogs/{}/posts", id)).dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let posts = body.as_array().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["title"], "Published Post");

    // With auth: both
    let resp = client.get(format!("/api/v1/blogs/{}/posts", id))
        .header(auth)
        .dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body.as_array().unwrap().len(), 2);
}

#[test]
fn test_get_post_by_slug() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "My Great Post", "content": "Content here", "status": "published"}"#)
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/posts/my-great-post", id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["slug"], "my-great-post");
}

#[test]
fn test_update_post() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Original"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", id, post_id))
        .header(ContentType::JSON)
        .header(auth)
        .body(r#"{"title": "Updated", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["title"], "Updated");
    assert_eq!(body["status"], "published");
}

#[test]
fn test_delete_post() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Delete Me", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    let resp = client.delete(format!("/api/v1/blogs/{}/posts/{}", id, post_id))
        .header(auth)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
}

#[test]
fn test_comments() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(auth)
        .body(r#"{"title": "Post", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Add comment
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "Alice", "content": "Great post!"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    // List comments
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/comments", id, post_id)).dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[test]
fn test_comment_on_draft_fails() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Draft"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "Alice", "content": "Can't comment on draft"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_rss_feed() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "RSS Blog");
    client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Feed Post", "content": "Hello", "status": "published"}"#)
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/feed.rss", id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();
    assert!(body.contains("<rss"));
    assert!(body.contains("Feed Post"));
}

#[test]
fn test_json_feed() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "JSON Blog");
    client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Feed Post", "content": "Hello", "status": "published"}"#)
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/feed.json", id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["title"], "JSON Blog");
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[test]
fn test_slug_conflict() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Same Title"}"#)
        .dispatch();

    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(auth)
        .body(r#"{"title": "Same Title"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Conflict);
}

#[test]
fn test_auth_methods() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog");

    // Bearer
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Bearer Post"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    // X-API-Key
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("X-API-Key", key.clone()))
        .body(r#"{"title": "XApiKey Post"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    // ?key= query param
    let resp = client.post(format!("/api/v1/blogs/{}/posts?key={}", id, key))
        .header(ContentType::JSON)
        .body(r#"{"title": "Query Post"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
}

#[test]
fn test_wrong_key_rejected() {
    let client = test_client();
    let (id, _) = create_blog_helper(&client, "Blog");
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", "Bearer blog_wrong"))
        .body(r#"{"title": "Nope"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[test]
fn test_llms_txt() {
    let client = test_client();
    let resp = client.get("/llms.txt").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();
    assert!(body.contains("Blog Platform API"));
}

#[test]
fn test_search_posts() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Search Blog");

    // Create and publish a post
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Rust Programming Guide", "content": "Learn Rust with examples", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    // Create a draft post (should NOT appear in search)
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Draft About Rust", "content": "secret draft"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    // Search should find published post
    let resp = client.get("/api/v1/search?q=Rust").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let results = body.as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "Rust Programming Guide");
    assert_eq!(results[0]["blog_name"], "Search Blog");

    // Search with no match
    let resp = client.get("/api/v1/search?q=Python").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);

    // Empty query should fail
    let resp = client.get("/api/v1/search?q=").dispatch();
    assert_eq!(resp.status(), Status::BadRequest);
}

#[test]
fn test_blog_creation_rate_limit() {
    // BLOG_RATE_LIMIT defaults to 10
    std::env::set_var("BLOG_RATE_LIMIT", "3");
    let client = test_client();

    // First 3 should succeed
    for i in 0..3 {
        let resp = client.post("/api/v1/blogs")
            .header(ContentType::JSON)
            .body(format!(r#"{{"name": "Blog {}"}}"#, i))
            .dispatch();
        assert_eq!(resp.status(), Status::Created);
    }

    // 4th should be rate limited
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(r#"{"name": "Blog 4"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::TooManyRequests);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["code"], "RATE_LIMIT_EXCEEDED");

    std::env::remove_var("BLOG_RATE_LIMIT");
}

#[test]
fn test_comment_creation_rate_limit() {
    std::env::set_var("COMMENT_RATE_LIMIT", "2");
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Comment RL Blog");

    // Create and publish a post
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Test Post", "content": "Content", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();

    // First 2 comments should succeed
    for i in 0..2 {
        let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
            .header(ContentType::JSON)
            .body(format!(r#"{{"author_name": "User", "content": "Comment {}"}}"#, i))
            .dispatch();
        assert_eq!(resp.status(), Status::Created);
    }

    // 3rd comment should be rate limited
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "User", "content": "Spam"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::TooManyRequests);

    std::env::remove_var("COMMENT_RATE_LIMIT");
}
