# Blog Platform - Status

## Current State: MVP Complete ✅

API-first blog platform with Rust backend, React frontend, Docker deployment.

### What's Done

- **Backend** - Full REST API with Rocket 0.5 + SQLite
  - Blog CRUD with per-blog manage keys (same auth model as kanban)
  - Post CRUD with markdown rendering (pulldown-cmark)
  - Auto-slug generation from titles
  - Draft/published workflow
  - Comments on published posts
  - Comment moderation: delete comments with manage_key
  - Post pinning: pin/unpin posts, pinned posts sort first
  - RSS 2.0 and JSON Feed 1.1
  - /llms.txt for API discovery
  - /api/v1/openapi.json — full OpenAPI 3.0.3 spec
  - JSON error catchers
  - FTS5 full-text search with BM25 ranking, snippets, and porter stemming
- **Semantic Search** - TF-IDF vectorization with cosine similarity
  - GET /api/v1/search/semantic?q=&limit=&blog_id= endpoint
  - In-memory TF-IDF index rebuilt on startup from published posts
  - Auto-updated on post create/update/delete
  - Simple stemmer + stop word filtering for better term matching
  - find_similar() API ready for related posts enhancement
- **Frontend** - React + Vite SPA
  - Home page with public blog listing + direct blog ID input
  - Blog creation with manage key display
  - Blog view with post listing
  - Post view with rendered HTML + comments
  - Post editor (create/edit) with markdown textarea
  - Client-side routing
  - Dark theme matching HNR design system
  - Auth key detection from URL (?key=) + localStorage persistence
- **Delete blog** - DELETE /api/v1/blogs/{id} cascade deletes all posts, comments, views + FTS/semantic indexes
- **Tests** - 160 Rust tests (136 integration + 12 unit × 2) + 170 Python SDK integration tests (321 total)
- **Python SDK** — Complete zero-dependency client library (`sdk/python/blog.py`) wrapping all API endpoints. 164 integration tests covering health, blogs, posts, comments, pinning, feeds, search, stats, preview, export, discovery, related posts, error handling, blog/post updates, metadata (word count, reading time), view tracking, pinned ordering, deletion cascade, semantic search, auth edge cases, slug behavior, draft behavior, full lifecycle, multi-blog isolation, post summaries, constructor variants. Fixed `create_comment` field name bug (`author` → `author_name`). README with full API reference.
- **Documentation** - Comprehensive README (195 lines), detailed llms.txt (all 27 endpoints)
- **Docker** - 3-stage multi-stage build
- **Auth** - Bearer/X-API-Key/?key= (same as kanban)

### Tech Stack

- Rust 1.83+ / Rocket 0.5 / SQLite (rusqlite)
- React 18 + Vite 5
- pulldown-cmark for markdown → HTML
- Docker multi-stage build
- Port: 3004

### What's Next (Priority Order)

1. ~~**SPA fallback + Open Graph meta tags**~~ ✅ Done (2026-03-11 15:55 UTC) - direct URL access fixed, OG meta injection for social sharing
1. ~~**Fix OG duplicate description + add og:url**~~ ✅ Done (2026-03-11 17:20 UTC) - replaced generic meta description instead of duplicating, added og:url

1. ~~**Deploy to staging**~~ ✅ Done (2026-02-09 04:10 UTC)
2. ~~**Syntax highlighting**~~ ✅ Done (2026-02-09 04:15 UTC)
3. ~~**Search**~~ ✅ Done (2026-02-09 04:20 UTC)
4. ~~**Post content styling**~~ ✅ Done (2026-02-09 04:15 UTC)
5. ~~**CORS**~~ ✅ Done (2026-02-09 04:25 UTC)
6. ~~**Rate limiting**~~ ✅ Done (2026-02-09 04:30 UTC)
7. ~~**Markdown preview in editor**~~ ✅ Done (2026-02-09 04:50 UTC)
8. ~~**Cross-posting export API**~~ ✅ Done (2026-02-09 10:38 UTC) - 3 export endpoints: markdown (frontmatter + raw), HTML (standalone styled page), Nostr NIP-23 (unsigned kind 30023 event template). Agents fetch formatted content and post to their platform.
9. ~~**SSE real-time updates**~~ ✅ Done (2026-02-09 04:55 UTC) → **REMOVED** (2026-02-11 02:55 UTC) per Jordan's direction to simplify codebase
10. ~~**Related posts (semantic search step 1)**~~ ✅ Done (2026-02-09 08:17 UTC) - GET /blogs/:id/posts/:post_id/related?limit=N. Tag overlap (3pts) + title word similarity (1pt), stop word filtering. Frontend section between article and comments with hover effects. 1 new test (28 total). Next step: vector embeddings for true semantic similarity.
11. ~~**Frontend UX polish**~~ ✅ Done (2026-02-09 06:50 UTC)
12. ~~**Semantic search**~~ ✅ Done (2026-02-13 22:35 UTC) - TF-IDF + cosine similarity. New /search/semantic endpoint. In-memory index, auto-rebuilt on startup, updated on mutations. 12 unit tests + 1 integration test.
13. **Cloudflare tunnel** - set up blog.ckbdev.com — needs Jordan to add DNS record in Cloudflare dashboard
   - **Jordan question (2026-02-13):** Viewable URL right now is LAN-only: http://192.168.0.79:3004 (staging). Public URL needs the Cloudflare DNS/tunnel record (blog.ckbdev.com).
