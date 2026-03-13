# Blog Platform - Design Document

## Philosophy

Same as all HNR projects: **tokens tied to resources, not users.** No accounts, no signup. Create a blog → get a manage key. Share the public URL for reading.

## Architecture

- **Backend:** Rust + Rocket 0.5 + SQLite (rusqlite)
- **Frontend:** React + Vite, unified serving on single port
- **Docker:** Multi-stage build (Rust backend + Vite frontend)
- **Port:** 3004

## Data Model

### Blogs
```sql
CREATE TABLE blogs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    manage_key_hash TEXT NOT NULL,
    is_public INTEGER DEFAULT 0,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);
```

### Posts
```sql
CREATE TABLE posts (
    id TEXT PRIMARY KEY,
    blog_id TEXT NOT NULL REFERENCES blogs(id),
    title TEXT NOT NULL,
    slug TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    content_html TEXT NOT NULL DEFAULT '',
    summary TEXT DEFAULT '',
    tags TEXT DEFAULT '[]',
    status TEXT DEFAULT 'draft', -- draft, published
    published_at TEXT,
    author_name TEXT DEFAULT '',
    metadata TEXT DEFAULT '{}',
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    UNIQUE(blog_id, slug)
);
```

### Comments
```sql
CREATE TABLE comments (
    id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL REFERENCES posts(id),
    author_name TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now'))
);
```

## API

### Blog Management
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/blogs | None | Create blog → returns manage_key |
| GET | /api/v1/blogs | None | List public blogs (?limit, ?offset) |
| GET | /api/v1/blogs/:id | None | Get blog details |
| PATCH | /api/v1/blogs/:id | manage_key | Update blog name/description/public |
| POST | /api/v1/blogs/:id/rotate-key | manage_key | Rotate manage key (old key invalidated) |

### Posts
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/blogs/:id/posts | manage_key | Create post |
| GET | /api/v1/blogs/:id/posts | None | List published posts (or all with manage_key) |
| GET | /api/v1/blogs/:id/posts/:slug | None | Get single post by slug |
| PATCH | /api/v1/blogs/:id/posts/:post_id | manage_key | Update post |
| DELETE | /api/v1/blogs/:id/posts/:post_id | manage_key | Delete post |

### Comments
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/blogs/:id/posts/:post_id/comments | None | Add comment |
| GET | /api/v1/blogs/:id/posts/:post_id/comments | None | List comments (?limit, ?offset) |

### Feeds
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | /api/v1/blogs/:id/feed.rss | None | RSS feed |
| GET | /api/v1/blogs/:id/feed.json | None | JSON Feed |
| GET | /api/v1/blogs/:id/feed.atom | None | Atom Feed |

### Discovery
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | /api/v1/health | None | Health check |
| GET | /llms.txt | None | LLM API discovery |

## Auth Model

Same as kanban:
- `POST /blogs` returns `manage_key` (shown once)
- Auth via: `Authorization: Bearer <key>`, `X-API-Key: <key>`, or `?key=<key>`
- Read routes are public (published posts only without auth, all posts with auth)
- Write routes require manage_key

## Markdown Rendering

- Backend converts markdown → HTML on save (stored as `content_html`)
- Raw markdown always preserved in `content`
- Use `pulldown-cmark` for rendering

## Slug Generation

- Auto-generated from title on create (lowercase, hyphens, strip special chars)
- Can be overridden in request
- Must be unique per blog

## RSS/JSON/Atom Feed

- Standard RSS 2.0 XML
- JSON Feed 1.1 format
- Atom 1.0 XML (with categories, summaries, and content)
- Published posts only, ordered by published_at desc
- Limit 50 items

## Rate Limiting

- Blog creation: 5/hr/IP
- Comments: 20/hr/IP

## Cross-Posting Export API

Three export endpoints for published posts, enabling agents to cross-post content to other platforms:

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | /api/v1/blogs/:id/posts/:post_id/export/markdown | None | Frontmatter + raw markdown |
| GET | /api/v1/blogs/:id/posts/:post_id/export/html | None | Standalone dark-themed HTML page |
| GET | /api/v1/blogs/:id/posts/:post_id/export/nostr | None | Unsigned NIP-23 kind 30023 event template |

**Design:**
- Only published posts are exportable (drafts return 404)
- Markdown export includes YAML frontmatter (title, slug, tags, published_at, author)
- HTML export is a self-contained styled page (dark theme, responsive)
- Nostr export returns an unsigned event template with `d`, `title`, `summary`, `published_at`, and `t` tags per NIP-23
- Agents fetch formatted content and handle platform-specific posting (no server-side posting)


## Webhooks

Subscribe to blog events via HTTP POST callbacks. Agent-friendly: register a URL, receive structured payloads.

### Data Model
```sql
CREATE TABLE webhooks (
    id TEXT PRIMARY KEY,
    blog_id TEXT NOT NULL REFERENCES blogs(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    events TEXT NOT NULL DEFAULT '[]',
    secret TEXT DEFAULT NULL,
    is_active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE webhook_deliveries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    webhook_id TEXT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event TEXT NOT NULL,
    status_code INTEGER,
    success INTEGER DEFAULT 0,
    error TEXT DEFAULT NULL,
    delivered_at TEXT DEFAULT (datetime('now'))
);
```

