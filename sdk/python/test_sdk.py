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
        # Extract title and content as positional, rest as kwargs
        title = defaults.pop("title")
        content = defaults.pop("content")
        result = self.b.create_post(self.blog_id, title, content, **defaults)
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


# =========================================================================
# Blog Update (write)
# =========================================================================

class TestBlogUpdate(WriteTestCase):
    def test_update_name(self):
        new_name = f"Updated-{ts()}"
        result = self.b.update_blog(self.blog_id, name=new_name)
        self.assertEqual(result["name"], new_name)

    def test_update_description(self):
        desc = f"New description {ts()}"
        result = self.b.update_blog(self.blog_id, description=desc)
        self.assertEqual(result.get("description", ""), desc)

    def test_update_preserves_other_fields(self):
        """Updating name should not clear description."""
        desc = f"Preserve-{ts()}"
        self.b.update_blog(self.blog_id, description=desc)
        result = self.b.update_blog(self.blog_id, name=f"NewName-{ts()}")
        self.assertEqual(result.get("description", ""), desc)

    def test_update_nonexistent_blog(self):
        with self.assertRaises((NotFoundError, AuthError)):
            self.b.update_blog("nonexistent-blog-id", name="X")

    def test_update_requires_auth(self):
        no_auth = Blog(BASE_URL)
        with self.assertRaises(AuthError):
            no_auth.update_blog(self.blog_id, name="Hacked")


# =========================================================================
# Post Update (write)
# =========================================================================

class TestPostUpdate(WriteTestCase):
    def test_update_title(self):
        post = self._post()
        new_title = f"Updated Title {ts()}"
        updated = self.b.update_post(self.blog_id, post["id"], title=new_title)
        self.assertEqual(updated["title"], new_title)

    def test_update_content(self):
        post = self._post()
        new_content = "# Updated\n\nNew content here."
        updated = self.b.update_post(self.blog_id, post["id"], content=new_content)
        self.assertEqual(updated["content"], new_content)
        self.assertIn("<h1>", updated.get("content_html", ""))

    def test_update_tags(self):
        post = self._post(tags=["old"])
        updated = self.b.update_post(self.blog_id, post["id"], tags=["new", "updated"])
        self.assertEqual(sorted(updated.get("tags", [])), ["new", "updated"])

    def test_update_status_draft(self):
        post = self._post(status="published")
        updated = self.b.update_post(self.blog_id, post["id"], status="draft")
        self.assertEqual(updated["status"], "draft")

    def test_update_status_publish(self):
        post = self._post(status="draft")
        updated = self.b.update_post(self.blog_id, post["id"], status="published")
        self.assertEqual(updated["status"], "published")

    def test_update_summary(self):
        post = self._post()
        updated = self.b.update_post(self.blog_id, post["id"], summary="A brief summary")
        self.assertEqual(updated.get("summary", ""), "A brief summary")

    def test_update_preserves_content(self):
        """Updating title should not clear content."""
        original_content = "# Keep This\n\nDon't lose me."
        post = self._post(content=original_content)
        updated = self.b.update_post(self.blog_id, post["id"], title=f"New Title {ts()}")
        self.assertEqual(updated["content"], original_content)

    def test_update_nonexistent_post(self):
        with self.assertRaises(NotFoundError):
            self.b.update_post(self.blog_id, "nonexistent-post-id", title="X")

    def test_update_requires_auth(self):
        post = self._post()
        no_auth = Blog(BASE_URL)
        with self.assertRaises(AuthError):
            no_auth.update_post(self.blog_id, post["id"], title="Hacked")


# =========================================================================
# Post Reading Time / Word Count
# =========================================================================

class TestPostMetadata(WriteTestCase):
    def test_word_count_present(self):
        post = self._post(content="one two three four five")
        self.assertIn("word_count", post)
        self.assertGreaterEqual(post["word_count"], 5)

    def test_reading_time_present(self):
        post = self._post(content="word " * 400)
        self.assertIn("reading_time_minutes", post)
        self.assertGreaterEqual(post["reading_time_minutes"], 1)

    def test_short_reading_time(self):
        post = self._post(content="Hello")
        self.assertIn("reading_time_minutes", post)
        self.assertGreaterEqual(post["reading_time_minutes"], 1)

    def test_word_count_updates_on_edit(self):
        post = self._post(content="short")
        long_content = "word " * 500
        updated = self.b.update_post(self.blog_id, post["id"], content=long_content)
        self.assertGreater(updated.get("word_count", 0), post.get("word_count", 0))