13. ~~**CI/CD pipeline**~~ ✅ Done (2026-02-09 07:30 UTC)
14. ~~**Clippy cleanup**~~ ✅ Done (2026-02-09 07:25 UTC)
15. ~~**Post view tracking + blog stats**~~ ✅ Done (2026-02-09 08:50 UTC) - post_views table, view_count on all PostResponse fields, auto-recorded on GET by slug, GET /blogs/:id/stats with 24h/7d/30d views + top 10 posts. 1 new test (29 total). Also synced main.rs route mounts (related_posts was missing).

### ⚠️ Gotchas

- `cargo` not on PATH by default - use `export PATH="$HOME/.cargo/bin:$PATH"`
- Tests must run with `--test-threads=1` (shared in-memory DB)
- Frontend needs `npm install` before build (node_modules not committed)
- Staging uses ghcr.io image via Watchtower — do NOT use `docker compose up -d --build`

### Completed (2026-03-11 SPA Fallback — 15:55 UTC)

- ~~**Post view tracking + blog stats**~~ ✅ — `post_views` table with viewer_ip + indexes. `view_count` on all PostResponse fields (list, get-by-slug, query_post). Views auto-recorded fire-and-forget on get_post_by_slug. `GET /api/v1/blogs/:id/stats` returns total/published posts, comments, total views + 24h/7d/30d breakdowns, top 10 posts by views. Also synced main.rs routes with lib.rs (related_posts + blog_stats were missing from binary). 1 new integration test (29 total: 4 unit + 25 integration). Commit: 1c5bd11

### Completed (2026-02-09 Overnight Session 2 — 08:17 UTC)

- ~~**Related posts API + frontend**~~ ✅ — `GET /api/v1/blogs/:id/posts/:post_id/related?limit=N` scores by shared tags (3pts) + title word overlap (1pt) with stop word filtering. Frontend shows related posts section with hover effects, tag chips, reading time. 1 new integration test (28 total: 4 unit + 24 integration). Commit: cf2e8b4

### Completed (2026-02-09 Daytime Session 3 — 08:00 UTC)

- ~~**Word count and reading time**~~ ✅ — `word_count` and `reading_time_minutes` fields on all post API responses. Computed from markdown content at 200 wpm. Frontend displays reading time in post cards (list view) and word count + reading time in post detail view. OpenAPI spec updated. 1 new integration test (31 total: 4 unit + 27 integration). Commit: d17f0a3

### Completed (2026-02-09 Daytime Session 2 — 07:45 UTC)

- ~~**OpenAPI 3.0 spec**~~ ✅ — GET /api/v1/openapi.json, covers all endpoints, schemas, auth model. 1 new test (26 total).
- ~~**Tag filtering in blog view**~~ ✅ — clickable tag bar at top of posts, click to filter/toggle, active highlight, clear button, result count.
- ~~**App directory listing**~~ ✅ — Blog Platform added to app directory (4 total apps).
- ~~**App directory "empty DB" investigation**~~ ✅ — DB has 4 apps; issue was no Cloudflare tunnel for apps.ckbdev.com. Task moved to Review for Jordan.

### Completed (2026-02-09 Daytime Session 1 — 07:30 UTC)

- ~~**CI/CD pipeline**~~ ✅ — GitHub Actions: test + Docker build/push to ghcr.io
- ~~**Clippy cleanup**~~ ✅ — zero warnings across all source files
- ~~**Staging migration**~~ ✅ — switched from local Docker build to ghcr.io/humans-not-required/blog:dev with Watchtower auto-updates
- **Cloudflare tunnel** — task created for Jordan (needs DNS record in Cloudflare dashboard)

### Completed (2026-02-09 Overnight Session 5 — 09:50 UTC)

- ~~**My Blogs**~~ ✅ — localStorage-based blog tracking on home page. Auto-adds blogs when visited or created. Shows ✏️/👁 icons based on manage key. ✕ remove button. Consistent with kanban My Boards pattern. Commit: 3b7b8f3

### Completed (2026-02-09 Overnight Session 4 — 09:32 UTC)

- ~~**Comment moderation + post pinning**~~ ✅ — `DELETE /comments/:comment_id` with manage_key auth; `POST /pin` + `POST /unpin`; pinned posts sort first. Frontend: pin/unpin button in post header + pinned badges in lists, comment delete button for editors. SSE listens to `comment.deleted`, `post.pinned`, `post.unpinned`. +2 integration tests. Commit: 4f39e6a

