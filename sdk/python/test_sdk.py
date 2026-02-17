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

    def test_health_version(self):
        h = self.b.health()
        self.assertIn("version", h)

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

    def test_blog_has_description(self):
        blog = self.b.get_blog(self.blog_id)
        self.assertIn("description", blog)


# =========================================================================
# Blog creation (write)
# =========================================================================

class TestBlogCreate(WriteTestCase):
    def test_create_blog_returns_id(self):
        blog = self.b.create_blog(f"SDK-Create-{ts()}", description="test", manage_key=MANAGE_KEY)
        self.assertIn("id", blog)
        self.assertIn("name", blog)

    def test_create_blog_returns_manage_key(self):
        blog = self.b.create_blog(f"SDK-Key-{ts()}", manage_key=MANAGE_KEY)
        self.assertIn("manage_key", blog)
        self.assertTrue(len(blog["manage_key"]) > 0)

    def test_create_blog_public_default(self):
        blog = self.b.create_blog(f"SDK-Public-{ts()}", manage_key=MANAGE_KEY)
        self.assertTrue(blog.get("is_public", True))


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

    def test_list_with_offset(self):
        all_posts = self.b.list_posts(self.blog_id)
        if len(all_posts) < 2:
            self.skipTest("Need 2+ posts for offset test")
        offset_posts = self.b.list_posts(self.blog_id, offset=1)
        self.assertLess(len(offset_posts), len(all_posts))

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

    def test_post_html_rendering(self):
        """Verify content_html is rendered from markdown."""
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts available")
        post = self.b.get_post(self.blog_id, posts[0]["slug"])
        if post.get("content_html"):
            self.assertIn("<", post["content_html"])


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

    def test_create_with_custom_slug(self):
        slug = f"custom-slug-{ts()}"
        post = self._post(slug=slug)
        self.assertEqual(post["slug"], slug)

    def test_slug_auto_generated(self):
        title = f"Auto Slug Test {ts()}"
        post = self._post(title=title)
        self.assertIn("slug", post)
        self.assertTrue(len(post["slug"]) > 0)

    def test_delete_post(self):
        post = self._post()
        self.b.delete_post(self.blog_id, post["id"])
        self.__class__._post_ids.remove(post["id"])

    def test_markdown_rendered(self):
        post = self._post(content="**bold** text")
        self.assertIn("<strong>", post["content_html"])

    def test_create_multiple_posts(self):
        p1 = self._post(title=f"Multi-A {ts()}")
        p2 = self._post(title=f"Multi-B {ts()}")
        self.assertNotEqual(p1["id"], p2["id"])
        posts = self.b.list_posts(self.blog_id)
        ids = [p["id"] for p in posts]
        self.assertIn(p1["id"], ids)
        self.assertIn(p2["id"], ids)

    def test_list_with_tag_filter(self):
        tag = f"unique-{ts()}"
        self._post(tags=[tag])
        self._post(tags=["other"])
        tagged = self.b.list_posts(self.blog_id, tag=tag)
        for p in tagged:
            self.assertIn(tag, p.get("tags", []))


# =========================================================================
# Comments (write)
# =========================================================================

class TestComments(WriteTestCase):
    def test_create_comment(self):
        post = self._post()
        comment = self.b.create_comment(self.blog_id, post["id"], "Tester", "Great post!")
        self.assertIn("id", comment)
        self.assertEqual(comment["author_name"], "Tester")

    def test_list_comments(self):
        post = self._post()
        self.b.create_comment(self.blog_id, post["id"], "Alice", "Comment 1")
        self.b.create_comment(self.blog_id, post["id"], "Bob", "Comment 2")
        comments = self.b.list_comments(self.blog_id, post["id"])
        self.assertGreaterEqual(len(comments), 2)

    def test_delete_comment(self):
        post = self._post()
        comment = self.b.create_comment(self.blog_id, post["id"], "Temp", "Will be deleted")
        result = self.b.delete_comment(self.blog_id, post["id"], comment["id"])
        # After deletion, should not appear in list
        comments = self.b.list_comments(self.blog_id, post["id"])
        ids = [c["id"] for c in comments]
        self.assertNotIn(comment["id"], ids)

    def test_comment_fields(self):
        post = self._post()
        comment = self.b.create_comment(self.blog_id, post["id"], "Fieldcheck", "Testing fields")
        for field in ["id", "author_name", "content", "created_at"]:
            self.assertIn(field, comment, f"Missing comment field: {field}")

    def test_comments_empty(self):
        post = self._post()
        comments = self.b.list_comments(self.blog_id, post["id"])
        self.assertIsInstance(comments, list)
        self.assertEqual(len(comments), 0)


