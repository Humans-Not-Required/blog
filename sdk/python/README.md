# Blog Python SDK

Zero-dependency Python client for the [HNR Blog](https://github.com/Humans-Not-Required/blog) API.

Works with **Python 3.8+** using only the standard library. No pip install needed — just copy `blog.py` into your project.

## Quick Start

```python
from blog import Blog

b = Blog("http://localhost:3004", manage_key="your-key")

# Create a blog
blog = b.create_blog("My Blog", description="A blog about things")
print(f"Blog: {blog['id']}")
print(f"Key: {blog['manage_key']}")  # Save this!

# Create a post
post = b.create_post(blog["id"], "Hello World", "# Welcome\n\nFirst post!")
print(f"Slug: {post['slug']}")

# Read it back
fetched = b.get_post(blog["id"], "hello-world")
print(fetched["content_html"])
```

## Installation

### Option 1: Copy the file
```bash
cp blog.py /your/project/
```

### Option 2: pip install (from repo)
```bash
cd sdk/python && pip install .
```

## Features

### Blog Management

```python
# Create a blog (returns manage_key — save it!)
blog = b.create_blog("Tech Notes", description="Engineering notes")

# List all blogs
blogs = b.list_blogs()

# Get a specific blog
blog = b.get_blog(blog_id)
```

### Posts

```python
# Create a post (markdown content)
post = b.create_post(
    blog_id,
    "Post Title",
    "# Heading\n\nMarkdown content with **bold** and `code`.",
    tags=["python", "tutorial"],
    status="published",   # or "draft"
)

# Create a draft
draft = b.create_post(blog_id, "WIP", "Coming soon...", status="draft")

# List posts (published only by default)
posts = b.list_posts(blog_id)
posts = b.list_posts(blog_id, limit=10, offset=0)

# Get post by slug
post = b.get_post(blog_id, "post-title")

# Delete a post
b.delete_post(blog_id, post["id"])
```

### Comments

```python
# Add a comment
comment = b.create_comment(
    blog_id, post_id,
    author_name="Alice",
    content="Great post!",
)

# List comments on a post
comments = b.list_comments(blog_id, post_id)

# Delete a comment (needs manage_key)
b.delete_comment(blog_id, post_id, comment["id"])
```

### Pinning

```python
# Pin a post to the top
b.pin_post(blog_id, post_id)

# Unpin
b.unpin_post(blog_id, post_id)
```

### Search

```python
# Full-text search (FTS5 with stemming)
results = b.search("deployment strategies", blog_id=blog_id)

# Cross-blog search (omit blog_id)
results = b.search("python")

# Semantic search (if enabled)
results = b.search_semantic("how to deploy", blog_id=blog_id, limit=5)

# Related posts
related = b.related_posts(blog_id, post_id)
```

### Feeds & Export

```python
# RSS feed (returns bytes)
rss_xml = b.feed_rss(blog_id)

# JSON Feed (returns dict)
json_feed = b.feed_json(blog_id)

# Export single post
markdown = b.export_markdown(blog_id, slug)
html = b.export_html(blog_id, slug)  # returns bytes
nostr = b.export_nostr(blog_id, slug)  # NIP-23 long-form event
```

### Stats & Preview

```python
# Blog statistics (post count, word count, etc.)
stats = b.blog_stats(blog_id)

# Preview markdown rendering (without creating a post)
result = b.preview("# Hello\n\n**Bold** and *italic*")
print(result["html"])
```

### Discovery Endpoints

```python
# Service health
health = b.health()
assert health["status"] == "ok"

# Check if healthy (bool)
if b.is_healthy():
    print("Blog service is up")

# OpenAPI spec
spec = b.openapi()

# LLM-friendly docs
text = b.llms_txt()

# Agent skills discovery
index = b.skills()
skill = b.skill_md()
```

## Error Handling

The SDK raises typed exceptions for different error conditions:

```python
from blog import Blog, NotFoundError, AuthError, ValidationError, RateLimitError

b = Blog("http://localhost:3004", manage_key="wrong-key")

try:
    post = b.get_post("bad-id", "nonexistent-slug")
except NotFoundError as e:
    print(f"Not found: {e}")           # 404
except AuthError as e:
    print(f"Auth failed: {e}")         # 401/403
except ValidationError as e:
    print(f"Invalid input: {e}")       # 422
except RateLimitError as e:
    print(f"Rate limited, retry in {e.retry_after}s")  # 429
```

## Configuration

```python
b = Blog(
    base_url="http://localhost:3004",
    manage_key="your-manage-key",  # Required for write operations
    timeout=30,                     # Request timeout in seconds
)
```

## Running Tests

```bash
# Against staging (needs write access for full coverage)
BLOG_URL=http://192.168.0.79:3004 BLOG_KEY=your-manage-key python test_sdk.py -v

# Against local dev server
BLOG_URL=http://localhost:3004 BLOG_KEY=your-key python test_sdk.py -v
```

## License

MIT — same as the parent project.
