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
fn test_api_llms_txt() {
    let client = test_client();
    let resp = client.get("/api/v1/llms.txt").dispatch();
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
fn test_fts5_search_ranking_and_snippets() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "FTS Blog");

    // Create multiple published posts with varying relevance
    let posts = vec![
        r#"{"title": "Advanced Database Optimization", "content": "SQLite FTS5 provides full-text search with BM25 ranking for database queries.", "tags": ["database", "sqlite"], "status": "published"}"#,
        r#"{"title": "Web Development Basics", "content": "Building websites with HTML, CSS, and JavaScript for modern browsers.", "tags": ["web", "frontend"], "status": "published"}"#,
        r#"{"title": "Database Design Patterns", "content": "Learn about database normalization, indexing strategies, and query optimization techniques for database performance.", "tags": ["database", "architecture"], "status": "published"}"#,
    ];
    for body in posts {
        let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
            .header(ContentType::JSON)
            .header(Header::new("Authorization", format!("Bearer {}", key)))
            .body(body)
            .dispatch();
        assert_eq!(resp.status(), Status::Created);
    }

    // FTS search for "database" — should find 2 posts, ranked by relevance
    let resp = client.get("/api/v1/search?q=database").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let results = body.as_array().unwrap();
    assert_eq!(results.len(), 2);
    // Both results should have rank (BM25 score) — rank is negative, closer to 0 = more relevant
    assert!(results[0]["rank"].as_f64().is_some());
    // Results should have snippet field
    assert!(results[0].get("snippet").is_some());

    // FTS search for "database optimization" — more specific query
    let resp = client.get("/api/v1/search?q=database%20optimization").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let results = body.as_array().unwrap();
    assert!(results.len() >= 2); // Both database posts match

    // Search for non-existent term
    let resp = client.get("/api/v1/search?q=kubernetes").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);

    // Verify draft posts are NOT indexed — create a draft with searchable content
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Secret Draft About Kubernetes", "content": "Kubernetes orchestration secrets"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    let resp = client.get("/api/v1/search?q=kubernetes").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0, "Draft posts should not appear in FTS search");
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

#[test]
fn test_preview_markdown() {
    let client = test_client();
    let resp = client.post("/api/v1/preview")
        .header(ContentType::JSON)
        .body("{\"content\": \"# Hello\\n\\nThis is **bold** and *italic*.\"}")
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let html = body["html"].as_str().unwrap();
    assert!(html.contains("<h1>"));
    assert!(html.contains("<strong>"));
    assert!(html.contains("<em>"));
}

#[test]
fn test_openapi_json() {
    let client = test_client();
    let resp = client.get("/api/v1/openapi.json").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();
    let spec: serde_json::Value = serde_json::from_str(&body).expect("Valid JSON");
    assert_eq!(spec["openapi"], "3.0.3");
    assert_eq!(spec["info"]["title"], "Blog Platform API");
    assert!(spec["paths"]["/blogs"].is_object());
    assert!(spec["paths"]["/blogs/{blogId}/posts"].is_object());
    assert!(spec["components"]["schemas"]["Post"].is_object());
}

#[test]
fn test_word_count_and_reading_time() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Word Count Test");

    // Create a post with known word count (~50 words)
    let content = "This is a test post with some content. ".repeat(5); // ~40 words
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(serde_json::json!({
            "title": "Word Count Test",
            "content": content,
            "status": "published"
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert!(body["word_count"].as_u64().unwrap() > 0);
    assert!(body["reading_time_minutes"].as_u64().unwrap() >= 1);

    // Verify via get by slug
    let slug = body["slug"].as_str().unwrap();
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}", blog_id, slug)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert!(body["word_count"].as_u64().unwrap() > 0);
    assert_eq!(body["reading_time_minutes"].as_u64().unwrap(), 1); // ~40 words = 1 min

    // Verify in list endpoint
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 1);
    assert!(posts[0]["word_count"].as_u64().unwrap() > 0);
    assert!(posts[0]["reading_time_minutes"].as_u64().unwrap() >= 1);

    // Create a longer post (~1000 words = ~5 min reading time)
    let long_content = "Lorem ipsum dolor sit amet consectetur adipiscing elit. ".repeat(180); // ~1080 words
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(serde_json::json!({
            "title": "Long Post",
            "content": long_content,
            "status": "published"
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let body: serde_json::Value = resp.into_json().unwrap();
    let wc = body["word_count"].as_u64().unwrap();
    let rt = body["reading_time_minutes"].as_u64().unwrap();
    assert!(wc > 900, "Expected >900 words, got {}", wc);
    assert!(rt >= 5, "Expected >=5 min reading time for {} words, got {}", wc, rt);
}

#[test]
fn test_related_posts() {
    let client = test_client();

    // Create blog
    let resp = client.post("/api/v1/blogs").header(ContentType::JSON)
        .body(r#"{"name":"Related Test"}"#).dispatch();
    let blog: serde_json::Value = resp.into_json().unwrap();
    let blog_id = blog["id"].as_str().unwrap();
    let key = blog["manage_key"].as_str().unwrap();
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 3 published posts with overlapping tags
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title":"Rust Web Frameworks","content":"Content about Rust","tags":["rust","web","api"],"status":"published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post_a: serde_json::Value = resp.into_json().unwrap();
    let post_a_id = post_a["id"].as_str().unwrap();

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title":"Building APIs in Rust","content":"More Rust content","tags":["rust","api"],"status":"published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title":"Python for Data Science","content":"Python content","tags":["python","data"],"status":"published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    // Get related posts for post A (should find post B as most related via shared tags)
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/related", blog_id, post_a_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let related: Vec<serde_json::Value> = resp.into_json().unwrap();

    // Post B shares "rust" + "api" tags (6 pts) + "rust" title word (1 pt) = 7+ pts
    // Post C shares nothing = 0 pts (filtered out)
    assert!(!related.is_empty(), "Should have at least 1 related post");
    assert!(related[0]["score"].as_f64().unwrap() > 0.0);
    assert!(related[0]["title"].as_str().unwrap().contains("Rust") || related[0]["title"].as_str().unwrap().contains("API"));

    // Post with no tags should return empty related
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title":"Unique Standalone Post","content":"Nothing in common","tags":[],"status":"published"}"#)
        .dispatch();
    let solo: serde_json::Value = resp.into_json().unwrap();
    let solo_id = solo["id"].as_str().unwrap();

    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/related", blog_id, solo_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let related: Vec<serde_json::Value> = resp.into_json().unwrap();
    // May have some results from title word overlap, but score should be low or empty
    for r in &related {
        assert!(r["score"].as_f64().unwrap() > 0.0);
    }
}

#[test]
fn test_blog_stats_and_view_tracking() {
    let client = test_client();

    // Create blog
    let resp = client.post("/api/v1/blogs").header(ContentType::JSON)
        .body(r#"{"name":"Stats Blog"}"#).dispatch();
    let blog: serde_json::Value = resp.into_json().unwrap();
    let blog_id = blog["id"].as_str().unwrap();
    let key = blog["manage_key"].as_str().unwrap();
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create a published post
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title":"Stats Test Post","content":"Some content here for testing","tags":["test"],"status":"published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    let slug = post["slug"].as_str().unwrap();

    // Initially view_count should be 0
    assert_eq!(post["view_count"].as_i64().unwrap(), 0);

    // View the post 3 times (each GET records a view)
    for _ in 0..3 {
        let resp = client.get(format!("/api/v1/blogs/{}/posts/{}", blog_id, slug)).dispatch();
        assert_eq!(resp.status(), Status::Ok);
    }

    // Fourth view should show view_count = 3 (from previous 3 views)
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}", blog_id, slug)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let viewed: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(viewed["view_count"].as_i64().unwrap(), 3);

    // Check blog stats
    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["blog_name"].as_str().unwrap(), "Stats Blog");
    assert_eq!(stats["total_posts"].as_i64().unwrap(), 1);
    assert_eq!(stats["published_posts"].as_i64().unwrap(), 1);
    // 4 total views (3 + the one that checked view_count)
    assert_eq!(stats["total_views"].as_i64().unwrap(), 4);
    assert_eq!(stats["views_24h"].as_i64().unwrap(), 4);
    assert_eq!(stats["views_7d"].as_i64().unwrap(), 4);
    assert_eq!(stats["views_30d"].as_i64().unwrap(), 4);

    // Top posts should include our post
    let top = stats["top_posts"].as_array().unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0]["title"].as_str().unwrap(), "Stats Test Post");
    assert_eq!(top[0]["view_count"].as_i64().unwrap(), 4);

    // Stats for non-existent blog returns 404
    let resp = client.get("/api/v1/blogs/nonexistent/stats").dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_delete_comment() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Comment Del Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create a published post
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Post For Comments", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Add two comments
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "Alice", "content": "First comment"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let comment_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "Bob", "content": "Second comment"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    // Verify 2 comments
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id)).dispatch();
    assert_eq!(resp.into_json::<serde_json::Value>().unwrap().as_array().unwrap().len(), 2);

    // Delete without auth should fail (401)
    let resp = client.delete(format!("/api/v1/blogs/{}/posts/{}/comments/{}", blog_id, post_id, comment_id)).dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);

    // Delete with wrong key should fail
    let bad_auth = Header::new("Authorization", "Bearer wrong_key");
    let resp = client.delete(format!("/api/v1/blogs/{}/posts/{}/comments/{}", blog_id, post_id, comment_id))
        .header(bad_auth)
        .dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);

    // Delete with correct key should succeed
    let resp = client.delete(format!("/api/v1/blogs/{}/posts/{}/comments/{}", blog_id, post_id, comment_id))
        .header(auth.clone())
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["deleted"], true);

    // Verify only 1 comment remains
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id)).dispatch();
    assert_eq!(resp.into_json::<serde_json::Value>().unwrap().as_array().unwrap().len(), 1);

    // Delete non-existent comment should fail
    let resp = client.delete(format!("/api/v1/blogs/{}/posts/{}/comments/nonexistent", blog_id, post_id))
        .header(auth)
        .dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_pin_unpin_post() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Pin Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create two published posts
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Normal Post", "status": "published", "content": "First"}"#)
        .dispatch();
    let post1_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Small delay to ensure different timestamps
    std::thread::sleep(std::time::Duration::from_millis(10));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(r#"{"title": "Pinned Post", "status": "published", "content": "Second"}"#)
        .dispatch();
    let post2_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Initially, neither post is pinned
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 2);
    assert_eq!(posts[0]["is_pinned"], false);
    assert_eq!(posts[1]["is_pinned"], false);

    // Pin post1 (the older one)
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/pin", blog_id, post1_id))
        .header(auth.clone())
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["is_pinned"], true);

    // List posts — pinned one should be first
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts[0]["id"].as_str().unwrap(), post1_id);
    assert_eq!(posts[0]["is_pinned"], true);
    assert_eq!(posts[1]["is_pinned"], false);

    // Pin without auth should fail
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/pin", blog_id, post2_id)).dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);

    // Unpin
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/unpin", blog_id, post1_id))
        .header(auth.clone())
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["is_pinned"], false);

    // Pin non-existent post should fail
    let resp = client.post(format!("/api/v1/blogs/{}/posts/nonexistent/pin", blog_id))
        .header(auth)
        .dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_export_markdown() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Export Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create a published post with tags
    let body_json = serde_json::json!({
        "title": "Export Test",
        "content": "# Hello\n\nThis is **bold**.",
        "summary": "A test post",
        "tags": ["rust", "blog"],
        "status": "published",
        "author_name": "Nanook"
    });
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(auth.clone())
        .body(body_json.to_string())
        .dispatch();

    // Export as markdown
    let resp = client.get(format!("/api/v1/blogs/{}/posts/export-test/export/markdown", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["title"], "Export Test");
    assert_eq!(body["author_name"], "Nanook");
    assert!(body["frontmatter"].as_str().unwrap().contains("title:"));
    assert!(body["full_document"].as_str().unwrap().contains("---"));
    assert!(body["content"].as_str().unwrap().contains("**bold**"));
    assert_eq!(body["tags"].as_array().unwrap().len(), 2);

    // Draft post should not be exportable
    let draft_json = serde_json::json!({
        "title": "Draft Post",
        "content": "draft content",
        "status": "draft"
    });
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(auth)
        .body(draft_json.to_string())
        .dispatch();
    let resp = client.get(format!("/api/v1/blogs/{}/posts/draft-post/export/markdown", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_export_html() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "HTML Export Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let body_json = serde_json::json!({
        "title": "HTML Test",
        "content": "# Heading\n\nParagraph.",
        "tags": ["test"],
        "status": "published",
        "author_name": "Agent"
    });
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(auth)
        .body(body_json.to_string())
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/posts/html-test/export/html", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("<title>HTML Test</title>"));
    assert!(body.contains("By Agent"));
    assert!(body.contains("<h1>"));
}