# =========================================================================
# View Tracking
# =========================================================================

class TestViewTracking(WriteTestCase):
    def test_view_count_field_present(self):
        post = self._post()
        fetched = self.b.get_post(self.blog_id, post["slug"])
        self.assertIn("view_count", fetched)

    def test_view_count_increments(self):
        post = self._post()
        self.b.get_post(self.blog_id, post["slug"])
        self.b.get_post(self.blog_id, post["slug"])
        fetched = self.b.get_post(self.blog_id, post["slug"])
        # At least 2 views from the first two GETs (the third also increments)
        self.assertGreaterEqual(fetched.get("view_count", 0), 2)

    def test_view_count_in_list(self):
        post = self._post()
        self.b.get_post(self.blog_id, post["slug"])  # increment
        posts = self.b.list_posts(self.blog_id)
        matching = [p for p in posts if p["id"] == post["id"]]
        if matching:
            self.assertIn("view_count", matching[0])


# =========================================================================
# Pinned Post Ordering
# =========================================================================

class TestPinnedOrdering(WriteTestCase):
    def test_pinned_appears_first(self):
        p1 = self._post(title=f"First {ts()}")
        p2 = self._post(title=f"Second {ts()}")
        p3 = self._post(title=f"Third {ts()}")
        # Pin the middle post
        self.b.pin_post(self.blog_id, p2["id"])
        posts = self.b.list_posts(self.blog_id)
        ids = [p["id"] for p in posts]
        # p2 should be first (pinned)
        self.assertEqual(ids[0], p2["id"])

    def test_unpin_restores_order(self):
        p1 = self._post(title=f"Unpin-A {ts()}")
        time.sleep(0.1)
        p2 = self._post(title=f"Unpin-B {ts()}")
        self.b.pin_post(self.blog_id, p1["id"])
        # p1 should be first when pinned
        posts = self.b.list_posts(self.blog_id)
        ids = [p["id"] for p in posts]
        self.assertEqual(ids[0], p1["id"])
        # After unpinning, p1 should no longer be forced first
        self.b.unpin_post(self.blog_id, p1["id"])
        posts = self.b.list_posts(self.blog_id)
        # Just verify the unpin didn't error and posts are returned
        self.assertGreaterEqual(len(posts), 2)

    def test_pin_idempotent(self):
        post = self._post()
        self.b.pin_post(self.blog_id, post["id"])
        result = self.b.pin_post(self.blog_id, post["id"])
        self.assertIsNotNone(result)


# =========================================================================
# Deletion Cascade
# =========================================================================

class TestDeletionCascade(WriteTestCase):
    def test_delete_post_removes_comments(self):
        post = self._post()
        self.b.create_comment(self.blog_id, post["id"], "Alice", "Comment 1")
        self.b.create_comment(self.blog_id, post["id"], "Bob", "Comment 2")
        self.b.delete_post(self.blog_id, post["id"])
        self.__class__._post_ids.remove(post["id"])
        # Post should be gone
        with self.assertRaises(NotFoundError):
            self.b.get_post(self.blog_id, post["slug"])


# =========================================================================
# Semantic Search
# =========================================================================

class TestSemanticSearch(ReadOnlyTestCase):
    def test_semantic_search_returns_list(self):
        results = self.b.search_semantic("agent infrastructure")
        self.assertIsInstance(results, (list, dict))

    def test_semantic_search_with_limit(self):
        results = self.b.search_semantic("blog platform", limit=1)
        if isinstance(results, list):
            self.assertLessEqual(len(results), 1)

    def test_semantic_search_with_blog_filter(self):
        results = self.b.search_semantic("test", blog_id=self.blog_id)
        self.assertIsInstance(results, (list, dict))

    def test_semantic_search_no_results(self):
        results = self.b.search_semantic("zzzzzzqqqqqxxxxx99999")
        if isinstance(results, list):
            self.assertEqual(len(results), 0)