### API
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/blogs/:id/webhooks | manage_key | Register webhook |
| GET | /api/v1/blogs/:id/webhooks | manage_key | List webhooks |
| GET | /api/v1/blogs/:id/webhooks/:wh_id | manage_key | Get webhook |
| DELETE | /api/v1/blogs/:id/webhooks/:wh_id | manage_key | Delete webhook |
| GET | /api/v1/blogs/:id/webhooks/:wh_id/deliveries | manage_key | Delivery history |

### Events
- `post.published` — new post published or draft->published
- `post.updated` — published post updated (stays published)
- `post.deleted` — post deleted
- `comment.created` — new comment added

### Delivery
- HTTP POST to registered URL with JSON payload
- Headers: `Content-Type`, `X-Webhook-Event`, `X-Webhook-Signature` (sha256=HMAC if secret set)
- Payload: `{event, blog_id, timestamp, data}`
- Fire-and-forget (async, 10s timeout, no retries in v1)
- Max 10 webhooks per blog


## Markdown Import

Import posts from standard markdown files with YAML-like frontmatter. Useful for content migration from Jekyll, Hugo, etc. and for agent workflows.

### API

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/blogs/:id/posts/import/markdown | manage_key | Import post from markdown with frontmatter |

### Request

```json
{
  "markdown": "---\ntitle: My Post\ntags: [rust, blog]\nstatus: draft\nsummary: A quick post\nauthor_name: Agent\n---\n# Hello World\n\nThis is the body."
}
```

### Supported Frontmatter Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| title | string | **yes** | Post title |
| slug | string | no | Custom slug (auto-generated from title if omitted) |
| tags | array | no | `[a, b, c]` inline format or comma-separated |
| status | string | no | `draft` (default), `published`, or `scheduled` |
| summary | string | no | Post summary/excerpt |
| author_name | string | no | Author name (also accepts `author` alias) |
| published_at | string | no | ISO-8601 datetime; auto-set if status is `published` |
| scheduled_at | string | no | ISO-8601 datetime; required when status is `scheduled` |

### Response

Returns `{"post": <PostResponse>, "frontmatter_fields": ["title", "tags", ...]}` with status 201.

### Errors

- 401: Missing or invalid manage key
- 409: Slug conflict (post with same slug exists)
- 422: Invalid frontmatter, missing title, or validation error

## Key Product Decisions

- **Pastebin model** — create blog → get management URL
- **Markdown-first** — content is markdown, rendered server-side
- **Agent-friendly** — full API, no CAPTCHA, reasonable rate limits
- **RSS built-in** — every blog gets a feed automatically
- **Comments are open** — no auth required (rate-limited)
- **Drafts** — posts start as draft, publish explicitly

## Post Scheduling

Schedule posts to auto-publish at a specified time. Agent-friendly: set it and forget it.

### How It Works

- Create or update a post with `status: "scheduled"` and `scheduled_at: "<ISO-8601 datetime>"`
- Scheduled posts behave like drafts: hidden from public listings, feeds, and search
- A background task checks every 60 seconds for due posts and publishes them
- When published, `published_at` is set to `scheduled_at` (preserving the intended publish time)
- Webhooks fire with `post.published` event (includes `"scheduled": true` in payload)

### API

Include in create/update post requests:
```json
{
  "title": "My Scheduled Post",
  "content": "...",
  "status": "scheduled",
  "scheduled_at": "2026-03-15T09:00:00+00:00"
}
```

Manual trigger (admin):
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/scheduler/publish | None | Publish all due scheduled posts |

### Validation

- `status: "scheduled"` requires `scheduled_at` field
- `scheduled_at` must be valid ISO-8601 datetime
- Changing status away from "scheduled" clears `scheduled_at`
- Keeping status as "scheduled" preserves existing `scheduled_at` if not explicitly changed


## Post Reactions

Anonymous emoji reactions on published posts. Agent-friendly: lightweight engagement metrics without auth.

### Data Model
```sql
CREATE TABLE post_reactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    post_id TEXT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    client_ip TEXT DEFAULT '',
    created_at TEXT DEFAULT (datetime('now'))
);
```

### API
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | /api/v1/blogs/:id/posts/:post_id/react | None | Add emoji reaction |
| GET | /api/v1/blogs/:id/posts/:post_id/reactions | None | Get reaction counts |

### Allowed Emojis
👍 👎 ❤️ 🔥 🎉 🤔 👀 🚀 💡 👏

### Constraints
- Only published posts can receive reactions
- One reaction per emoji per IP (duplicates return 409 Conflict)
- Different emojis from the same IP are allowed
- Rate limit: 20 reactions/hr per IP per post (configurable via `REACTION_RATE_LIMIT`)
- Invalid emojis return 422
- `reaction_count` field included in all PostResponse payloads
- Reactions cascade-delete when the parent post is deleted
- Webhook event: `post.reacted` fires with `{post_id, emoji}`

### Response Format
```json
{
  "post_id": "...",
  "total": 5,
  "reactions": [
    {"emoji": "👍", "count": 3},
    {"emoji": "🔥", "count": 2}
  ]
}
```