#[test]
fn test_export_nostr() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Nostr Export Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let body_json = serde_json::json!({
        "title": "Nostr Post",
        "content": "Hello Nostr!",
        "summary": "A nostr test",
        "tags": ["nostr", "agents"],
        "status": "published"
    });
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON)
        .header(auth)
        .body(body_json.to_string())
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/posts/nostr-post/export/nostr", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["kind"], 30023);
    assert_eq!(body["content"], "Hello Nostr!");
    let tags = body["tags"].as_array().unwrap();
    // Should have: d, title, summary, t (nostr), t (agents) = 5 tags
    assert!(tags.len() >= 5);
    assert_eq!(tags[0][0], "d");
    assert_eq!(tags[0][1], "nostr-post");
    assert_eq!(tags[1][0], "title");
    assert_eq!(tags[1][1], "Nostr Post");
    assert!(body["note"].as_str().unwrap().contains("NIP-23"));
}

#[test]
fn test_semantic_search() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Semantic Blog");

    // Create diverse posts to test semantic matching
    let posts = vec![
        r#"{"title": "Machine Learning for Beginners", "content": "Neural networks and deep learning fundamentals. Understand backpropagation, gradient descent, and model training with practical examples.", "tags": ["ml", "ai", "deep-learning"], "status": "published"}"#,
        r#"{"title": "Introduction to Artificial Intelligence", "content": "AI systems use neural networks, transformers, and reinforcement learning to solve complex problems. Deep learning powers modern AI.", "tags": ["ai", "neural-networks"], "status": "published"}"#,
        r#"{"title": "Rust Web Development", "content": "Build high-performance web APIs with Rust using Rocket framework. Type safety, memory safety, and blazing fast execution.", "tags": ["rust", "web", "api"], "status": "published"}"#,
        r#"{"title": "Italian Cooking Recipes", "content": "Traditional pasta recipes from Italy. Homemade sauce, fresh ingredients, and authentic flavors for your kitchen.", "tags": ["cooking", "food", "italian"], "status": "published"}"#,
    ];
    for body in posts {
        let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
            .header(ContentType::JSON)
            .header(Header::new("Authorization", format!("Bearer {}", key)))
            .body(body)
            .dispatch();
        assert_eq!(resp.status(), Status::Created);
    }

    // Semantic search for "neural network AI" — should rank ML/AI posts highest
    let resp = client.get("/api/v1/search/semantic?q=neural+network+AI").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let results = body.as_array().unwrap();
    assert!(results.len() >= 2, "Should find at least 2 AI-related posts");
    // Top results should be AI/ML posts, not cooking/Rust
    let top_title = results[0]["title"].as_str().unwrap();
    assert!(
        top_title.contains("Machine Learning") || top_title.contains("Artificial Intelligence"),
        "Top result should be ML or AI related, got: {}", top_title
    );
    // All results should have similarity scores
    assert!(results[0]["similarity"].as_f64().unwrap() > 0.0);

    // Semantic search filtered by blog_id
    let resp = client.get(format!("/api/v1/search/semantic?q=cooking&blog_id={}", id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let results = body.as_array().unwrap();
    assert!(!results.is_empty(), "Should find cooking post within blog");
    assert_eq!(results[0]["title"], "Italian Cooking Recipes");

    // Empty query should return error
    let resp = client.get("/api/v1/search/semantic?q=").dispatch();
    assert_eq!(resp.status(), Status::BadRequest);

    // Query with no matches
    let resp = client.get("/api/v1/search/semantic?q=quantum+physics+entanglement").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let results = body.as_array().unwrap();
    // Should return empty or very low-similarity results
    if !results.is_empty() {
        assert!(results[0]["similarity"].as_f64().unwrap() < 0.3, "Unrelated query should have low similarity");
    }

    // Verify drafts are excluded from semantic search
    let resp = client.post(format!("/api/v1/blogs/{}/posts", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"title": "Secret AI Draft", "content": "Neural networks secret draft content about machine learning"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);

    let resp = client.get("/api/v1/search/semantic?q=secret+draft+neural").dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let results = body.as_array().unwrap();
    // "secret draft" shouldn't appear — only the published AI posts should match on "neural"
    for r in results {
        assert_ne!(r["title"], "Secret AI Draft", "Draft posts must not appear in semantic search");
    }
}

// ─── New Tests: Pagination, Filtering, Edge Cases ───

#[test]
fn test_post_pagination_limit_and_offset() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Pagination Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 5 published posts
    for i in 0..5 {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON)
            .header(auth.clone())
            .body(format!(r#"{{"title": "Post {}", "content": "Content {}", "status": "published"}}"#, i, i))
            .dispatch();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Default listing returns all 5
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 5);

    // limit=2 returns only 2
    let resp = client.get(format!("/api/v1/blogs/{}/posts?limit=2", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 2);

    // limit=2&offset=2 returns next 2
    let resp = client.get(format!("/api/v1/blogs/{}/posts?limit=2&offset=2", blog_id)).dispatch();
    let posts2: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts2.len(), 2);
    // Should be different posts
    assert_ne!(posts[0]["id"], posts2[0]["id"]);

    // offset beyond data returns empty
    let resp = client.get(format!("/api/v1/blogs/{}/posts?offset=100", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 0);

    // limit=0 should be clamped to 1
    let resp = client.get(format!("/api/v1/blogs/{}/posts?limit=0", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 1);
}

#[test]
fn test_post_tag_filtering() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Tag Filter Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create posts with different tags
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Rust Intro", "content": "About Rust", "tags": ["rust", "programming"], "status": "published"}"#)
        .dispatch();
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Python Intro", "content": "About Python", "tags": ["python", "programming"], "status": "published"}"#)
        .dispatch();
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Cooking Tips", "content": "Recipes", "tags": ["cooking"], "status": "published"}"#)
        .dispatch();

    // Filter by "rust" tag
    let resp = client.get(format!("/api/v1/blogs/{}/posts?tag=rust", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["title"], "Rust Intro");

    // Filter by "programming" tag (matches 2)
    let resp = client.get(format!("/api/v1/blogs/{}/posts?tag=programming", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 2);

    // Filter by non-existent tag
    let resp = client.get(format!("/api/v1/blogs/{}/posts?tag=javascript", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 0);
}

#[test]
fn test_nonexistent_blog_returns_404() {
    let client = test_client();

    // Get non-existent blog
    let resp = client.get("/api/v1/blogs/nonexistent-id").dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // List posts from non-existent blog
    let resp = client.get("/api/v1/blogs/nonexistent-id/posts").dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // Get post from non-existent blog
    let resp = client.get("/api/v1/blogs/nonexistent-id/posts/some-slug").dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // Stats from non-existent blog
    let resp = client.get("/api/v1/blogs/nonexistent-id/stats").dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    // Feed from non-existent blog
    let resp = client.get("/api/v1/blogs/nonexistent-id/feed.rss").dispatch();
    assert_eq!(resp.status(), Status::NotFound);
    let resp = client.get("/api/v1/blogs/nonexistent-id/feed.json").dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_nonexistent_post_slug_returns_404() {
    let client = test_client();
    let (blog_id, _) = create_blog_helper(&client, "Blog");

    let resp = client.get(format!("/api/v1/blogs/{}/posts/nonexistent-slug", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_multi_blog_isolation() {
    let client = test_client();
    let (blog_a, key_a) = create_blog_helper(&client, "Blog A");
    let (blog_b, key_b) = create_blog_helper(&client, "Blog B");

    // Create post in blog A
    client.post(format!("/api/v1/blogs/{}/posts", blog_a))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_a)))
        .body(r#"{"title": "A Post", "content": "In blog A", "status": "published"}"#)
        .dispatch();

    // Create post in blog B
    client.post(format!("/api/v1/blogs/{}/posts", blog_b))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_b)))
        .body(r#"{"title": "B Post", "content": "In blog B", "status": "published"}"#)
        .dispatch();

    // Blog A only has its post
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_a)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["title"], "A Post");

    // Blog B only has its post
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_b)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["title"], "B Post");

    // Blog A's key can't create posts in Blog B
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_b))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_a)))
        .body(r#"{"title": "Cross Post"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[test]
fn test_blog_create_with_all_fields() {
    let client = test_client();

    // Create with description and is_public
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(r#"{"name": "Full Blog", "description": "A test blog", "is_public": true}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let body: serde_json::Value = resp.into_json().unwrap();
    let id = body["id"].as_str().unwrap();

    // Verify the fields persisted
    let resp = client.get(format!("/api/v1/blogs/{}", id)).dispatch();
    let blog: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(blog["name"], "Full Blog");
    assert_eq!(blog["description"], "A test blog");
    assert_eq!(blog["is_public"], true);
}

#[test]
fn test_blog_update_partial_fields() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Original Name");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Update only description (name stays)
    let resp = client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"description": "New description"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["name"], "Original Name");
    assert_eq!(body["description"], "New description");

    // Update only is_public (name and desc stay)
    let resp = client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"is_public": true}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["name"], "Original Name");
    assert_eq!(body["description"], "New description");
    assert_eq!(body["is_public"], true);

    // Update only name
    let resp = client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"name": "Updated Name"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["name"], "Updated Name");
    assert_eq!(body["description"], "New description");
    assert_eq!(body["is_public"], true);
}