---

### Completed (2026-02-09 Overnight Session 6 — 10:38 UTC)

- ~~**Cross-posting export API**~~ ✅ — 3 export endpoints for published posts: `/export/markdown` (frontmatter + raw content), `/export/html` (standalone dark-themed page), `/export/nostr` (unsigned NIP-23 kind 30023 event template with d/title/summary/published_at/t tags). Agents fetch formatted content and handle posting to their platform. Only published posts exportable (drafts return 404). +3 integration tests (34 total: 4 unit + 30 integration). Commit: 6c9ffaa

### Completed (2026-02-09 Overnight Session 7 — 11:52 UTC)

- ~~**Fix Docker build (CI failure)**~~ ✅ — Dockerfile was missing `COPY backend/openapi.json` needed by `include_str!("../openapi.json")` in routes.rs. All previous CI runs were failing on Docker build step. Tests always passed. Commit: 275fd0b

### Completed (2026-02-09 Overnight Session 8 — 12:47 UTC)

- ~~**FTS5 full-text search**~~ ✅ — SQLite FTS5 virtual table with porter stemming tokenizer. Search endpoint now returns BM25-ranked results with highlighted snippets (`<mark>` tags). Graceful fallback to LIKE search for invalid FTS queries. FTS index auto-rebuilt from published posts on startup, kept in sync on create/update/delete. Drafts excluded from index. +1 integration test (35 total: 4 unit + 31 integration). Commit: ddc117f

### Completed (2026-02-09 Overnight Session 9 — 13:36 UTC)

- **HNR blog created + launch post published** ✅ — Created "Humans Not Required" public blog on staging. First post: "We Built Five Products in 48 Hours (No Humans Required)" — 623 words, pinned. Covers all 5 products, tech stack, auth model, and lessons learned. Blog ID: `0416e210-514a-49e0-9b24-16e1763debf0`, manage key: `blog_abb2455d72a74ec79f5b26e8a1e2a67b`.

### Completed (2026-02-11 — Daytime Session)

- ~~**Remove SSE real-time updates**~~ ✅ — Removed EventBus, event stream endpoint (`/blogs/:id/events/stream`), all `bus.emit()` calls from mutation routes, frontend EventSource subscriptions, tokio sync dependency, and SSE references from OpenAPI spec. -208 lines of code. 35 tests pass, zero clippy warnings. Commit: 688660c

### Completed (2026-03-01 — Frontend Polish Session)

- ~~**Aesthetics overhaul**~~ ✅ — Light/dark theme with proper CSS, zero inline styles
- ~~**Component decomposition**~~ ✅ — App.jsx split from 952→66 lines into 8 focused components
- ~~**Dynamic document titles + view counts**~~ ✅ — Title updates per page, view count display, select styling
- ~~**Code copy buttons + heading anchors + TOC**~~ ✅ — Copy-to-clipboard on code blocks, anchor links on headings, auto-generated table of contents
- ~~**Search snippets with highlights**~~ ✅ — FTS results show highlighted snippets
- ~~**Semantic search toggle on home page**~~ ✅ — Users can switch between FTS and semantic search
- ~~**Markdown editor enhancements**~~ ✅ — Toolbar (bold/italic/link/code/etc), tab indent, word count, unsaved changes warning
- Cleaned up stale WIP components, standardized .gitignore

*Last updated: 2026-03-11 17:35 UTC — 160 Rust tests (140 integration + 12 unit × 2) + 170 Python SDK tests (330 total). All CI green. Zero clippy warnings. **Blog is LIVE at https://blog.hnrstage.xyz** — DNS resolved via Cloudflare wildcard tunnel (*.hnrstage.xyz). 16 published posts. OG meta tags fixed (duplicate description bug). Project is feature-complete, polish-complete, and publicly accessible.*

## Incoming directions (2026-02-13T17:49:01Z)
- Jordan: Cloudflare tunnel/DNS task being archived (he’s rolling out a more permanent solution). No action on my side for now. (task 8479e4ca)
- Jordan: Cross-posting export API — verify it’s documented somewhere (DESIGN.md/etc) before archiving/marking done. ✅ Verified: `DESIGN.md` has a dedicated “Cross-Posting Export API” section with the 3 endpoints + constraints. OK to archive. (2026-02-13T18:40:08Z; task_id: 68343bab-4e11-4772-9961-216225d1c841)
- Jordan: SSE real-time updates — question was whether a blog needs real-time streaming. Resolved: SSE was removed on 2026-02-11 per direction; no further action unless it needs a cleanup/archival pass. (2026-02-13T18:40:08Z; task_id: 2f1fc78b-4252-427d-9f5c-e8a92ac16349)
