#!/usr/bin/env python3
"""
blog — Python SDK for HNR Blog Service

Zero-dependency client library for the Blog API.
Works with Python 3.8+ using only the standard library.

Quick start:
    from blog import Blog

    b = Blog("http://localhost:3004")

    # Create a blog
    blog = b.create_blog("My Blog", manage_key="secret")
    blog_id = blog["id"]

    # Create a post
    post = b.create_post(blog_id, "Hello World", "# Welcome\\n\\nFirst post!", manage_key="secret")

    # List posts
    posts = b.list_posts(blog_id)

    # Search
    results = b.search("hello")

    # Get RSS feed
    rss = b.feed_rss(blog_id)

Full docs: GET /api/v1/llms.txt or /.well-known/skills/blog/SKILL.md
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.parse
import urllib.request
from typing import Any, Dict, List, Optional


__version__ = "1.0.0"


# ---------------------------------------------------------------------------
# Exceptions
# ---------------------------------------------------------------------------


class BlogError(Exception):
    """Base exception for Blog API errors."""

    def __init__(self, message: str, status_code: int = 0, body: Any = None):
        super().__init__(message)
        self.status_code = status_code
        self.body = body


class NotFoundError(BlogError):
    pass


class AuthError(BlogError):
    pass


class ValidationError(BlogError):
    pass


class RateLimitError(BlogError):
    pass


class ServerError(BlogError):
    pass


# ---------------------------------------------------------------------------
# Client
# ---------------------------------------------------------------------------


class Blog:
    """Client for the HNR Blog API.

    Args:
        base_url: Service URL (default: ``$BLOG_URL`` or ``http://localhost:3004``).
        manage_key: Default manage key for authenticated operations.
        timeout: HTTP timeout in seconds.
    """

    def __init__(
        self,
        base_url: Optional[str] = None,
        *,
        manage_key: Optional[str] = None,
        timeout: int = 30,
    ):
        self.base_url = (
            base_url or os.environ.get("BLOG_URL") or "http://localhost:3004"
        ).rstrip("/")
        self.manage_key = manage_key or os.environ.get("BLOG_KEY")
        self.timeout = timeout

    # ------------------------------------------------------------------
    # Internal
    # ------------------------------------------------------------------

    def _request(
        self,
        method: str,
        path: str,
        *,
        json_body: Any = None,
        headers: Optional[Dict[str, str]] = None,
        query: Optional[Dict[str, Any]] = None,
        manage_key: Optional[str] = None,
    ) -> Any:
        url = f"{self.base_url}{path}"
        if query:
            filtered = {k: str(v) for k, v in query.items() if v is not None}
            if filtered:
                url += "?" + urllib.parse.urlencode(filtered)

        hdrs = dict(headers or {})
        body: Optional[bytes] = None

        if json_body is not None:
            body = json.dumps(json_body).encode()
            hdrs.setdefault("Content-Type", "application/json")

        key = manage_key or self.manage_key
        if key:
            hdrs["Authorization"] = f"Bearer {key}"

        req = urllib.request.Request(url, data=body, headers=hdrs, method=method)

        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                ct = resp.headers.get("Content-Type", "")
                raw = resp.read()
                if "json" in ct:
                    return json.loads(raw)
                return raw
        except urllib.error.HTTPError as exc:
            self._raise(exc)

    def _raise(self, exc: urllib.error.HTTPError) -> None:
        status = exc.code
        try:
            body = json.loads(exc.read())
        except Exception:
            body = None

        msg = ""
        if isinstance(body, dict):
            msg = body.get("error", "") or body.get("message", "")
        if not msg:
            msg = f"HTTP {status}"

        if status == 404:
            raise NotFoundError(msg, status, body)
        if status in (401, 403):
            raise AuthError(msg, status, body)
        if status == 429:
            raise RateLimitError(msg, status, body)
        if status in (400, 422):
            raise ValidationError(msg, status, body)
        if status >= 500:
            raise ServerError(msg, status, body)
        raise BlogError(msg, status, body)

    # ------------------------------------------------------------------
    # Health
    # ------------------------------------------------------------------

    def health(self) -> Dict[str, Any]:
        """``GET /api/v1/health``"""
        return self._request("GET", "/api/v1/health")

    def is_healthy(self) -> bool:
        try:
            h = self.health()
            return h.get("status") == "ok"
        except Exception:
            return False

    # ------------------------------------------------------------------
    # Blogs
    # ------------------------------------------------------------------

    def create_blog(
        self,
        name: str,
        *,
        description: Optional[str] = None,
        is_public: bool = True,
        manage_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """``POST /api/v1/blogs`` — create a new blog.

        Auth required. Returns the blog object with ``manage_key_hash``.
        """
        body: Dict[str, Any] = {"name": name, "is_public": is_public}
        if description is not None:
            body["description"] = description
        return self._request("POST", "/api/v1/blogs", json_body=body, manage_key=manage_key)

    def list_blogs(self) -> List[Dict[str, Any]]:
        """``GET /api/v1/blogs`` — list all public blogs."""
        return self._request("GET", "/api/v1/blogs")

    def get_blog(self, blog_id: str) -> Dict[str, Any]:
        """``GET /api/v1/blogs/{id}`` — get a blog by ID."""
        return self._request("GET", f"/api/v1/blogs/{blog_id}")

    # ------------------------------------------------------------------
    # Posts
    # ------------------------------------------------------------------

    def create_post(
        self,
        blog_id: str,
        title: str,
        content: str,
        *,
        slug: Optional[str] = None,
        tags: Optional[List[str]] = None,
        status: str = "published",
        manage_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """``POST /api/v1/blogs/{id}/posts`` — create a post.

        Auth required.
        """
        body: Dict[str, Any] = {
            "title": title,
            "content": content,
            "status": status,
        }
        if slug is not None:
            body["slug"] = slug
        if tags is not None:
            body["tags"] = tags
        return self._request(
            "POST", f"/api/v1/blogs/{blog_id}/posts",
            json_body=body, manage_key=manage_key,
        )

    def list_posts(
        self,
        blog_id: str,
        *,
        tag: Optional[str] = None,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
    ) -> List[Dict[str, Any]]:
        """``GET /api/v1/blogs/{id}/posts`` — list posts."""
        return self._request("GET", f"/api/v1/blogs/{blog_id}/posts", query={
            "tag": tag,
            "limit": limit,
            "offset": offset,
        })

    def get_post(self, blog_id: str, slug: str) -> Dict[str, Any]:
        """``GET /api/v1/blogs/{id}/posts/{slug}`` — get post by slug."""
        return self._request("GET", f"/api/v1/blogs/{blog_id}/posts/{slug}")

    def delete_post(
        self,
        blog_id: str,
        post_id: str,
        *,
        manage_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """``DELETE /api/v1/blogs/{id}/posts/{post_id}`` — delete a post. Auth required."""
        return self._request(
            "DELETE", f"/api/v1/blogs/{blog_id}/posts/{post_id}",
            manage_key=manage_key,
        )

    # ------------------------------------------------------------------
    # Comments
    # ------------------------------------------------------------------

    def create_comment(
        self,
        blog_id: str,
        post_id: str,
        author: str,
        content: str,
        *,
        manage_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """``POST /api/v1/blogs/{id}/posts/{post_id}/comments``"""
        return self._request(
            "POST", f"/api/v1/blogs/{blog_id}/posts/{post_id}/comments",
            json_body={"author_name": author, "content": content},
            manage_key=manage_key,
        )

    def list_comments(self, blog_id: str, post_id: str) -> List[Dict[str, Any]]:
        """``GET /api/v1/blogs/{id}/posts/{post_id}/comments``"""
        return self._request("GET", f"/api/v1/blogs/{blog_id}/posts/{post_id}/comments")

    def delete_comment(
        self,
        blog_id: str,
        post_id: str,
        comment_id: str,
        *,
        manage_key: Optional[str] = None,
    ) -> Dict[str, Any]:
        """``DELETE /api/v1/blogs/{id}/posts/{post_id}/comments/{comment_id}``"""
        return self._request(
            "DELETE", f"/api/v1/blogs/{blog_id}/posts/{post_id}/comments/{comment_id}",
            manage_key=manage_key,
        )

    # ------------------------------------------------------------------
    # Pin/Unpin
    # ------------------------------------------------------------------

    def pin_post(self, blog_id: str, post_id: str, *, manage_key: Optional[str] = None) -> Dict[str, Any]:
        """``POST /api/v1/blogs/{id}/posts/{post_id}/pin``"""
        return self._request("POST", f"/api/v1/blogs/{blog_id}/posts/{post_id}/pin", json_body={}, manage_key=manage_key)

    def unpin_post(self, blog_id: str, post_id: str, *, manage_key: Optional[str] = None) -> Dict[str, Any]:
        """``POST /api/v1/blogs/{id}/posts/{post_id}/unpin``"""
        return self._request("POST", f"/api/v1/blogs/{blog_id}/posts/{post_id}/unpin", json_body={}, manage_key=manage_key)

    # ------------------------------------------------------------------
    # Feeds
    # ------------------------------------------------------------------

    def feed_rss(self, blog_id: str) -> bytes:
        """``GET /api/v1/blogs/{id}/feed.rss`` — RSS 2.0 feed."""
        result = self._request("GET", f"/api/v1/blogs/{blog_id}/feed.rss")
        return result if isinstance(result, bytes) else result.encode()

    def feed_json(self, blog_id: str) -> Dict[str, Any]:
        """``GET /api/v1/blogs/{id}/feed.json`` — JSON Feed 1.1."""
        return self._request("GET", f"/api/v1/blogs/{blog_id}/feed.json")

    # ------------------------------------------------------------------
    # Search
    # ------------------------------------------------------------------

    def search(
        self,
        query: str,
        *,
        limit: Optional[int] = None,
        offset: Optional[int] = None,
    ) -> Dict[str, Any]:
        """``GET /api/v1/search`` — FTS5 full-text search across all blogs."""
        return self._request("GET", "/api/v1/search", query={
            "q": query,
            "limit": limit,
            "offset": offset,
        })

    def search_semantic(
        self,
        query: str,
        *,
        limit: Optional[int] = None,
        blog_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """``GET /api/v1/search/semantic`` — semantic search (if enabled)."""
        return self._request("GET", "/api/v1/search/semantic", query={
            "q": query,
            "limit": limit,
            "blog_id": blog_id,
        })

    # ------------------------------------------------------------------
    # Related / Stats
    # ------------------------------------------------------------------

    def related_posts(
        self,
        blog_id: str,
        post_id: str,
        *,
        limit: Optional[int] = None,
    ) -> Any:
        """``GET /api/v1/blogs/{id}/posts/{post_id}/related``"""
        return self._request("GET", f"/api/v1/blogs/{blog_id}/posts/{post_id}/related", query={
            "limit": limit,
        })

    def blog_stats(self, blog_id: str) -> Dict[str, Any]:
        """``GET /api/v1/blogs/{id}/stats`` — blog analytics."""
        return self._request("GET", f"/api/v1/blogs/{blog_id}/stats")

    # ------------------------------------------------------------------
    # Preview & Export
    # ------------------------------------------------------------------

    def preview(self, content: str) -> Dict[str, Any]:
        """``POST /api/v1/preview`` — render markdown to HTML."""
        return self._request("POST", "/api/v1/preview", json_body={"content": content})

    def export_markdown(self, blog_id: str, slug: str) -> Any:
        """``GET /api/v1/blogs/{id}/posts/{slug}/export/markdown``

        Returns bytes or dict depending on server response.
        """
        return self._request("GET", f"/api/v1/blogs/{blog_id}/posts/{slug}/export/markdown")

    def export_html(self, blog_id: str, slug: str) -> bytes:
        """``GET /api/v1/blogs/{id}/posts/{slug}/export/html``"""
        result = self._request("GET", f"/api/v1/blogs/{blog_id}/posts/{slug}/export/html")
        return result if isinstance(result, bytes) else result.encode()

    def export_nostr(self, blog_id: str, slug: str) -> Dict[str, Any]:
        """``GET /api/v1/blogs/{id}/posts/{slug}/export/nostr`` — Nostr NIP-23 event."""
        return self._request("GET", f"/api/v1/blogs/{blog_id}/posts/{slug}/export/nostr")

    # ------------------------------------------------------------------
    # Discovery
    # ------------------------------------------------------------------

    def llms_txt(self) -> str:
        data = self._request("GET", "/api/v1/llms.txt")
        return data.decode() if isinstance(data, bytes) else str(data)

    def openapi(self) -> Dict[str, Any]:
        return self._request("GET", "/api/v1/openapi.json")

    def skills(self) -> Dict[str, Any]:
        return self._request("GET", "/.well-known/skills/index.json")

    def skill_md(self) -> str:
        data = self._request("GET", "/.well-known/skills/blog/SKILL.md")
        return data.decode() if isinstance(data, bytes) else str(data)

    def __repr__(self) -> str:
        return f"Blog(base_url={self.base_url!r})"
