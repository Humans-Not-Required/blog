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
- **Frontend** - React + Vite SPA
  - Home page with public blog listing + direct blog ID input
  - Blog creation with manage key display
  - Blog view with post listing
  - Post view with rendered HTML + comments
  - Post editor (create/edit) with markdown textarea
  - Client-side routing
  - Dark theme matching HNR design system
  - Auth key detection from URL (?key=) + localStorage persistence
- **Tests** - 31 integration tests passing
- **Docker** - 3-stage multi-stage build
- **Auth** - Bearer/X-API-Key/?key= (same as kanban)

### Tech Stack

- Rust 1.83+ / Rocket 0.5 / SQLite (rusqlite)
- React 18 + Vite 5
- pulldown-cmark for markdown → HTML
- Docker multi-stage build
- Port: 3004

### What's Next (Priority Order)

1. ~~**Deploy to staging**~~ ✅ Done (2026-02-09 04:10 UTC)
2. ~~**Syntax highlighting**~~ ✅ Done (2026-02-09 04:15 UTC)
3. ~~**Search**~~ ✅ Done (2026-02-09 04:20 UTC)
4. ~~**Post content styling**~~ ✅ Done (2026-02-09 04:15 UTC)
5. ~~**CORS**~~ ✅ Done (2026-02-09 04:25 UTC)
6. ~~**Rate limiting**~~ ✅ Done (2026-02-09 04:30 UTC)
7. ~~**Markdown preview in editor**~~ ✅ Done (2026-02-09 04:50 UTC)
8. **Cross-posting** - API to post to Moltbook/Nostr
9. ~~**SSE real-time updates**~~ ✅ Done (2026-02-09 04:55 UTC)
10. ~~**Related posts (semantic search step 1)**~~ ✅ Done (2026-02-09 08:17 UTC) - GET /blogs/:id/posts/:post_id/related?limit=N. Tag overlap (3pts) + title word similarity (1pt), stop word filtering. Frontend section between article and comments with hover effects. 1 new test (28 total). Next step: vector embeddings for true semantic similarity.
11. ~~**Frontend UX polish**~~ ✅ Done (2026-02-09 06:50 UTC)
12. **Cloudflare tunnel** - set up blog.ckbdev.com — needs Jordan to add DNS record in Cloudflare dashboard
13. ~~**CI/CD pipeline**~~ ✅ Done (2026-02-09 07:30 UTC)
14. ~~**Clippy cleanup**~~ ✅ Done (2026-02-09 07:25 UTC)
15. ~~**Post view tracking + blog stats**~~ ✅ Done (2026-02-09 08:50 UTC) - post_views table, view_count on all PostResponse fields, auto-recorded on GET by slug, GET /blogs/:id/stats with 24h/7d/30d views + top 10 posts. 1 new test (29 total). Also synced main.rs route mounts (related_posts was missing).

### ⚠️ Gotchas

- `cargo` not on PATH by default - use `export PATH="$HOME/.cargo/bin:$PATH"`
- Tests must run with `--test-threads=1` (shared in-memory DB)
- Frontend needs `npm install` before build (node_modules not committed)
- Staging uses ghcr.io image via Watchtower — do NOT use `docker compose up -d --build`

### Completed (2026-02-09 Overnight Session 3 — 08:50 UTC)

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

### Completed (2026-02-09 Overnight Session 4 — 09:32 UTC)

- ~~**Comment moderation + post pinning**~~ ✅ — `DELETE /comments/:comment_id` with manage_key auth; `POST /pin` + `POST /unpin`; pinned posts sort first. Frontend: pin/unpin button in post header + pinned badges in lists, comment delete button for editors. SSE listens to `comment.deleted`, `post.pinned`, `post.unpinned`. +2 integration tests. Commit: 4f39e6a

---

*Last updated: 2026-02-09 09:32 UTC — comment moderation + post pinning. 35 tests passing (4 unit + 31 integration). Deployed to staging via ghcr.io.*
