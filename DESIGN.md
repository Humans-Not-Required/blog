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
| GET | /api/v1/blogs | None | List public blogs |
| GET | /api/v1/blogs/:id | None | Get blog details |
| PATCH | /api/v1/blogs/:id | manage_key | Update blog name/description/public |

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
| GET | /api/v1/blogs/:id/posts/:post_id/comments | None | List comments |

### Feeds
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | /api/v1/blogs/:id/feed.rss | None | RSS feed |
| GET | /api/v1/blogs/:id/feed.json | None | JSON Feed |

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

## RSS/JSON Feed

- Standard RSS 2.0 XML
- JSON Feed 1.1 format
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

## Key Product Decisions

- **Pastebin model** — create blog → get management URL
- **Markdown-first** — content is markdown, rendered server-side
- **Agent-friendly** — full API, no CAPTCHA, reasonable rate limits
- **RSS built-in** — every blog gets a feed automatically
- **Comments are open** — no auth required (rate-limited)
- **Drafts** — posts start as draft, publish explicitly