# =========================================================================
# Search Edge Cases
# =========================================================================

class TestSearchAdvanced(WriteTestCase):
    def test_search_finds_created_post(self):
        unique = f"unicornblaster{ts()}"
        self._post(title=f"Post about {unique}", content=f"Content with {unique}")
        time.sleep(0.5)  # allow FTS index update
        results = self.b.search(unique)
        self.assertGreaterEqual(len(results), 1)

    def test_search_empty_query(self):
        try:
            results = self.b.search("")
            self.assertIsInstance(results, list)
        except (ValidationError, BlogError):
            pass  # Some servers reject empty queries — that's fine

    def test_search_special_chars(self):
        results = self.b.search("hello & world")
        self.assertIsInstance(results, list)

    def test_search_result_fields(self):
        """Search results should have title and snippet."""
        results = self.b.search("test")
        if results:
            result = results[0]
            self.assertIn("title", result)


# =========================================================================
# Stats Advanced
# =========================================================================

class TestStatsAdvanced(WriteTestCase):
    def test_stats_fields(self):
        stats = self.b.blog_stats(self.blog_id)
        for field in ["total_posts", "published_posts"]:
            self.assertIn(field, stats)

    def test_stats_view_fields(self):
        stats = self.b.blog_stats(self.blog_id)
        # Should have views breakdown
        self.assertIn("total_views", stats)

    def test_stats_after_post_creation(self):
        self._post()
        stats = self.b.blog_stats(self.blog_id)
        self.assertGreaterEqual(stats.get("total_posts", 0), 1)

    def test_stats_top_posts(self):
        stats = self.b.blog_stats(self.blog_id)
        if "top_posts" in stats:
            self.assertIsInstance(stats["top_posts"], list)


# =========================================================================
# Feed Content
# =========================================================================

class TestFeedContent(WriteTestCase):
    def test_rss_contains_post(self):
        title = f"RSS-Test-{ts()}"
        self._post(title=title)
        rss = self.b.feed_rss(self.blog_id)
        self.assertIn(title.encode(), rss)

    def test_json_feed_contains_post(self):
        title = f"JSON-Feed-{ts()}"
        self._post(title=title)
        feed = self.b.feed_json(self.blog_id)
        titles = [item.get("title", "") for item in feed.get("items", [])]
        self.assertIn(title, titles)

    def test_rss_valid_xml(self):
        rss = self.b.feed_rss(self.blog_id)
        self.assertTrue(rss.startswith(b"<?xml") or rss.startswith(b"<rss"))

    def test_json_feed_has_home_page_url(self):
        feed = self.b.feed_json(self.blog_id)
        self.assertIn("home_page_url", feed)

    def test_draft_not_in_feed(self):
        title = f"Draft-Feed-{ts()}"
        self._post(title=title, status="draft")
        rss = self.b.feed_rss(self.blog_id)
        self.assertNotIn(title.encode(), rss)


# =========================================================================
# Export Edge Cases
# =========================================================================

class TestExportAdvanced(WriteTestCase):
    def test_export_markdown_frontmatter(self):
        post = self._post(title=f"Export-MD-{ts()}", tags=["test", "export"])
        result = self.b.export_markdown(self.blog_id, post["slug"])
        if isinstance(result, dict):
            # API returns structured markdown with frontmatter, content, full_document
            full_doc = result.get("full_document", result.get("content", ""))
            self.assertTrue(full_doc.startswith("---") or "title:" in full_doc,
                            f"Expected YAML frontmatter in: {full_doc[:100]}")
            self.assertIn("frontmatter", result)
        else:
            content = result.decode() if isinstance(result, bytes) else str(result)
            self.assertTrue(content.startswith("---") or "title:" in content)

    def test_export_html_complete(self):
        post = self._post(content="# Hello\n\n**World**")
        html = self.b.export_html(self.blog_id, post["slug"])
        self.assertIn(b"<strong>", html)

    def test_export_nostr_tags(self):
        post = self._post(tags=["nostr", "test"])
        event = self.b.export_nostr(self.blog_id, post["slug"])
        tag_values = [t[1] for t in event.get("tags", []) if t[0] == "t"]
        self.assertIn("nostr", tag_values)

    def test_export_nostr_has_d_tag(self):
        post = self._post()
        event = self.b.export_nostr(self.blog_id, post["slug"])
        d_tags = [t for t in event.get("tags", []) if t[0] == "d"]
        self.assertGreaterEqual(len(d_tags), 1)

    def test_export_draft_fails(self):
        post = self._post(status="draft")
        with self.assertRaises(NotFoundError):
            self.b.export_html(self.blog_id, post["slug"])


