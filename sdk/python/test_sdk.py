#!/usr/bin/env python3
"""
Integration tests for the Blog Python SDK.

Usage:
    python test_sdk.py
    BLOG_URL=http://192.168.0.79:3004 python test_sdk.py -v
    BLOG_URL=http://192.168.0.79:3004 BLOG_KEY=<manage_key> python test_sdk.py -v

Write tests require a valid BLOG_KEY env var matching a blog's manage key.
Read tests work against any staging instance.
"""

import json
import os
import sys
import time
import unittest

sys.path.insert(0, os.path.dirname(__file__))
from blog import (
    AuthError,
    Blog,
    BlogError,
    NotFoundError,
    RateLimitError,
    ServerError,
    ValidationError,
)

BASE_URL = os.environ.get("BLOG_URL", "http://localhost:3004")
MANAGE_KEY = os.environ.get("BLOG_KEY", "")


def ts() -> str:
    return str(int(time.time() * 1000) % 1_000_000)


def needs_write(test_func):
    """Decorator to skip tests that need write access (BLOG_KEY must be set and valid)."""
    def wrapper(self, *args, **kwargs):
        if not MANAGE_KEY:
            self.skipTest("BLOG_KEY not set — skipping write test")
        return test_func(self, *args, **kwargs)
    wrapper.__name__ = test_func.__name__
    wrapper.__doc__ = test_func.__doc__
    return wrapper


class ReadOnlyTestCase(unittest.TestCase):
    """Base for read-only tests — uses existing blogs on staging."""

    b: Blog
    blog_id: str  # first available blog

    @classmethod
    def setUpClass(cls) -> None:
        cls.b = Blog(BASE_URL, manage_key=MANAGE_KEY or None)
        blogs = cls.b.list_blogs()
        if not blogs:
            raise unittest.SkipTest("No blogs on staging")
        cls.blog_id = blogs[-1]["id"]  # use the oldest (likely HNR blog)


class WriteTestCase(unittest.TestCase):
    """Base for write tests — creates a fresh blog. Requires BLOG_KEY."""

    b: Blog
    blog_id: str
    _post_ids: list

    @classmethod
    def setUpClass(cls) -> None:
        if not MANAGE_KEY:
            raise unittest.SkipTest("BLOG_KEY not set — write tests skipped")
        cls.b = Blog(BASE_URL, manage_key=MANAGE_KEY)
        cls._post_ids = []
        try:
            blog = cls.b.create_blog(f"SDK-Test-{ts()}", description="Integration tests")
            cls.blog_id = blog["id"]
            # Use the new blog's manage_key (each blog has its own key)
            if "manage_key" in blog:
                cls.b = Blog(BASE_URL, manage_key=blog["manage_key"])
        except (RateLimitError, AuthError) as e:
            raise unittest.SkipTest(f"Cannot create test blog: {e}")

    @classmethod
    def tearDownClass(cls) -> None:
        for pid in getattr(cls, "_post_ids", []):
            try:
                cls.b.delete_post(cls.blog_id, pid)
            except Exception:
                pass

    def _post(self, **overrides) -> dict:
        defaults = {
            "title": f"Test Post {ts()}",
            "content": "# Hello\n\nTest content.",
        }
        defaults.update(overrides)
        result = self.b.create_post(self.blog_id, **defaults)
        self.__class__._post_ids.append(result["id"])
        return result


# =========================================================================
# Health (read-only)
# =========================================================================

class TestHealth(ReadOnlyTestCase):
    def test_health(self):
        h = self.b.health()
        self.assertEqual(h["status"], "ok")

    def test_is_healthy(self):
        self.assertTrue(self.b.is_healthy())

    def test_unhealthy(self):
        self.assertFalse(Blog("http://localhost:1").is_healthy())


# =========================================================================
# Blogs (read-only)
# =========================================================================

class TestBlogs(ReadOnlyTestCase):
    def test_list_blogs(self):
        blogs = self.b.list_blogs()
        self.assertIsInstance(blogs, list)
        self.assertGreaterEqual(len(blogs), 1)

    def test_get_blog(self):
        blog = self.b.get_blog(self.blog_id)
        self.assertEqual(blog["id"], self.blog_id)
        self.assertIn("name", blog)

    def test_get_nonexistent(self):
        with self.assertRaises(NotFoundError):
            self.b.get_blog("nonexistent-id")

    def test_blog_fields(self):
        blog = self.b.get_blog(self.blog_id)
        for field in ["id", "name", "is_public", "created_at"]:
            self.assertIn(field, blog)


# =========================================================================
# Posts (read-only — uses existing posts on staging)
# =========================================================================

