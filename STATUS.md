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
- **Tests** - 21 integration tests passing
- **Docker** - 3-stage multi-stage build
- **Auth** - Bearer/X-API-Key/?key= (same as kanban)

### Tech Stack

- Rust 1.83+ / Rocket 0.5 / SQLite (rusqlite)
- React 18 + Vite 5
- pulldown-cmark for markdown → HTML
- Docker multi-stage build
- Port: 3004

### What's Next (Priority Order)

1. ~~**Deploy to staging**~~ ✅ Done (2026-02-09 04:10 UTC) - Docker Compose on 192.168.0.79:3004, fixed Rust version (1.83→1.89 for time crate compat)
2. ~~**Syntax highlighting**~~ ✅ Done (2026-02-09 04:15 UTC) - highlight.js via CDN, auto-highlights code blocks on post render
3. ~~**Search**~~ ✅ Done (2026-02-09 04:20 UTC) - GET /api/v1/search?q= searches title/content/tags/author across all published posts. Frontend search bar on home page.
4. ~~**Post content styling**~~ ✅ Done (2026-02-09 04:15 UTC) - full prose CSS for headers, code, blockquotes, tables, lists, links, images, hr
5. ~~**CORS**~~ ✅ Done (2026-02-09 04:25 UTC) - rocket_cors with all origins allowed
6. ~~**Rate limiting**~~ ✅ Done (2026-02-09 04:30 UTC) - IP-based: blog creation 10/hr (BLOG_RATE_LIMIT env), comments 30/hr (COMMENT_RATE_LIMIT env), ClientIp guard (XFF/X-Real-Ip/socket), 429 JSON catcher, 2 new tests (28 total: 4 unit + 24 integration)
7. ~~**Markdown preview in editor**~~ ✅ Done (2026-02-09 04:50 UTC) - POST /api/v1/preview endpoint, Write/Preview tab switcher in PostEditor, 300ms debounce, syntax highlighting in preview, 1 new test (25 total)
8. **Cross-posting** - API to post to Moltbook/Nostr
9. ~~**SSE real-time updates**~~ ✅ Done (2026-02-09 04:55 UTC) - EventBus with per-blog broadcast channels, GET /blogs/:id/events/stream endpoint (15s heartbeat), events: post.created/updated/deleted + comment.created, frontend auto-refreshes on events (300ms debounce)
10. **Semantic search** - vector embeddings for related posts
11. ~~**Frontend UX polish**~~ ✅ Done (2026-02-09 06:50 UTC) - SVG logo, manage URL display with copy buttons, author name persistence, keyboard shortcuts (Cmd/Ctrl+S, Cmd/Ctrl+Enter, Esc), relative dates, hover cards, sticky header, better empty states, draft/published sections, mobile responsive, blog ID extraction from URLs, SVG favicon
12. **Cloudflare tunnel** - set up blog.ckbdev.com — needs Jordan to add DNS record in Cloudflare dashboard (no cloudflared on staging, kanban uses proxied A record)
13. ~~**CI/CD pipeline**~~ ✅ Done (2026-02-09 07:30 UTC) - GitHub Actions workflow: test (--test-threads=1) + Docker build/push to ghcr.io. Staging switched from local build to ghcr.io/humans-not-required/blog:dev with Watchtower auto-update label.
14. ~~**Clippy cleanup**~~ ✅ Done (2026-02-09 07:25 UTC) - zero warnings: type alias, clamp(), redundant closures, collapsible match, dead_code suppression.

### ⚠️ Gotchas

- `cargo` not on PATH by default - use `export PATH="$HOME/.cargo/bin:$PATH"`
- Tests must run with `--test-threads=1` (shared in-memory DB)
- Frontend needs `npm install` before build (node_modules not committed)
- Staging uses ghcr.io image via Watchtower — do NOT use `docker compose up -d --build`

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

---

*Last updated: 2026-02-09 07:45 UTC — OpenAPI spec + tag filtering + app directory listing. 26 tests passing. Deployed to staging via ghcr.io.*