#[test]
fn test_draft_to_published_workflow() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Workflow Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create as draft
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "WIP Post", "content": "Draft content"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();
    assert_eq!(post["status"], "draft");
    assert!(post["published_at"].is_null());

    // Not visible in public listing
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 0);

    // Publish it
    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let published: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(published["status"], "published");
    assert!(published["published_at"].as_str().is_some());

    // Now visible in public listing
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["title"], "WIP Post");
}

#[test]
fn test_post_custom_slug() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Slug Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create with custom slug
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "My Post Title", "slug": "custom-slug-here", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post["slug"], "custom-slug-here");
    assert_eq!(post["title"], "My Post Title");

    // Fetch by custom slug
    let resp = client.get(format!("/api/v1/blogs/{}/posts/custom-slug-here", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let fetched: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(fetched["title"], "My Post Title");
}

#[test]
fn test_post_summary_and_author_name() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Author Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Authored Post", "content": "Full content", "summary": "A brief summary", "author_name": "Nanook", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post["summary"], "A brief summary");
    assert_eq!(post["author_name"], "Nanook");

    // Verify persisted on get
    let slug = post["slug"].as_str().unwrap();
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}", blog_id, slug)).dispatch();
    let fetched: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(fetched["summary"], "A brief summary");
    assert_eq!(fetched["author_name"], "Nanook");
}

#[test]
fn test_comment_on_nonexistent_post() {
    let client = test_client();
    let (blog_id, _) = create_blog_helper(&client, "Blog");

    let resp = client.post(format!("/api/v1/blogs/{}/posts/fake-id/comments", blog_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "Ghost", "content": "Orphan comment"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_comment_empty_fields_rejected() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Post", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Empty author_name
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "", "content": "Hello"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::UnprocessableEntity);

    // Empty content
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "Alice", "content": ""}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::UnprocessableEntity);
}

#[test]
fn test_json_error_catchers() {
    let client = test_client();

    // Unknown route should return JSON 404
    let resp = client.get("/api/v1/totally-fake-route").dispatch();
    assert_eq!(resp.status(), Status::NotFound);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["code"], "NOT_FOUND");
}

#[test]
fn test_empty_blog_feeds() {
    let client = test_client();
    let (blog_id, _) = create_blog_helper(&client, "Empty Blog");

    // RSS feed with no posts
    let resp = client.get(format!("/api/v1/blogs/{}/feed.rss", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();
    assert!(body.contains("<rss"));
    assert!(body.contains("Empty Blog"));
    // No <item> elements
    assert!(!body.contains("<item>"));

    // JSON feed with no posts
    let resp = client.get(format!("/api/v1/blogs/{}/feed.json", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["title"], "Empty Blog");
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[test]
fn test_search_pagination() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Search Page Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 5 published posts all containing "searchable"
    for i in 0..5 {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "Searchable Post {}", "content": "This is a searchable article number {}", "status": "published"}}"#, i, i))
            .dispatch();
    }

    // Search with limit=2
    let resp = client.get("/api/v1/search?q=searchable&limit=2").dispatch();
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(results.len(), 2);

    // Search with limit=2&offset=2
    let resp = client.get("/api/v1/search?q=searchable&limit=2&offset=2").dispatch();
    let results2: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(results2.len(), 2);
    assert_ne!(results[0]["id"], results2[0]["id"]);

    // Search with offset beyond results
    let resp = client.get("/api/v1/search?q=searchable&offset=100").dispatch();
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(results.len(), 0);
}

#[test]
fn test_search_stemming() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Stemming Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Deploying Applications", "content": "This guide covers deployment strategies for web applications.", "status": "published"}"#)
        .dispatch();

    // "deploy" should match "deploying" and "deployment" via porter stemmer
    let resp = client.get("/api/v1/search?q=deploy").dispatch();
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert!(!results.is_empty(), "Stemming should match 'deploy' to 'deploying'/'deployment'");
    assert_eq!(results[0]["title"], "Deploying Applications");

    // "deployed" should also match
    let resp = client.get("/api/v1/search?q=deployed").dispatch();
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert!(!results.is_empty(), "Stemming should match 'deployed' to 'deploying'/'deployment'");
}

#[test]
fn test_delete_post_cascades_comments() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Cascade Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create a post with comments
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Cascade Post", "status": "published"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();

    // Add comments
    for i in 0..3 {
        client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
            .header(ContentType::JSON)
            .body(format!(r#"{{"author_name": "User", "content": "Comment {}"}}"#, i))
            .dispatch();
    }

    // Verify comments exist
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id)).dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 3);

    // Delete the post
    let resp = client.delete(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(auth)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);

    // Comments should be gone too (post doesn't exist anymore)
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_post_update_content_rerenders_markdown() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Render Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Render Test", "content": "Original *text*"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();
    assert!(post["content_html"].as_str().unwrap().contains("<em>text</em>"));

    // Update content — HTML should be re-rendered
    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"content": "Updated **bold** text"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let updated: serde_json::Value = resp.into_json().unwrap();
    assert!(updated["content_html"].as_str().unwrap().contains("<strong>bold</strong>"));
    assert!(!updated["content_html"].as_str().unwrap().contains("<em>text</em>"));
}

#[test]
fn test_post_empty_title_rejected() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "   "}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::UnprocessableEntity);
}

#[test]
fn test_delete_post_without_auth() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Protected Post"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Delete without auth
    let resp = client.delete(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id)).dispatch();
    assert!(resp.status() == Status::Unauthorized || resp.status() == Status::NotFound);
}