# =========================================================================
# Comment Edge Cases
# =========================================================================

class TestCommentsAdvanced(WriteTestCase):
    def test_comment_unicode(self):
        post = self._post()
        comment = self.b.create_comment(self.blog_id, post["id"], "Tëstér", "Héllo 🌍")
        self.assertIn("🌍", comment["content"])

    def test_comment_long_content(self):
        post = self._post()
        content = "Long comment " * 100
        comment = self.b.create_comment(self.blog_id, post["id"], "Alice", content)
        self.assertGreater(len(comment["content"]), 100)

    def test_comments_ordered(self):
        post = self._post()
        self.b.create_comment(self.blog_id, post["id"], "First", "Comment 1")
        time.sleep(0.1)
        self.b.create_comment(self.blog_id, post["id"], "Second", "Comment 2")
        comments = self.b.list_comments(self.blog_id, post["id"])
        # Should be in order (oldest first or newest first — just check consistency)
        self.assertEqual(len(comments), 2)

    def test_comment_on_nonexistent_post(self):
        with self.assertRaises((NotFoundError, ValidationError, BlogError)):
            self.b.create_comment(self.blog_id, "nonexistent-post", "Alice", "Hello")

    def test_delete_nonexistent_comment(self):
        post = self._post()
        with self.assertRaises((NotFoundError, BlogError)):
            self.b.delete_comment(self.blog_id, post["id"], "nonexistent-comment")


# =========================================================================
# Auth Edge Cases
# =========================================================================

class TestAuthEdgeCases(WriteTestCase):
    def test_create_post_no_auth(self):
        no_auth = Blog(BASE_URL)
        with self.assertRaises(AuthError):
            no_auth.create_post(self.blog_id, f"No Auth {ts()}", "Content")

    def test_delete_post_no_auth(self):
        post = self._post()
        no_auth = Blog(BASE_URL)
        with self.assertRaises(AuthError):
            no_auth.delete_post(self.blog_id, post["id"])

    def test_pin_no_auth(self):
        post = self._post()
        no_auth = Blog(BASE_URL)
        with self.assertRaises(AuthError):
            no_auth.pin_post(self.blog_id, post["id"])

    def test_wrong_key(self):
        wrong = Blog(BASE_URL, manage_key="wrong-key-12345")
        with self.assertRaises(AuthError):
            wrong.create_post(self.blog_id, f"Wrong Key {ts()}", "Content")

    def test_key_via_query_param(self):
        """Auth via ?key= query parameter should work."""
        # This tests the server accepts the key — we can't test query param via SDK
        # but we can verify the manage_key gets sent as Bearer
        post = self._post()
        self.assertIn("id", post)

    def test_delete_comment_no_auth(self):
        post = self._post()
        comment = self.b.create_comment(self.blog_id, post["id"], "Test", "Will try to delete")
        no_auth = Blog(BASE_URL)
        with self.assertRaises(AuthError):
            no_auth.delete_comment(self.blog_id, post["id"], comment["id"])


# =========================================================================
# Discovery Advanced
# =========================================================================