# =========================================================================
# Pinning (write)
# =========================================================================

class TestPinning(WriteTestCase):
    def test_pin_post(self):
        post = self._post()
        result = self.b.pin_post(self.blog_id, post["id"])
        self.assertIsNotNone(result)

    def test_unpin_post(self):
        post = self._post()
        self.b.pin_post(self.blog_id, post["id"])
        result = self.b.unpin_post(self.blog_id, post["id"])
        self.assertIsNotNone(result)

    def test_pin_shows_in_post(self):
        post = self._post()
        self.b.pin_post(self.blog_id, post["id"])
        fetched = self.b.get_post(self.blog_id, post["slug"])
        # Pinned posts should have a pinned_at or is_pinned field
        has_pin_indicator = (
            fetched.get("pinned_at") is not None
            or fetched.get("is_pinned") is True
        )
        self.assertTrue(has_pin_indicator, f"Post should be pinned: {fetched}")


# =========================================================================
# Feeds (read-only)
# =========================================================================

class TestFeeds(ReadOnlyTestCase):
    def test_rss(self):
        rss = self.b.feed_rss(self.blog_id)
        self.assertIn(b"<rss", rss)

    def test_rss_xml_structure(self):
        rss = self.b.feed_rss(self.blog_id)
        self.assertIn(b"<channel>", rss)

    def test_json_feed(self):
        feed = self.b.feed_json(self.blog_id)
        self.assertIn("items", feed)

    def test_json_feed_structure(self):
        feed = self.b.feed_json(self.blog_id)
        self.assertIn("version", feed)
        self.assertIn("title", feed)

    def test_rss_nonexistent_blog(self):
        with self.assertRaises(NotFoundError):
            self.b.feed_rss("nonexistent-blog-id")

    def test_json_feed_nonexistent_blog(self):
        with self.assertRaises(NotFoundError):
            self.b.feed_json("nonexistent-blog-id")


# =========================================================================
# Search (read-only)
# =========================================================================

class TestSearch(ReadOnlyTestCase):
    def test_search(self):
        results = self.b.search("agent")
        # Search returns a list of results (may be empty on fresh server)
        self.assertIsInstance(results, list)

    def test_search_no_results(self):
        results = self.b.search("zzzznonexistent99999")
        self.assertIsInstance(results, list)
        self.assertEqual(len(results), 0)

    def test_search_pagination(self):
        results = self.b.search("the", limit=1)
        self.assertLessEqual(len(results), 1)

    def test_search_with_offset(self):
        all_results = self.b.search("test", limit=10)
        if len(all_results) < 2:
            self.skipTest("Need 2+ search results for offset test")
        offset_results = self.b.search("test", limit=10, offset=1)
        self.assertLess(len(offset_results), len(all_results))


# =========================================================================
# Stats (read-only)
# =========================================================================

class TestStats(ReadOnlyTestCase):
    def test_blog_stats(self):
        stats = self.b.blog_stats(self.blog_id)
        self.assertIsInstance(stats, dict)

    def test_stats_nonexistent(self):
        with self.assertRaises(NotFoundError):
            self.b.blog_stats("nonexistent-blog-id")


# =========================================================================
# Preview
# =========================================================================

class TestPreview(ReadOnlyTestCase):
    def test_preview(self):
        result = self.b.preview("**bold** text")
        self.assertIn("html", result)
        self.assertIn("<strong>", result["html"])

    def test_preview_heading(self):
        result = self.b.preview("# Title")
        self.assertIn("<h1>", result["html"])

    def test_preview_list(self):
        result = self.b.preview("- item 1\n- item 2")
        self.assertIn("<li>", result["html"])

    def test_preview_code_block(self):
        result = self.b.preview("```python\nprint('hello')\n```")
        self.assertIn("<code", result["html"])

    def test_preview_link(self):
        result = self.b.preview("[click](https://example.com)")
        self.assertIn("href", result["html"])