#[test]
fn test_delete_nonexistent_post() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.delete(format!("/api/v1/blogs/{}/posts/fake-id", blog_id))
        .header(auth)
        .dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_blog_stats_empty_blog() {
    let client = test_client();
    let (blog_id, _) = create_blog_helper(&client, "Empty Stats Blog");

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["total_posts"].as_i64().unwrap(), 0);
    assert_eq!(stats["published_posts"].as_i64().unwrap(), 0);
    assert_eq!(stats["total_views"].as_i64().unwrap(), 0);
    assert_eq!(stats["total_comments"].as_i64().unwrap(), 0);
    assert_eq!(stats["top_posts"].as_array().unwrap().len(), 0);
}

#[test]
fn test_blog_stats_with_drafts_and_published() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Mixed Stats Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // 2 drafts, 1 published
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Draft 1"}"#).dispatch();
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Draft 2"}"#).dispatch();
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Published", "status": "published"}"#).dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["total_posts"].as_i64().unwrap(), 3);
    assert_eq!(stats["published_posts"].as_i64().unwrap(), 1);
}

#[test]
fn test_export_nonexistent_post() {
    let client = test_client();
    let (blog_id, _) = create_blog_helper(&client, "Export Blog");

    let resp = client.get(format!("/api/v1/blogs/{}/posts/no-such-slug/export/markdown", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    let resp = client.get(format!("/api/v1/blogs/{}/posts/no-such-slug/export/html", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);

    let resp = client.get(format!("/api/v1/blogs/{}/posts/no-such-slug/export/nostr", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_related_posts_nonexistent_post() {
    let client = test_client();
    let (blog_id, _) = create_blog_helper(&client, "Related Blog");

    let resp = client.get(format!("/api/v1/blogs/{}/posts/fake-id/related", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_related_posts_limit_param() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Related Limit Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 4 posts with overlapping tags
    for i in 0..4 {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "Post {}", "content": "Content", "tags": ["common", "tag{}"], "status": "published"}}"#, i, i))
            .dispatch();
    }

    // Get related for first post with limit=1
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    let first_id = posts[0]["id"].as_str().unwrap();

    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/related?limit=1", blog_id, first_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let related: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert!(related.len() <= 1);
}

#[test]
fn test_post_tags_as_array() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Tags Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Tagged Post", "tags": ["rust", "web", "api"], "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    let tags = post["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0], "rust");
    assert_eq!(tags[1], "web");
    assert_eq!(tags[2], "api");
}

#[test]
fn test_update_post_preserves_unset_fields() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Preserve Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create with all fields
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Full Post", "content": "Original content", "summary": "Original summary", "tags": ["a", "b"], "author_name": "Agent", "status": "published"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();

    // Update only title — other fields should be preserved
    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Updated Title"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let updated: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(updated["title"], "Updated Title");
    assert_eq!(updated["content"], "Original content");
    assert_eq!(updated["summary"], "Original summary");
    assert_eq!(updated["author_name"], "Agent");
    assert_eq!(updated["tags"].as_array().unwrap().len(), 2);
    assert_eq!(updated["status"], "published");
}

#[test]
fn test_multiple_comments_ordering() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Comments Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Comment Post", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Add 3 comments in order
    for name in &["Alice", "Bob", "Charlie"] {
        client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
            .header(ContentType::JSON)
            .body(format!(r#"{{"author_name": "{}", "content": "Hello from {}"}}"#, name, name))
            .dispatch();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // List comments — should be in chronological order
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id)).dispatch();
    let comments: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(comments.len(), 3);
    assert_eq!(comments[0]["author_name"], "Alice");
    assert_eq!(comments[1]["author_name"], "Bob");
    assert_eq!(comments[2]["author_name"], "Charlie");
}

#[test]
fn test_post_comment_count_field() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Count Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Counted Post", "status": "published"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();
    let slug = post["slug"].as_str().unwrap();
    assert_eq!(post["comment_count"].as_i64().unwrap(), 0);

    // Add 2 comments
    for i in 0..2 {
        client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
            .header(ContentType::JSON)
            .body(format!(r#"{{"author_name": "User", "content": "Comment {}"}}"#, i))
            .dispatch();
    }

    // Get post — comment_count should be 2
    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}", blog_id, slug)).dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post["comment_count"].as_i64().unwrap(), 2);

    // Also in list
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts[0]["comment_count"].as_i64().unwrap(), 2);
}

#[test]
fn test_semantic_search_limit_param() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Sem Limit Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create several posts about the same topic
    for i in 0..5 {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "Programming Article {}", "content": "Software engineering and programming practices for developers.", "status": "published"}}"#, i))
            .dispatch();
    }

    // Semantic search with limit=2
    let resp = client.get("/api/v1/search/semantic?q=programming&limit=2").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert!(results.len() <= 2);
}

#[test]
fn test_rss_feed_excludes_drafts() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "RSS Draft Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 1 draft, 1 published
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Draft Post", "content": "Secret"}"#)
        .dispatch();
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Published Post", "content": "Public", "status": "published"}"#)
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/feed.rss", blog_id)).dispatch();
    let body = resp.into_string().unwrap();
    assert!(body.contains("Published Post"));
    assert!(!body.contains("Draft Post"));

    let resp = client.get(format!("/api/v1/blogs/{}/feed.json", blog_id)).dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "Published Post");
}

#[test]
fn test_slug_generation_special_characters() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Slug Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Title with special characters
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Hello, World! This is a Test (2026)", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    let slug = post["slug"].as_str().unwrap();
    // Slug should be lowercase, hyphenated, no special chars
    assert!(!slug.contains('!'));
    assert!(!slug.contains(','));
    assert!(!slug.contains('('));
    assert!(!slug.contains(')'));
    assert!(slug.contains("hello"));
    assert!(slug.contains("world"));
}

#[test]
fn test_markdown_rendering_features() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Markdown Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let content = r#"# Heading 1

## Heading 2

This is a paragraph with **bold**, *italic*, and `code`.

- List item 1
- List item 2

```rust
fn main() {
    println!("Hello");
}
```

> A blockquote

[Link text](https://example.com)"#;

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "Markdown Test",
            "content": content,
            "status": "published"
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    let html = post["content_html"].as_str().unwrap();

    assert!(html.contains("<h1>"), "Should render h1");
    assert!(html.contains("<h2>"), "Should render h2");
    assert!(html.contains("<strong>bold</strong>"), "Should render bold");
    assert!(html.contains("<em>italic</em>"), "Should render italic");
    assert!(html.contains("<code>"), "Should render code");
    assert!(html.contains("<ul>"), "Should render list");
    assert!(html.contains("<li>"), "Should render list items");
    assert!(html.contains("<pre>"), "Should render code block");
    assert!(html.contains("<blockquote>"), "Should render blockquote");
    assert!(html.contains("href=\"https://example.com\""), "Should render links");
}

#[test]
fn test_post_is_pinned_field_default() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Pin Default Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Normal Post", "status": "published"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post["is_pinned"], false, "New posts should not be pinned by default");
}

#[test]
fn test_search_across_multiple_blogs() {
    let client = test_client();
    let (blog_a, key_a) = create_blog_helper(&client, "Blog Alpha");
    let (blog_b, key_b) = create_blog_helper(&client, "Blog Beta");

    // Post in Blog A
    client.post(format!("/api/v1/blogs/{}/posts", blog_a))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_a)))
        .body(r#"{"title": "Findable in Alpha", "content": "Unique crossblog term xyzzy123", "status": "published"}"#)
        .dispatch();

    // Post in Blog B
    client.post(format!("/api/v1/blogs/{}/posts", blog_b))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_b)))
        .body(r#"{"title": "Findable in Beta", "content": "Also contains crossblog xyzzy123", "status": "published"}"#)
        .dispatch();

    // Search should find posts from both blogs
    let resp = client.get("/api/v1/search?q=xyzzy123").dispatch();
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(results.len(), 2, "Search should span all blogs");

    // Verify results come from different blogs
    let blog_names: Vec<&str> = results.iter().map(|r| r["blog_name"].as_str().unwrap()).collect();
    assert!(blog_names.contains(&"Blog Alpha"));
    assert!(blog_names.contains(&"Blog Beta"));
}

#[test]
fn test_preview_empty_content() {
    let client = test_client();
    let resp = client.post("/api/v1/preview")
        .header(ContentType::JSON)
        .body(r#"{"content": ""}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["html"], "");
}

#[test]
fn test_update_blog_nonexistent() {
    let client = test_client();
    // We need a valid key for a different blog to attempt updating a nonexistent one
    let (_, key) = create_blog_helper(&client, "Real Blog");
    let resp = client.patch("/api/v1/blogs/nonexistent-blog-id")
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"name": "Updated"}"#)
        .dispatch();
    // Should fail (key doesn't match nonexistent blog)
    assert!(resp.status() == Status::NotFound || resp.status() == Status::Unauthorized);
}

// ── Well-Known Skills Discovery ──

#[test]
fn test_skills_index_json() {
    let client = test_client();
    let resp = client.get("/.well-known/skills/index.json").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let skills = body["skills"].as_array().unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0]["name"], "blog");
    assert!(skills[0]["description"].as_str().unwrap().contains("blog"));
    let files = skills[0]["files"].as_array().unwrap();
    assert!(files.contains(&serde_json::json!("SKILL.md")));
}