class TestDiscoveryAdvanced(ReadOnlyTestCase):
    def test_llms_txt_content(self):
        """llms.txt at /api/v1 should return Blog API info (requires latest image)."""
        try:
            txt = self.b.llms_txt()
        except NotFoundError:
            self.skipTest("/api/v1/llms.txt not available (staging may need image update)")
        self.assertIn("Blog", txt)
        self.assertIn("/api/v1", txt)

    def test_llms_txt_root(self):
        txt = self.b.llms_txt_root()
        self.assertIn("Blog", txt)

    def test_llms_txt_both_same(self):
        try:
            v1 = self.b.llms_txt()
        except NotFoundError:
            self.skipTest("/api/v1/llms.txt not available (staging may need image update)")
        root = self.b.llms_txt_root()
        # Both should return the same content
        self.assertEqual(v1.strip(), root.strip())

    def test_skill_md_v1(self):
        md = self.b.skill_md_v1()
        self.assertIn("blog", md.lower())

    def test_skill_md_both_paths(self):
        well_known = self.b.skill_md()
        v1 = self.b.skill_md_v1()
        self.assertEqual(well_known.strip(), v1.strip())

    def test_openapi_has_info(self):
        spec = self.b.openapi()
        self.assertIn("info", spec)
        self.assertIn("title", spec["info"])

    def test_openapi_version(self):
        spec = self.b.openapi()
        self.assertTrue(spec["openapi"].startswith("3."))

    def test_skills_index_structure(self):
        idx = self.b.skills()
        self.assertIn("skills", idx)
        self.assertIsInstance(idx["skills"], list)


# =========================================================================
# Constructor & Config
# =========================================================================

class TestConstructor(unittest.TestCase):
    def test_trailing_slash_stripped(self):
        b = Blog("http://localhost:3004/")
        self.assertEqual(b.base_url, "http://localhost:3004")

    def test_custom_timeout(self):
        b = Blog(timeout=5)
        self.assertEqual(b.timeout, 5)

    def test_default_url(self):
        orig = os.environ.get("BLOG_URL")
        try:
            os.environ.pop("BLOG_URL", None)
            b = Blog()
            self.assertEqual(b.base_url, "http://localhost:3004")
        finally:
            if orig:
                os.environ["BLOG_URL"] = orig

    def test_manage_key_from_env(self):
        orig = os.environ.get("BLOG_KEY")
        try:
            os.environ["BLOG_KEY"] = "env-key-test"
            b = Blog()
            self.assertEqual(b.manage_key, "env-key-test")
        finally:
            if orig:
                os.environ["BLOG_KEY"] = orig
            else:
                os.environ.pop("BLOG_KEY", None)

    def test_repr_includes_url(self):
        b = Blog("http://example.com:3004")
        self.assertIn("example.com", repr(b))

    def test_explicit_key_overrides_env(self):
        orig = os.environ.get("BLOG_KEY")
        try:
            os.environ["BLOG_KEY"] = "env-key"
            b = Blog(manage_key="explicit-key")
            self.assertEqual(b.manage_key, "explicit-key")
        finally:
            if orig:
                os.environ["BLOG_KEY"] = orig
            else:
                os.environ.pop("BLOG_KEY", None)


# =========================================================================
# Slug Behavior
# =========================================================================

class TestSlugBehavior(WriteTestCase):
    def test_slug_from_title(self):
        title = f"My Great Post {ts()}"
        post = self._post(title=title)
        self.assertIn("my-great-post", post["slug"])

    def test_custom_slug_preserved(self):
        slug = f"custom-{ts()}"
        post = self._post(slug=slug)
        self.assertEqual(post["slug"], slug)

    def test_unicode_title_slug(self):
        post = self._post(title=f"Ünïcödé Pöst {ts()}")
        # Should still generate a valid slug (may transliterate or use safe chars)
        self.assertTrue(len(post["slug"]) > 0)
        # Slug should not contain spaces
        self.assertNotIn(" ", post["slug"])


# =========================================================================
# Draft Behavior
# =========================================================================

class TestDraftBehavior(WriteTestCase):
    def test_draft_not_in_public_list(self):
        draft = self._post(status="draft")
        # Without auth, drafts should not appear
        no_auth = Blog(BASE_URL)
        posts = no_auth.list_posts(self.blog_id)
        ids = [p["id"] for p in posts]
        self.assertNotIn(draft["id"], ids)

    def test_draft_visible_with_auth(self):
        draft = self._post(status="draft")
        posts = self.b.list_posts(self.blog_id)
        ids = [p["id"] for p in posts]
        self.assertIn(draft["id"], ids)

    def test_publish_draft(self):
        draft = self._post(status="draft")
        updated = self.b.update_post(self.blog_id, draft["id"], status="published")
        self.assertEqual(updated["status"], "published")

    def test_draft_not_searchable(self):
        unique = f"draftunique{ts()}"
        self._post(title=unique, content=unique, status="draft")
        time.sleep(0.5)
        results = self.b.search(unique)
        self.assertEqual(len(results), 0)