class TestPostsRead(ReadOnlyTestCase):
    def test_list_posts(self):
        posts = self.b.list_posts(self.blog_id)
        self.assertIsInstance(posts, list)

    def test_list_with_limit(self):
        posts = self.b.list_posts(self.blog_id, limit=1)
        self.assertLessEqual(len(posts), 1)

    def test_get_nonexistent_post(self):
        with self.assertRaises(NotFoundError):
            self.b.get_post(self.blog_id, "nonexistent-slug-99999")

    def test_get_post_by_slug(self):
        """Get a post by slug if any exist."""
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts available")
        slug = posts[0]["slug"]
        fetched = self.b.get_post(self.blog_id, slug)
        self.assertEqual(fetched["slug"], slug)

    def test_post_fields(self):
        """Verify key fields on a post."""
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts available")
        post = self.b.get_post(self.blog_id, posts[0]["slug"])
        for field in ["id", "title", "slug", "content", "content_html", "status", "created_at"]:
            self.assertIn(field, post, f"Missing: {field}")


# =========================================================================
# Posts (write — requires BLOG_KEY)
# =========================================================================

class TestPostsWrite(WriteTestCase):
    def test_create_post(self):
        post = self._post()
        self.assertIn("id", post)
        self.assertIn("title", post)
        self.assertIn("slug", post)

    def test_create_with_tags(self):
        post = self._post(tags=["python", "sdk"])
        self.assertIn("tags", post)

    def test_create_draft(self):
        post = self._post(status="draft")
        self.assertEqual(post["status"], "draft")

    def test_delete_post(self):
        post = self._post()
        self.b.delete_post(self.blog_id, post["id"])
        self.__class__._post_ids.remove(post["id"])

    def test_markdown_rendered(self):
        post = self._post(content="**bold** text")
        self.assertIn("<strong>", post["content_html"])


# =========================================================================
# Feeds (read-only)
# =========================================================================

class TestFeeds(ReadOnlyTestCase):
    def test_rss(self):
        rss = self.b.feed_rss(self.blog_id)
        self.assertIn(b"<rss", rss)

    def test_json_feed(self):
        feed = self.b.feed_json(self.blog_id)
        self.assertIn("items", feed)


# =========================================================================
# Search (read-only)
# =========================================================================

class TestSearch(ReadOnlyTestCase):
    def test_search(self):
        results = self.b.search("agent")
        # Search returns a list of results
        self.assertIsInstance(results, list)
        self.assertGreaterEqual(len(results), 1)

    def test_search_no_results(self):
        results = self.b.search("zzzznonexistent99999")
        self.assertIsInstance(results, list)
        self.assertEqual(len(results), 0)

    def test_search_pagination(self):
        results = self.b.search("the", limit=1)
        self.assertLessEqual(len(results), 1)


# =========================================================================
# Stats (read-only)
# =========================================================================

class TestStats(ReadOnlyTestCase):
    def test_blog_stats(self):
        stats = self.b.blog_stats(self.blog_id)
        self.assertIsInstance(stats, dict)


# =========================================================================
# Export (read-only — uses existing posts)
# =========================================================================

class TestExport(ReadOnlyTestCase):
    def test_preview(self):
        result = self.b.preview("**bold** text")
        self.assertIn("html", result)
        self.assertIn("<strong>", result["html"])

    def test_export_markdown(self):
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts")
        result = self.b.export_markdown(self.blog_id, posts[0]["slug"])
        # May return bytes or dict depending on Content-Type
        if isinstance(result, dict):
            self.assertIn("content", result)
        else:
            self.assertTrue(len(result) > 0)

    def test_export_html(self):
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts")
        html = self.b.export_html(self.blog_id, posts[0]["slug"])
        self.assertIn(b"<", html)

    def test_export_nostr(self):
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts")
        event = self.b.export_nostr(self.blog_id, posts[0]["slug"])
        self.assertIn("kind", event)
        self.assertEqual(event["kind"], 30023)


# =========================================================================
# Discovery (read-only)
# =========================================================================

class TestDiscovery(ReadOnlyTestCase):
    def test_openapi(self):
        spec = self.b.openapi()
        self.assertIn("openapi", spec)

    def test_skills_index(self):
        idx = self.b.skills()
        self.assertIn("skills", idx)

    def test_skill_md(self):
        md = self.b.skill_md()
        self.assertIn("blog", md.lower())


# =========================================================================
# Exceptions & Edge Cases
# =========================================================================

class TestExceptions(ReadOnlyTestCase):
    def test_hierarchy(self):
        for cls in (NotFoundError, AuthError, ValidationError, RateLimitError, ServerError):
            self.assertTrue(issubclass(cls, BlogError))

    def test_status_code(self):
        try:
            self.b.get_blog("nonexistent")
        except NotFoundError as e:
            self.assertEqual(e.status_code, 404)

    def test_repr(self):
        self.assertIn(BASE_URL, repr(self.b))

    def test_env_config(self):
        orig = os.environ.get("BLOG_URL")
        try:
            os.environ["BLOG_URL"] = "http://custom:9999"
            c = Blog()
            self.assertEqual(c.base_url, "http://custom:9999")
        finally:
            if orig:
                os.environ["BLOG_URL"] = orig
            else:
                os.environ.pop("BLOG_URL", None)


if __name__ == "__main__":
    print(f"\n📝 Blog Python SDK Tests")
    print(f"   Server: {BASE_URL}")
    print(f"   Write tests: {'ENABLED (BLOG_KEY set)' if MANAGE_KEY else 'SKIPPED (set BLOG_KEY to enable)'}\n")
    unittest.main(verbosity=2)