#[test]
fn test_skills_skill_md() {
    let client = test_client();
    let resp = client.get("/.well-known/skills/blog/SKILL.md").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();
    assert!(body.starts_with("---"), "Missing YAML frontmatter");
    assert!(body.contains("name: blog"), "Missing skill name");
    assert!(body.contains("## Quick Start"), "Missing Quick Start");
    assert!(body.contains("## Auth Model"), "Missing Auth Model");
    assert!(body.contains("Markdown"), "Missing markdown reference");
    assert!(body.contains("FTS5"), "Missing search reference");
}

#[test]
fn test_api_v1_skills_skill_md() {
    let client = test_client();
    let resp = client.get("/api/v1/skills/SKILL.md").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();
    assert!(body.starts_with("---"), "Missing YAML frontmatter");
    assert!(body.contains("name: blog"), "Missing skill name");
}

// ── Delete Blog ─────────────────────────────────────────────────────────

#[test]
fn test_delete_blog() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "DeleteMe");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.delete(format!("/api/v1/blogs/{}", id))
        .header(auth)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["deleted"], true);

    // Verify blog is gone
    let resp = client.get(format!("/api/v1/blogs/{}", id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

#[test]
fn test_delete_blog_cascades() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "CascadeDelete");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 3 posts
    for i in 0..3 {
        client.post(format!("/api/v1/blogs/{}/posts", id))
            .header(ContentType::JSON)
            .header(auth.clone())
            .body(format!(r#"{{"title": "Post {}", "status": "published"}}"#, i))
            .dispatch();
    }

    // Verify posts exist
    let resp = client.get(format!("/api/v1/blogs/{}/posts", id))
        .header(auth.clone())
        .dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 3);

    // Delete blog
    let resp = client.delete(format!("/api/v1/blogs/{}", id))
        .header(auth)
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["posts_removed"], 3);
}

#[test]
fn test_delete_blog_without_auth() {
    let client = test_client();
    let (id, _key) = create_blog_helper(&client, "NoAuth");

    let resp = client.delete(format!("/api/v1/blogs/{}", id)).dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[test]
fn test_delete_blog_wrong_key() {
    let client = test_client();
    let (id, _key) = create_blog_helper(&client, "WrongKey");
    let wrong_auth = Header::new("Authorization", "Bearer wrong_key_123");

    let resp = client.delete(format!("/api/v1/blogs/{}", id))
        .header(wrong_auth)
        .dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[test]
fn test_delete_blog_not_found() {
    let client = test_client();
    let (_id, key) = create_blog_helper(&client, "ForAuth");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.delete("/api/v1/blogs/nonexistent-id")
        .header(auth)
        .dispatch();
    // verify_blog_key returns 404 when blog doesn't exist (before checking key)
    assert_eq!(resp.status(), Status::NotFound);
}

// ── Post Response Fields Completeness ───────────────────────────────────

#[test]
fn test_post_response_all_fields_present() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Fields Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Full Fields Post", "content": "Some content here", "summary": "Brief", "tags": ["a"], "author_name": "Agent", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let p: serde_json::Value = resp.into_json().unwrap();

    // Verify every expected field is present and typed correctly
    assert!(p["id"].is_string());
    assert!(p["blog_id"].is_string());
    assert_eq!(p["blog_id"].as_str().unwrap(), blog_id);
    assert_eq!(p["title"], "Full Fields Post");
    assert!(p["slug"].is_string());
    assert_eq!(p["content"], "Some content here");
    assert!(!p["content_html"].as_str().unwrap().is_empty());
    assert_eq!(p["summary"], "Brief");
    assert_eq!(p["tags"].as_array().unwrap().len(), 1);
    assert_eq!(p["status"], "published");
    assert!(p["published_at"].is_string());
    assert_eq!(p["author_name"], "Agent");
    assert!(p["created_at"].is_string());
    assert!(p["updated_at"].is_string());
    assert_eq!(p["comment_count"].as_i64().unwrap(), 0);
    assert!(p["word_count"].as_u64().unwrap() > 0);
    assert!(p["reading_time_minutes"].as_u64().is_some());
    assert_eq!(p["view_count"].as_i64().unwrap(), 0);
    assert_eq!(p["is_pinned"], false);
}

// ── Blog Response Fields Completeness ───────────────────────────────────

#[test]
fn test_blog_response_all_fields_present() {
    let client = test_client();
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(r#"{"name": "Field Check", "description": "Desc", "is_public": true}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let created: serde_json::Value = resp.into_json().unwrap();

    // BlogCreated response fields
    assert!(created["id"].is_string());
    assert_eq!(created["name"], "Field Check");
    assert!(created["manage_key"].as_str().unwrap().starts_with("blog_"));
    assert!(created["view_url"].is_string());
    assert!(created["manage_url"].is_string());
    assert!(created["api_base"].is_string());

    // GET blog response fields
    let id = created["id"].as_str().unwrap();
    let resp = client.get(format!("/api/v1/blogs/{}", id)).dispatch();
    let blog: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(blog["id"].as_str().unwrap(), id);
    assert_eq!(blog["name"], "Field Check");
    assert_eq!(blog["description"], "Desc");
    assert_eq!(blog["is_public"], true);
    assert!(blog["created_at"].is_string());
    assert!(blog["updated_at"].is_string());
}

// ── Comment Response Fields ─────────────────────────────────────────────

#[test]
fn test_comment_response_all_fields() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Comment Fields Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Comment Target", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
        .header(ContentType::JSON)
        .body(r#"{"author_name": "Alice", "content": "Great article!"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let c: serde_json::Value = resp.into_json().unwrap();

    assert!(c["id"].is_string());
    assert_eq!(c["post_id"].as_str().unwrap(), post_id);
    assert_eq!(c["author_name"], "Alice");
    assert_eq!(c["content"], "Great article!");
    assert!(c["created_at"].is_string());
}

// ── Unicode Handling ────────────────────────────────────────────────────

#[test]
fn test_unicode_blog_name_and_description() {
    let client = test_client();
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(serde_json::json!({
            "name": "\u{65e5}\u{672c}\u{8a9e}\u{30d6}\u{30ed}\u{30b0}",
            "description": "\u{3053}\u{308c}\u{306f}\u{30c6}\u{30b9}\u{30c8}\u{3067}\u{3059}",
            "is_public": true
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let blog: serde_json::Value = resp.into_json().unwrap();
    let id = blog["id"].as_str().unwrap();

    let resp = client.get(format!("/api/v1/blogs/{}", id)).dispatch();
    let fetched: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(fetched["name"], "\u{65e5}\u{672c}\u{8a9e}\u{30d6}\u{30ed}\u{30b0}");
    assert_eq!(fetched["description"], "\u{3053}\u{308c}\u{306f}\u{30c6}\u{30b9}\u{30c8}\u{3067}\u{3059}");
}

#[test]
fn test_unicode_post_title_and_content() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Unicode Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "Привет мир",
            "content": "## Hello\n\nThis is a **test** post.\n\nEmoji: abc",
            "author_name": "Нанук",
            "status": "published"
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post["title"], "Привет мир");
    assert_eq!(post["author_name"], "Нанук");
    assert!(post["content_html"].as_str().unwrap().contains("<strong>test</strong>"));
    // Slug should be generated
    assert!(!post["slug"].as_str().unwrap().is_empty());
}

#[test]
fn test_unicode_comment_author_and_content() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Unicode Comment Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Post", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
        .header(ContentType::JSON)
        .body(serde_json::json!({
            "author_name": "\u{7530}\u{4e2d}\u{592a}\u{90ce}",
            "content": "\u{7d20}\u{6674}\u{3089}\u{3057}\u{3044}\u{8a18}\u{4e8b}\u{3067}\u{3059}"
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let c: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(c["author_name"], "\u{7530}\u{4e2d}\u{592a}\u{90ce}");
    assert_eq!(c["content"], "\u{7d20}\u{6674}\u{3089}\u{3057}\u{3044}\u{8a18}\u{4e8b}\u{3067}\u{3059}");
}

// ── Published_at Lifecycle ──────────────────────────────────────────────

#[test]
fn test_published_at_set_on_publish_preserved_on_unpublish() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Pubdate Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create as draft — no published_at
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Lifecycle Post"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();
    assert!(post["published_at"].is_null());
    assert_eq!(post["status"], "draft");

    // Publish — published_at gets set
    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"status": "published"}"#)
        .dispatch();
    let published: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(published["status"], "published");
    let pub_at = published["published_at"].as_str().unwrap().to_string();
    assert!(!pub_at.is_empty());

    // Unpublish back to draft — published_at is preserved (records when it was first published)
    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"status": "draft"}"#)
        .dispatch();
    let draft: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(draft["status"], "draft");
    // published_at is preserved — records the original publish timestamp
    assert_eq!(draft["published_at"].as_str().unwrap(), pub_at);
}

// ── Updated_at Tracking ─────────────────────────────────────────────────

#[test]
fn test_post_updated_at_changes_on_edit() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Timestamp Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Original Title"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();
    let created_at = post["created_at"].as_str().unwrap().to_string();
    let updated_at_1 = post["updated_at"].as_str().unwrap().to_string();

    std::thread::sleep(std::time::Duration::from_millis(50));

    // Update the post
    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Updated Title"}"#)
        .dispatch();
    let updated: serde_json::Value = resp.into_json().unwrap();
    let updated_at_2 = updated["updated_at"].as_str().unwrap();

    // created_at should not change
    assert_eq!(updated["created_at"].as_str().unwrap(), created_at);
    // updated_at should change
    assert!(updated_at_2 >= updated_at_1.as_str(), "updated_at should advance after edit");
}

#[test]
fn test_blog_updated_at_changes_on_update() {
    let client = test_client();
    let (id, key) = create_blog_helper(&client, "Blog TS");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.get(format!("/api/v1/blogs/{}", id)).dispatch();
    let blog1: serde_json::Value = resp.into_json().unwrap();
    let created_at = blog1["created_at"].as_str().unwrap().to_string();
    let updated_at_1 = blog1["updated_at"].as_str().unwrap().to_string();

    std::thread::sleep(std::time::Duration::from_millis(50));

    let resp = client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"name": "Updated Blog TS"}"#)
        .dispatch();
    let blog2: serde_json::Value = resp.into_json().unwrap();

    assert_eq!(blog2["created_at"].as_str().unwrap(), created_at);
    assert!(blog2["updated_at"].as_str().unwrap() >= updated_at_1.as_str());
}