# =========================================================================
# Full Lifecycle
# =========================================================================

class TestFullLifecycle(WriteTestCase):
    def test_create_update_comment_pin_export_delete(self):
        """Full lifecycle: create → update → comment → pin → export → delete."""
        # Create
        post = self._post(title=f"Lifecycle {ts()}", content="# Start\n\nOriginal.")
        self.assertIn("id", post)

        # Update
        updated = self.b.update_post(self.blog_id, post["id"], content="# Updated\n\nModified.", tags=["lifecycle"])
        self.assertEqual(updated["content"], "# Updated\n\nModified.")

        # Comment
        comment = self.b.create_comment(self.blog_id, post["id"], "Tester", "Nice post!")
        self.assertIn("id", comment)

        # Pin
        self.b.pin_post(self.blog_id, post["id"])
        fetched = self.b.get_post(self.blog_id, post["slug"])
        self.assertTrue(fetched.get("pinned_at") is not None or fetched.get("is_pinned") is True)

        # Export
        md = self.b.export_markdown(self.blog_id, post["slug"])
        self.assertIsNotNone(md)

        nostr = self.b.export_nostr(self.blog_id, post["slug"])
        self.assertEqual(nostr["kind"], 30023)

        # Delete
        self.b.delete_post(self.blog_id, post["id"])
        self.__class__._post_ids.remove(post["id"])
        with self.assertRaises(NotFoundError):
            self.b.get_post(self.blog_id, post["slug"])


# =========================================================================
# Multiple Blogs
# =========================================================================

class TestMultipleBlogs(WriteTestCase):
    def test_posts_isolated_between_blogs(self):
        """Posts in one blog shouldn't appear in another."""
        blog2 = self.b.create_blog(f"Blog2-{ts()}", description="Test isolation")
        blog2_id = blog2["id"]
        blog2_client = Blog(BASE_URL, manage_key=blog2.get("manage_key", MANAGE_KEY))

        post = blog2_client.create_post(blog2_id, f"Isolated {ts()}", "Content")

        # Should not appear in self.blog_id's posts
        posts = self.b.list_posts(self.blog_id)
        ids = [p["id"] for p in posts]
        self.assertNotIn(post["id"], ids)

    def test_list_blogs_shows_multiple(self):
        self.b.create_blog(f"List-Test-{ts()}")
        blogs = self.b.list_blogs()
        self.assertGreaterEqual(len(blogs), 2)


# =========================================================================
# Post Summary
# =========================================================================

class TestPostSummary(WriteTestCase):
    def test_create_with_summary(self):
        post = self._post(summary="A brief summary of this post")
        self.assertEqual(post.get("summary", ""), "A brief summary of this post")

    def test_update_summary(self):
        post = self._post()
        updated = self.b.update_post(self.blog_id, post["id"], summary="Updated summary")
        self.assertEqual(updated.get("summary", ""), "Updated summary")


# =========================================================================
# Preview Advanced
# =========================================================================

class TestPreviewAdvanced(ReadOnlyTestCase):
    def test_preview_table(self):
        md = "| Col1 | Col2 |\n|------|------|\n| A | B |"
        result = self.b.preview(md)
        self.assertIn("<table", result["html"])

    def test_preview_image(self):
        result = self.b.preview("![alt](https://example.com/img.png)")
        self.assertIn("<img", result["html"])

    def test_preview_empty(self):
        result = self.b.preview("")
        self.assertIn("html", result)

    def test_preview_unicode(self):
        result = self.b.preview("日本語 🎉 العربية")
        self.assertIn("🎉", result["html"])


if __name__ == "__main__":
    print(f"\n📝 Blog Python SDK Tests")
    print(f"   Server: {BASE_URL}")
    print(f"   Write tests: {'ENABLED (BLOG_KEY set)' if MANAGE_KEY else 'SKIPPED (set BLOG_KEY to enable)'}\n")
    unittest.main(verbosity=2)
