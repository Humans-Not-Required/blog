import React, { useState, useEffect } from 'react';
import { apiFetch, formatDate } from '../utils';
import { useDocTitle } from '../hooks';

const HNR_BLOG_ID = '0416e210-514a-49e0-9b24-16e1763debf0';

export default function HNRBlog({ onNavigate }) {
  const [blog, setBlog] = useState(null);
  const [posts, setPosts] = useState([]);
  const [filterTag, setFilterTag] = useState(null);

  useDocTitle(null); // default title on home page

  useEffect(() => {
    apiFetch(`/blogs/${HNR_BLOG_ID}`).then(setBlog).catch(console.error);
    apiFetch(`/blogs/${HNR_BLOG_ID}/posts`).then(setPosts).catch(console.error);
  }, []);

  if (!blog) {
    return (
      <div className="container">
        <p className="muted">Loading...</p>
      </div>
    );
  }

  // Filter to published posts only
  const allPublished = posts.filter(p => p.status === 'published');

  // Apply tag filter if active
  const publishedPosts = filterTag
    ? allPublished.filter(p => p.tags.includes(filterTag))
    : allPublished;

  // Sort: pinned first, then by published date (newest first)
  const sortedPosts = [...publishedPosts].sort((a, b) => {
    if (a.is_pinned && !b.is_pinned) return -1;
    if (!a.is_pinned && b.is_pinned) return 1;
    const dateA = new Date(a.published_at || a.created_at);
    const dateB = new Date(b.published_at || b.created_at);
    return dateB - dateA;
  });

  // Get all unique tags from published posts
  const allTags = [...new Set(allPublished.flatMap(p => p.tags))].sort();

  // Separate pinned and regular posts for display
  const pinnedPosts = sortedPosts.filter(p => p.is_pinned);
  const regularPosts = sortedPosts.filter(p => !p.is_pinned);

  return (
    <>
      <div className="hnr-hero">
        <h1>Humans Not Required</h1>
        <p className="hnr-hero__tagline">
          Exploring the frontier of AI agents, automation, and the future of work.
          A blog about building intelligent systems that work while we dream.
        </p>
      </div>

      <div className="container">
        {allTags.length > 0 && (
          <div className="tags-filter">
            {filterTag && (
              <button className="btn btn--sm" onClick={() => setFilterTag(null)}>
                ✕ Clear filter
              </button>
            )}
            {allTags.map(t => (
              <button
                key={t}
                onClick={() => setFilterTag(filterTag === t ? null : t)}
                className={`tag tag--btn${filterTag === t ? ' tag--active' : ''}`}
              >
                {t}
              </button>
            ))}
            {filterTag && (
              <span className="filter-count">
                {publishedPosts.length} result{publishedPosts.length !== 1 ? 's' : ''}
              </span>
            )}
          </div>
        )}

        {sortedPosts.length === 0 && !filterTag && (
          <div className="card card--empty">
            <p className="muted">No posts yet. Check back soon!</p>
          </div>
        )}

        {sortedPosts.length === 0 && filterTag && (
          <div className="card card--empty">
            <p className="muted">No posts found with this tag.</p>
          </div>
        )}

        {pinnedPosts.length > 0 && (
          <>
            {pinnedPosts.map(post => (
              <article
                key={post.id}
                className="hnr-post-card card card--hoverable"
                onClick={() => onNavigate('post', HNR_BLOG_ID, post.slug)}
              >
                <h2>
                  <span className="post-card__pin" title="Pinned">📌</span>
                  {post.title}
                </h2>
                <div className="post-meta">
                  {post.author_name && <span className="post-meta__author">{post.author_name}</span>}
                  <span>{formatDate(post.published_at || post.created_at)}</span>
                  {post.reading_time_minutes > 0 && <span>· {post.reading_time_minutes} min read</span>}
                  {post.comment_count > 0 && <span>💬 {post.comment_count}</span>}
                </div>
                {post.summary && (
                  <p className="hnr-post-card__summary">{post.summary}</p>
                )}
                <div className="hnr-post-card__read-more">Read more →</div>
                {post.tags.length > 0 && (
                  <div className="post-card__tags">
                    {post.tags.map((t, i) => (
                      <span
                        key={i}
                        onClick={(e) => {
                          e.stopPropagation();
                          setFilterTag(filterTag === t ? null : t);
                        }}
                        className={`tag tag--btn${filterTag === t ? ' tag--active' : ''}`}
                      >
                        {t}
                      </span>
                    ))}
                  </div>
                )}
              </article>
            ))}
          </>
        )}

        {regularPosts.map(post => (
          <article
            key={post.id}
            className="hnr-post-card card card--hoverable"
            onClick={() => onNavigate('post', HNR_BLOG_ID, post.slug)}
          >
            <h2>{post.title}</h2>
            <div className="post-meta">
              {post.author_name && <span className="post-meta__author">{post.author_name}</span>}
              <span>{formatDate(post.published_at || post.created_at)}</span>
              {post.reading_time_minutes > 0 && <span>· {post.reading_time_minutes} min read</span>}
              {post.comment_count > 0 && <span>💬 {post.comment_count}</span>}
            </div>
            {post.summary && (
              <p className="hnr-post-card__summary">{post.summary}</p>
            )}
            <div className="hnr-post-card__read-more">Read more →</div>
            {post.tags.length > 0 && (
              <div className="post-card__tags">
                {post.tags.map((t, i) => (
                  <span
                    key={i}
                    onClick={(e) => {
                      e.stopPropagation();
                      setFilterTag(filterTag === t ? null : t);
                    }}
                    className={`tag tag--btn${filterTag === t ? ' tag--active' : ''}`}
                  >
                    {t}
                  </span>
                ))}
              </div>
            )}
          </article>
        ))}
      </div>
    </>
  );
}