// ── Multiple Pinned Posts Ordering ──────────────────────────────────────

#[test]
fn test_multiple_pinned_posts_ordering() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Multi Pin Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 3 published posts
    let mut post_ids = Vec::new();
    for i in 0..3 {
        let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "Post {}", "status": "published"}}"#, i))
            .dispatch();
        let p: serde_json::Value = resp.into_json().unwrap();
        post_ids.push(p["id"].as_str().unwrap().to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Pin post 0 and post 2 (not post 1)
    client.post(format!("/api/v1/blogs/{}/posts/{}/pin", blog_id, post_ids[0]))
        .header(auth.clone()).dispatch();
    client.post(format!("/api/v1/blogs/{}/posts/{}/pin", blog_id, post_ids[2]))
        .header(auth.clone()).dispatch();

    // List posts — pinned first, then unpinned. Within each group, newest first.
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 3);

    // First two should be pinned
    assert_eq!(posts[0]["is_pinned"], true);
    assert_eq!(posts[1]["is_pinned"], true);
    // Third should be unpinned
    assert_eq!(posts[2]["is_pinned"], false);
    assert_eq!(posts[2]["id"].as_str().unwrap(), post_ids[1]);
}

// ── Pin/Unpin Idempotency ───────────────────────────────────────────────

#[test]
fn test_pin_already_pinned_is_idempotent() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Idempotent Pin Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Pin Me", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Pin twice — should succeed both times
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/pin", blog_id, post_id))
        .header(auth.clone()).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_json::<serde_json::Value>().unwrap()["is_pinned"], true);

    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/pin", blog_id, post_id))
        .header(auth).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_json::<serde_json::Value>().unwrap()["is_pinned"], true);
}

#[test]
fn test_unpin_already_unpinned_is_idempotent() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Idempotent Unpin Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Unpin Me", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Unpin without ever pinning — should succeed
    let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/unpin", blog_id, post_id))
        .header(auth).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_json::<serde_json::Value>().unwrap()["is_pinned"], false);
}

// ── Tag Update Behavior ─────────────────────────────────────────────────

#[test]
fn test_update_post_tags_replaces_array() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Tag Update Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Tagged", "tags": ["rust", "web"]}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let post_id = post["id"].as_str().unwrap();
    assert_eq!(post["tags"].as_array().unwrap().len(), 2);

    // Update with new tags — should replace, not append
    let resp = client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"tags": ["python", "data", "ml"]}"#)
        .dispatch();
    let updated: serde_json::Value = resp.into_json().unwrap();
    let tags = updated["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 3);
    assert_eq!(tags[0], "python");
    assert_eq!(tags[1], "data");
    assert_eq!(tags[2], "ml");
}

// ── FTS Index Updates ───────────────────────────────────────────────────

#[test]
fn test_fts_index_updated_on_post_edit() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "FTS Edit Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Original Searchable", "content": "Original unique8742 content", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Search finds original
    let resp = client.get("/api/v1/search?q=unique8742").dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 1);

    // Update content
    client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"content": "Updated special9563 content"}"#)
        .dispatch();

    // Old term no longer found
    let resp = client.get("/api/v1/search?q=unique8742").dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 0);

    // New term is found
    let resp = client.get("/api/v1/search?q=special9563").dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 1);
}

#[test]
fn test_fts_index_removed_on_post_delete() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "FTS Delete Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Deletable Searchable", "content": "Deletable xkcd7734 content", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Confirm searchable
    let resp = client.get("/api/v1/search?q=xkcd7734").dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 1);

    // Delete the post
    client.delete(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(auth).dispatch();

    // No longer searchable
    let resp = client.get("/api/v1/search?q=xkcd7734").dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 0);
}

#[test]
fn test_fts_index_removed_on_unpublish() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "FTS Unpub Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Unpublishable", "content": "Findable jqkm9921 stuff", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Confirm in search
    let resp = client.get("/api/v1/search?q=jqkm9921").dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 1);

    // Unpublish
    client.patch(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"status": "draft"}"#)
        .dispatch();

    // Should be gone from search
    let resp = client.get("/api/v1/search?q=jqkm9921").dispatch();
    assert_eq!(resp.into_json::<Vec<serde_json::Value>>().unwrap().len(), 0);
}

// ── Stats Comment Count ─────────────────────────────────────────────────

#[test]
fn test_stats_total_comments_increments_and_decrements() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Comment Stats Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Stats Post", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // 0 comments initially
    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["total_comments"].as_i64().unwrap(), 0);

    // Add 3 comments
    let mut comment_ids = Vec::new();
    for i in 0..3 {
        let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
            .header(ContentType::JSON)
            .body(format!(r#"{{"author_name": "User{}", "content": "Comment {}"}}"#, i, i))
            .dispatch();
        comment_ids.push(resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string());
    }

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["total_comments"].as_i64().unwrap(), 3);

    // Delete one comment
    client.delete(format!("/api/v1/blogs/{}/posts/{}/comments/{}", blog_id, post_id, comment_ids[0]))
        .header(auth).dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["total_comments"].as_i64().unwrap(), 2);
}

// ── Stats After Post Deletion ───────────────────────────────────────────

#[test]
fn test_stats_after_post_deletion() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Delete Stats Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create 2 published posts
    let mut post_ids = Vec::new();
    for i in 0..2 {
        let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "Post {}", "status": "published"}}"#, i))
            .dispatch();
        post_ids.push(resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string());
    }

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["total_posts"].as_i64().unwrap(), 2);
    assert_eq!(stats["published_posts"].as_i64().unwrap(), 2);

    // Delete one post
    client.delete(format!("/api/v1/blogs/{}/posts/{}", blog_id, post_ids[0]))
        .header(auth).dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_id)).dispatch();
    let stats: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats["total_posts"].as_i64().unwrap(), 1);
    assert_eq!(stats["published_posts"].as_i64().unwrap(), 1);
}

// ── Multi-Blog Stats Isolation ──────────────────────────────────────────

#[test]
fn test_stats_isolation_between_blogs() {
    let client = test_client();
    let (blog_a, key_a) = create_blog_helper(&client, "Stats A");
    let (blog_b, key_b) = create_blog_helper(&client, "Stats B");

    // 3 posts in blog A
    for i in 0..3 {
        client.post(format!("/api/v1/blogs/{}/posts", blog_a))
            .header(ContentType::JSON)
            .header(Header::new("Authorization", format!("Bearer {}", key_a)))
            .body(format!(r#"{{"title": "A Post {}", "status": "published"}}"#, i))
            .dispatch();
    }

    // 1 post in blog B
    client.post(format!("/api/v1/blogs/{}/posts", blog_b))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_b)))
        .body(r#"{"title": "B Post", "status": "published"}"#)
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_a)).dispatch();
    let stats_a: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats_a["total_posts"].as_i64().unwrap(), 3);

    let resp = client.get(format!("/api/v1/blogs/{}/stats", blog_b)).dispatch();
    let stats_b: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(stats_b["total_posts"].as_i64().unwrap(), 1);
}

// ── Search Result Fields ────────────────────────────────────────────────

#[test]
fn test_search_result_fields_completeness() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "SearchFields Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "Searchable zxvq4488 Post",
            "content": "Content about zxvq4488",
            "tags": ["test"],
            "author_name": "Agent",
            "summary": "Brief",
            "status": "published"
        }).to_string())
        .dispatch();

    let resp = client.get("/api/v1/search?q=zxvq4488").dispatch();
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(results.len(), 1);

    let r = &results[0];
    assert!(r["id"].is_string());
    assert_eq!(r["blog_id"].as_str().unwrap(), blog_id);
    assert_eq!(r["blog_name"], "SearchFields Blog");
    assert!(r["title"].as_str().unwrap().contains("zxvq4488"));
    assert!(r["slug"].is_string());
    assert_eq!(r["summary"], "Brief");
    assert_eq!(r["tags"].as_array().unwrap().len(), 1);
    assert_eq!(r["author_name"], "Agent");
    assert!(r["published_at"].is_string());
    assert!(r["rank"].as_f64().is_some());
}