# =========================================================================
# Export (read-only — uses existing posts)
# =========================================================================

class TestExport(ReadOnlyTestCase):
    def _get_slug(self):
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts")
        return posts[0]["slug"]

    def test_export_markdown(self):
        slug = self._get_slug()
        result = self.b.export_markdown(self.blog_id, slug)
        if isinstance(result, dict):
            self.assertIn("content", result)
        else:
            self.assertTrue(len(result) > 0)

    def test_export_html(self):
        slug = self._get_slug()
        html = self.b.export_html(self.blog_id, slug)
        self.assertIn(b"<", html)

    def test_export_html_structure(self):
        slug = self._get_slug()
        html = self.b.export_html(self.blog_id, slug)
        # Should be a complete HTML document
        self.assertTrue(b"<html" in html or b"<body" in html or b"<div" in html)

    def test_export_nostr(self):
        slug = self._get_slug()
        event = self.b.export_nostr(self.blog_id, slug)
        self.assertIn("kind", event)
        self.assertEqual(event["kind"], 30023)

    def test_export_nostr_fields(self):
        slug = self._get_slug()
        event = self.b.export_nostr(self.blog_id, slug)
        self.assertIn("content", event)
        self.assertIn("tags", event)

    def test_export_nonexistent(self):
        with self.assertRaises(NotFoundError):
            self.b.export_html(self.blog_id, "nonexistent-slug-99999")


# =========================================================================
# Discovery (read-only)
# =========================================================================

class TestDiscovery(ReadOnlyTestCase):
    def test_openapi(self):
        spec = self.b.openapi()
        self.assertIn("openapi", spec)

    def test_openapi_paths(self):
        spec = self.b.openapi()
        self.assertIn("paths", spec)
        self.assertGreater(len(spec["paths"]), 10)

    def test_skills_index(self):
        idx = self.b.skills()
        self.assertIn("skills", idx)

    def test_skill_md(self):
        md = self.b.skill_md()
        self.assertIn("blog", md.lower())

    def test_skill_md_has_frontmatter(self):
        md = self.b.skill_md()
        self.assertTrue(md.startswith("---"), "SKILL.md should have YAML frontmatter")


# =========================================================================
# Related Posts (read-only)
# =========================================================================

class TestRelated(ReadOnlyTestCase):
    def test_related_posts(self):
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts available")
        related = self.b.related_posts(self.blog_id, posts[0]["id"])
        self.assertIsInstance(related, list)

    def test_related_with_limit(self):
        posts = self.b.list_posts(self.blog_id, limit=1)
        if not posts:
            self.skipTest("No posts available")
        related = self.b.related_posts(self.blog_id, posts[0]["id"], limit=1)
        self.assertLessEqual(len(related), 1)


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

    def test_manage_key_env(self):
        orig = os.environ.get("BLOG_KEY")
        try:
            os.environ["BLOG_KEY"] = "test-key-123"
            c = Blog()
            self.assertEqual(c.manage_key, "test-key-123")
        finally:
            if orig:
                os.environ["BLOG_KEY"] = orig
            else:
                os.environ.pop("BLOG_KEY", None)

    def test_default_timeout(self):
        c = Blog(timeout=5)
        self.assertEqual(c.timeout, 5)


# =========================================================================
# Write Edge Cases
# =========================================================================

class TestWriteEdgeCases(WriteTestCase):
    def test_long_content(self):
        content = "x" * 5000
        post = self._post(content=content)
        self.assertEqual(len(post["content"]), 5000)

    def test_unicode_content(self):
        post = self._post(title=f"Ünïcödé {ts()}", content="日本語テスト 🎉")
        self.assertIn("🎉", post["content"])

    def test_empty_tags(self):
        post = self._post(tags=[])
        self.assertIsInstance(post.get("tags", []), list)

    def test_multiple_tags(self):
        tags = ["tag1", "tag2", "tag3"]
        post = self._post(tags=tags)
        self.assertEqual(sorted(post.get("tags", [])), sorted(tags))


if __name__ == "__main__":
    print(f"\n📝 Blog Python SDK Tests")
    print(f"   Server: {BASE_URL}")
    print(f"   Write tests: {'ENABLED (BLOG_KEY set)' if MANAGE_KEY else 'SKIPPED (set BLOG_KEY to enable)'}\n")
    unittest.main(verbosity=2)
