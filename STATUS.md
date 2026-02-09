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

1. **Deploy to staging** - Docker Compose on 192.168.0.79:3004
2. **Syntax highlighting** - code blocks in rendered markdown
3. **Search** - full-text search across posts
4. **Cross-posting** - API to post to Moltbook/Nostr
5. **Rate limiting** - IP-based for blog creation and comments
6. **SSE real-time updates** - live comment/post updates
7. **Post content styling** - CSS for rendered HTML (headers, code, lists, etc.)
8. **Semantic search** - vector embeddings for related posts

### ⚠️ Gotchas

- `cargo` not on PATH by default - use `export PATH="$HOME/.cargo/bin:$PATH"`
- Tests must run with `--test-threads=1` (shared in-memory DB)
- CORS not configured yet (same-origin only)
- No rate limiting in v1

---

*Last updated: 2026-02-09 03:15 UTC — MVP complete: backend + frontend + tests + Docker. 21 tests passing.*