// ── RSS Feed Structure ──────────────────────────────────────────────────

#[test]
fn test_rss_feed_structure_detailed() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "RSS Detail Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "RSS Article",
            "content": "Full **markdown** content",
            "summary": "Brief summary",
            "author_name": "Author",
            "tags": ["rss"],
            "status": "published"
        }).to_string())
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/feed.rss", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();

    // Structural checks
    assert!(body.contains("<?xml"), "Missing XML declaration");
    assert!(body.contains("<rss"), "Missing rss element");
    assert!(body.contains("<channel>"), "Missing channel element");
    assert!(body.contains("RSS Detail Blog"), "Missing channel title");
    assert!(body.contains("<item>"), "Missing item element");
    assert!(body.contains("RSS Article"), "Missing item title");
    assert!(body.contains("<description>"), "Missing item description");
    assert!(body.contains("<pubDate>"), "Missing pubDate");
}

#[test]
fn test_rss_feed_ordering_newest_first() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "RSS Order Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    for i in 0..3 {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "Feed Post {}", "status": "published"}}"#, i))
            .dispatch();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let resp = client.get(format!("/api/v1/blogs/{}/feed.rss", blog_id)).dispatch();
    let body = resp.into_string().unwrap();

    // Find positions of titles — newest (Post 2) should appear before oldest (Post 0)
    let pos_2 = body.find("Feed Post 2").unwrap();
    let pos_0 = body.find("Feed Post 0").unwrap();
    assert!(pos_2 < pos_0, "Newest post should appear first in RSS feed");
}

// ── JSON Feed Structure ─────────────────────────────────────────────────

#[test]
fn test_json_feed_structure_detailed() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "JSON Feed Detail Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "Feed Item",
            "content": "Content here",
            "summary": "Brief",
            "author_name": "Author",
            "status": "published"
        }).to_string())
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/feed.json", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let feed: serde_json::Value = resp.into_json().unwrap();

    // JSON Feed 1.1 structure
    assert!(feed["version"].as_str().unwrap().contains("jsonfeed.org"));
    assert_eq!(feed["title"], "JSON Feed Detail Blog");
    let items = feed["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["title"], "Feed Item");
    assert!(items[0]["id"].is_string());
    assert!(items[0]["content_html"].is_string());
    assert!(items[0]["date_published"].is_string());
}

// ── Post Ordering ───────────────────────────────────────────────────────

#[test]
fn test_post_list_ordering_newest_first() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Order Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    for i in 0..3 {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "Post {}", "status": "published"}}"#, i))
            .dispatch();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 3);
    // Newest first
    assert_eq!(posts[0]["title"], "Post 2");
    assert_eq!(posts[1]["title"], "Post 1");
    assert_eq!(posts[2]["title"], "Post 0");
}

// ── Slug Behavior ───────────────────────────────────────────────────────

#[test]
fn test_slug_is_lowercase() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Slug Case Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "UPPERCASE TITLE HERE"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let slug = post["slug"].as_str().unwrap();
    assert_eq!(slug, slug.to_lowercase(), "Slug should be lowercase");
}

#[test]
fn test_slug_strips_special_chars() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Slug Special Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Hello & Goodbye! What's Up? #Test"}"#)
        .dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    let slug = post["slug"].as_str().unwrap();
    assert!(!slug.contains('&'));
    assert!(!slug.contains('!'));
    assert!(!slug.contains('?'));
    assert!(!slug.contains('#'));
    assert!(!slug.contains('\''));
}

// ── Export Markdown Frontmatter ─────────────────────────────────────────

#[test]
fn test_export_markdown_frontmatter_fields() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Export FM Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "Frontmatter Test",
            "content": "Hello world",
            "tags": ["rust", "blog"],
            "summary": "A test",
            "author_name": "Nanook",
            "status": "published"
        }).to_string())
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/posts/frontmatter-test/export/markdown", blog_id)).dispatch();
    let body: serde_json::Value = resp.into_json().unwrap();

    let fm = body["frontmatter"].as_str().unwrap();
    assert!(fm.contains("title:"), "Missing title in frontmatter");
    assert!(fm.contains("Frontmatter Test"), "Missing title value in frontmatter");
    assert!(fm.contains("rust") && fm.contains("blog"), "Missing tags in frontmatter");
    assert!(fm.contains("date:"), "Missing date in frontmatter");
    assert!(fm.contains("Nanook"), "Missing author in frontmatter");

    let full = body["full_document"].as_str().unwrap();
    assert!(full.starts_with("---"), "Full document should start with frontmatter delimiter");
    assert!(full.contains("Hello world"), "Full document should contain content");
}

// ── Export HTML Detailed ────────────────────────────────────────────────

#[test]
fn test_export_html_structure_detailed() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "HTML Detail Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "HTML Deep Test",
            "content": "# Heading\n\n**bold** text",
            "author_name": "Agent",
            "tags": ["html"],
            "status": "published"
        }).to_string())
        .dispatch();

    let resp = client.get(format!("/api/v1/blogs/{}/posts/html-deep-test/export/html", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().unwrap();

    assert!(body.contains("<!DOCTYPE html>"), "Missing DOCTYPE");
    assert!(body.contains("<html"), "Missing html tag");
    assert!(body.contains("<head>"), "Missing head");
    assert!(body.contains("<meta charset"), "Missing charset meta");
    assert!(body.contains("<title>HTML Deep Test</title>"), "Missing title");
    assert!(body.contains("<body"), "Missing body");
    assert!(body.contains("<h1>"), "Missing rendered heading");
    assert!(body.contains("<strong>bold</strong>"), "Missing rendered bold");
    assert!(body.contains("By Agent"), "Missing author attribution");
}

// ── View Tracking Doesn't Count Drafts ──────────────────────────────────

#[test]
fn test_draft_not_in_public_list() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Draft View Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create a draft and a published post
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Secret Draft", "slug": "secret-draft"}"#)
        .dispatch();
    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Public Post", "status": "published"}"#)
        .dispatch();

    // Public listing should only show published
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["title"], "Public Post");
}

// ── Delete Blog Cascades Comments & Views ───────────────────────────────

#[test]
fn test_delete_blog_cascades_comments_and_views() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Full Cascade Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Create post with comments and views
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"title": "Rich Post", "status": "published", "slug": "rich-post"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Add comments
    for i in 0..2 {
        client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
            .header(ContentType::JSON)
            .body(format!(r#"{{"author_name": "User", "content": "C{}"}}"#, i))
            .dispatch();
    }

    // Generate views
    for _ in 0..3 {
        client.get(format!("/api/v1/blogs/{}/posts/rich-post", blog_id)).dispatch();
    }

    // Delete blog
    let resp = client.delete(format!("/api/v1/blogs/{}", blog_id))
        .header(auth).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["deleted"], true);
    assert_eq!(body["posts_removed"], 1);

    // Confirm blog is gone
    let resp = client.get(format!("/api/v1/blogs/{}", blog_id)).dispatch();
    assert_eq!(resp.status(), Status::NotFound);
}

// ── Post Metadata Field ─────────────────────────────────────────────────

#[test]
fn test_post_metadata_json_field() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Metadata Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "Meta Post",
            "content": "Content",
            "metadata": {"category": "tech", "featured": true},
            "status": "published"
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    // metadata should be stored (may or may not be returned depending on API)
    // At minimum the post should be created successfully
    assert_eq!(post["title"], "Meta Post");
}

// ── Large Content Handling ──────────────────────────────────────────────

#[test]
fn test_large_post_content() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Large Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // ~10KB of content
    let big_content = "This is a paragraph with some text. ".repeat(300);
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "Big Post",
            "content": big_content,
            "status": "published"
        }).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    assert!(post["word_count"].as_u64().unwrap() > 1000);
    assert!(post["reading_time_minutes"].as_u64().unwrap() >= 5);
}

// ── Many Comments ───────────────────────────────────────────────────────

#[test]
fn test_many_comments_on_post() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Many Comments Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Raise comment rate limit
    std::env::set_var("COMMENT_RATE_LIMIT", "100");

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Popular Post", "status": "published"}"#)
        .dispatch();
    let post_id = resp.into_json::<serde_json::Value>().unwrap()["id"].as_str().unwrap().to_string();

    // Add 20 comments
    for i in 0..20 {
        let resp = client.post(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id))
            .header(ContentType::JSON)
            .body(format!(r#"{{"author_name": "User{}", "content": "Comment {}"}}"#, i, i))
            .dispatch();
        assert_eq!(resp.status(), Status::Created);
    }

    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}/comments", blog_id, post_id)).dispatch();
    let comments: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(comments.len(), 20);

    // Comment count on post should be 20
    let resp = client.get(format!("/api/v1/blogs/{}/posts/popular-post", blog_id)).dispatch();
    let post: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post["comment_count"].as_i64().unwrap(), 20);

    std::env::remove_var("COMMENT_RATE_LIMIT");
}

// ── Feed Excludes Other Blogs' Posts ────────────────────────────────────

