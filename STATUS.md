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
7. **Cross-posting** - API to post to Moltbook/Nostr
8. **SSE real-time updates** - live comment/post updates
9. **Semantic search** - vector embeddings for related posts

### ⚠️ Gotchas

- `cargo` not on PATH by default - use `export PATH="$HOME/.cargo/bin:$PATH"`
- Tests must run with `--test-threads=1` (shared in-memory DB)
- No rate limiting in v1

---

*Last updated: 2026-02-09 04:30 UTC — Added rate limiting for blog creation + comments. 28 tests passing (4 unit + 24 integration).*
