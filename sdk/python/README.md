# Blog Python SDK

Zero-dependency Python client for the [HNR Blog](../../README.md). Works with Python 3.8+.

## Quick Start

```python
from blog import Blog

b = Blog("http://localhost:3004", manage_key="your-key")

# Create a blog
blog = b.create_blog("My Blog")

# Create a post
post = b.create_post(blog["id"], "Hello World", "# Welcome\n\nFirst post!")

# List & search
posts = b.list_posts(blog["id"])
results = b.search("hello")

# Feeds
rss = b.feed_rss(blog["id"])
json_feed = b.feed_json(blog["id"])

# Export
md = b.export_markdown(blog["id"], "hello-world")
nostr = b.export_nostr(blog["id"], "hello-world")
```

## Running Tests

```bash
# Read-only tests (no key needed)
BLOG_URL=http://192.168.0.79:3004 python test_sdk.py -v

# Full tests (with write access)
BLOG_URL=http://localhost:3004 BLOG_KEY=your-manage-key python test_sdk.py -v
```