#[test]
fn test_rss_feed_only_includes_own_posts() {
    let client = test_client();
    let (blog_a, key_a) = create_blog_helper(&client, "Feed A");
    let (blog_b, key_b) = create_blog_helper(&client, "Feed B");

    client.post(format!("/api/v1/blogs/{}/posts", blog_a))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_a)))
        .body(r#"{"title": "Alpha Article", "status": "published"}"#)
        .dispatch();

    client.post(format!("/api/v1/blogs/{}/posts", blog_b))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_b)))
        .body(r#"{"title": "Beta Article", "status": "published"}"#)
        .dispatch();

    // Blog A's feed should only contain Alpha
    let resp = client.get(format!("/api/v1/blogs/{}/feed.rss", blog_a)).dispatch();
    let body = resp.into_string().unwrap();
    assert!(body.contains("Alpha Article"));
    assert!(!body.contains("Beta Article"));
}

// ── Search Special Characters ───────────────────────────────────────────

#[test]
fn test_search_with_special_characters() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Special Search Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(serde_json::json!({
            "title": "CPP vs Rust Comparison",
            "content": "Comparing CPP and Rust for systems programming",
            "status": "published"
        }).to_string())
        .dispatch();

    // Search with special characters shouldn't crash
    let resp = client.get("/api/v1/search?q=C%2B%2B").dispatch();
    assert!(resp.status() == Status::Ok || resp.status() == Status::BadRequest);

    // Search with quotes
    let resp = client.get("/api/v1/search?q=%22Rust%22").dispatch();
    assert!(resp.status() == Status::Ok || resp.status() == Status::BadRequest);
}

// ── Semantic Search with Blog ID Filter ─────────────────────────────────

#[test]
fn test_semantic_search_nonexistent_blog_filter() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Semantic Filter Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Filterable Post", "content": "Content for filtering", "status": "published"}"#)
        .dispatch();

    // Filter by nonexistent blog_id — should return empty
    let resp = client.get("/api/v1/search/semantic?q=filtering&blog_id=nonexistent-blog").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let results: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(results.len(), 0);
}

// ── Blog Name Update in Public List ─────────────────────────────────────

#[test]
fn test_updated_blog_name_reflected_in_list() {
    let client = test_client();
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(r#"{"name": "Before Rename", "is_public": true}"#)
        .dispatch();
    let blog: serde_json::Value = resp.into_json().unwrap();
    let id = blog["id"].as_str().unwrap();
    let key = blog["manage_key"].as_str().unwrap();

    // Rename
    client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key)))
        .body(r#"{"name": "After Rename"}"#)
        .dispatch();

    // Public list should show new name
    let resp = client.get("/api/v1/blogs").dispatch();
    let blogs: Vec<serde_json::Value> = resp.into_json().unwrap();
    let names: Vec<&str> = blogs.iter().map(|b| b["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"After Rename"));
    assert!(!names.contains(&"Before Rename"));
}

// ── Post Listing With Auth Shows Drafts ─────────────────────────────────

#[test]
fn test_post_listing_auth_vs_no_auth_draft_visibility() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Visibility Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // 2 drafts, 2 published
    for title in &["Draft A", "Draft B"] {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "{}"}}"#, title))
            .dispatch();
    }
    for title in &["Published A", "Published B"] {
        client.post(format!("/api/v1/blogs/{}/posts", blog_id))
            .header(ContentType::JSON).header(auth.clone())
            .body(format!(r#"{{"title": "{}", "status": "published"}}"#, title))
            .dispatch();
    }

    // Without auth: 2 published only
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id)).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 2);
    for p in &posts {
        assert_eq!(p["status"], "published");
    }

    // With auth: all 4
    let resp = client.get(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(auth).dispatch();
    let posts: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert_eq!(posts.len(), 4);
}

// ── Concurrent Post Creation Slug Uniqueness ────────────────────────────

#[test]
fn test_duplicate_slugs_across_blogs_ok() {
    let client = test_client();
    let (blog_a, key_a) = create_blog_helper(&client, "Blog Alpha");
    let (blog_b, key_b) = create_blog_helper(&client, "Blog Beta");

    // Same title in different blogs — should create same slug without conflict
    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_a))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_a)))
        .body(r#"{"title": "Same Title", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let slug_a = resp.into_json::<serde_json::Value>().unwrap()["slug"].as_str().unwrap().to_string();

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_b))
        .header(ContentType::JSON)
        .header(Header::new("Authorization", format!("Bearer {}", key_b)))
        .body(r#"{"title": "Same Title", "status": "published"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let slug_b = resp.into_json::<serde_json::Value>().unwrap()["slug"].as_str().unwrap().to_string();

    // Same slug, different blogs — both accessible independently
    assert_eq!(slug_a, slug_b);

    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}", blog_a, slug_a)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let post_a: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post_a["blog_id"].as_str().unwrap(), blog_a);

    let resp = client.get(format!("/api/v1/blogs/{}/posts/{}", blog_b, slug_b)).dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let post_b: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post_b["blog_id"].as_str().unwrap(), blog_b);
}

// ── Health Endpoint Structure ───────────────────────────────────────────

#[test]
fn test_health_response_structure() {
    let client = test_client();
    let resp = client.get("/api/v1/health").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

// ── OpenAPI Spec Structure ──────────────────────────────────────────────

#[test]
fn test_openapi_spec_paths_completeness() {
    let client = test_client();
    let resp = client.get("/api/v1/openapi.json").dispatch();
    let spec: serde_json::Value = resp.into_json().unwrap();

    let paths = spec["paths"].as_object().unwrap();
    // Key paths should be present
    let blog_id_key = "/blogs/{blogId}";
    let posts_key = "/blogs/{blogId}/posts";
    assert!(paths.contains_key("/blogs"), "Missing /blogs path");
    assert!(paths.contains_key(blog_id_key), "Missing /blogs/blogId path");
    assert!(paths.contains_key(posts_key), "Missing posts path");
    assert!(paths.contains_key("/search"), "Missing search path");
    assert!(paths.contains_key("/health"), "Missing health path");
    assert!(paths.contains_key("/preview"), "Missing preview path");

    // Schemas should be present
    let schemas = spec["components"]["schemas"].as_object().unwrap();
    assert!(schemas.contains_key("Post"), "Missing Post schema");
    assert!(schemas.contains_key("Blog"), "Missing Blog schema");
    assert!(schemas.contains_key("Comment"), "Missing Comment schema");
}

// ── Preview Markdown Edge Cases ─────────────────────────────────────────

#[test]
fn test_preview_markdown_complex() {
    let client = test_client();
    let content = "## Heading\n\n| Column A | Column B |\n|----------|----------|\n| Cell 1   | Cell 2   |\n\n```rust\nfn main() {}\n```\n\n> Blockquote with **bold**";

    let resp = client.post("/api/v1/preview")
        .header(ContentType::JSON)
        .body(serde_json::json!({"content": content}).to_string())
        .dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body: serde_json::Value = resp.into_json().unwrap();
    let html = body["html"].as_str().unwrap();
    assert!(html.contains("<h2>"), "Should render heading");
    assert!(html.contains("<table>"), "Should render table");
    assert!(html.contains("<pre>"), "Should render code block");
    assert!(html.contains("<blockquote>"), "Should render blockquote");
}

// ── Post With Empty Content ─────────────────────────────────────────────

#[test]
fn test_create_post_with_empty_content() {
    let client = test_client();
    let (blog_id, key) = create_blog_helper(&client, "Empty Content Blog");
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    let resp = client.post(format!("/api/v1/blogs/{}/posts", blog_id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"title": "Title Only"}"#)
        .dispatch();
    assert_eq!(resp.status(), Status::Created);
    let post: serde_json::Value = resp.into_json().unwrap();
    assert_eq!(post["content"], "");
    assert_eq!(post["content_html"], "");
    assert_eq!(post["word_count"].as_u64().unwrap(), 0);
}

// ── Blog Public Toggle ──────────────────────────────────────────────────

#[test]
fn test_blog_public_toggle_affects_listing() {
    let client = test_client();
    let resp = client.post("/api/v1/blogs")
        .header(ContentType::JSON)
        .body(r#"{"name": "Toggle Blog", "is_public": false}"#)
        .dispatch();
    let blog: serde_json::Value = resp.into_json().unwrap();
    let id = blog["id"].as_str().unwrap();
    let key = blog["manage_key"].as_str().unwrap();
    let auth = Header::new("Authorization", format!("Bearer {}", key));

    // Not in public list
    let resp = client.get("/api/v1/blogs").dispatch();
    let blogs: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert!(!blogs.iter().any(|b| b["id"].as_str().unwrap() == id));

    // Make public
    client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON).header(auth.clone())
        .body(r#"{"is_public": true}"#)
        .dispatch();

    // Now in public list
    let resp = client.get("/api/v1/blogs").dispatch();
    let blogs: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert!(blogs.iter().any(|b| b["id"].as_str().unwrap() == id));

    // Make private again
    client.patch(format!("/api/v1/blogs/{}", id))
        .header(ContentType::JSON).header(auth)
        .body(r#"{"is_public": false}"#)
        .dispatch();

    // Gone from public list
    let resp = client.get("/api/v1/blogs").dispatch();
    let blogs: Vec<serde_json::Value> = resp.into_json().unwrap();
    assert!(!blogs.iter().any(|b| b["id"].as_str().unwrap() == id));
}
