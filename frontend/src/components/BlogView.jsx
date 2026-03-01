import React, { useState, useEffect, useCallback } from 'react';
import { apiFetch, formatDate, addMyBlog } from '../utils';
import { useEscapeKey } from '../hooks';
import PostEditor from './PostEditor';

export default function BlogView({ blogId, onNavigate }) {
  const [blog, setBlog] = useState(null);
  const [posts, setPosts] = useState([]);
  const [showCreate, setShowCreate] = useState(false);
  const [filterTag, setFilterTag] = useState(null);
  const manageKey = localStorage.getItem(`blog_key_${blogId}`);
  const canEdit = !!manageKey;

  const refreshPosts = useCallback(() => {
    apiFetch(`/blogs/${blogId}/posts`).then(setPosts).catch(console.error);
  }, [blogId]);

  useEffect(() => {
    apiFetch(`/blogs/${blogId}`).then(b => {
      setBlog(b);
      if (b && b.name) addMyBlog(b.id, b.name);
    }).catch(() => setBlog({ error: true }));
    refreshPosts();
  }, [blogId, refreshPosts]);

  useEscapeKey(useCallback(() => {
    if (showCreate) setShowCreate(false);
  }, [showCreate]));

  if (!blog) return <div className="container"><p className="muted">Loading...</p></div>;
  if (blog.error) return (
    <div className="container">
      <div className="card card--empty">
        <p className="mb-8">Blog not found.</p>
        <span className="link" onClick={() => onNavigate('home')}>← Back home</span>
      </div>
    </div>
  );

  const allPublished = posts.filter(p => p.status === 'published');
  const allDrafts = posts.filter(p => p.status === 'draft');
  const publishedPosts = filterTag ? allPublished.filter(p => p.tags.includes(filterTag)) : allPublished;
  const draftPosts = filterTag ? allDrafts.filter(p => p.tags.includes(filterTag)) : allDrafts;
  const allTags = [...new Set(posts.flatMap(p => p.tags))].sort();

  return (
    <div className="container">
      <div className="blog-header">
        <div className="blog-header__info">
          <h1>{blog.name}</h1>
          {blog.description && <p className="muted mb-8">{blog.description}</p>}
          <div className="badges-row">
            {canEdit && <span className="badge badge--green">Full Access</span>}
            <span className="badge">{publishedPosts.length} published</span>
            {canEdit && draftPosts.length > 0 && <span className="badge badge--blue">{draftPosts.length} draft{draftPosts.length !== 1 ? 's' : ''}</span>}
          </div>
        </div>
        <div className="blog-header__actions">
          <a
            className="btn btn--sm"
            href={`/api/v1/blogs/${blogId}/feed.rss`}
            target="_blank"
            rel="noreferrer"
          >RSS</a>
          {canEdit && <button className="btn btn--primary" onClick={() => setShowCreate(true)}>New Post</button>}
        </div>
      </div>

      {showCreate && canEdit && (
        <PostEditor
          blogId={blogId}
          onDone={() => { setShowCreate(false); refreshPosts(); }}
          onCancel={() => setShowCreate(false)}
        />
      )}

      {allTags.length > 0 && (
        <div className="tags-filter">
          {filterTag && (
            <button className="btn btn--sm" onClick={() => setFilterTag(null)}>✕ Clear filter</button>
          )}
          {allTags.map(t => (
            <button
              key={t}
              onClick={() => setFilterTag(filterTag === t ? null : t)}
              className={`tag tag--btn${filterTag === t ? ' tag--active' : ''}`}
            >{t}</button>
          ))}
          {filterTag && <span className="filter-count">{publishedPosts.length + draftPosts.length} result{publishedPosts.length + draftPosts.length !== 1 ? 's' : ''}</span>}
        </div>
      )}

      {posts.length === 0 && !showCreate && (
        <div className="card card--empty">
          <p className={`muted ${canEdit ? "mb-12" : "mb-0"}`}>No posts yet.</p>
          {canEdit && <button className="btn btn--primary" onClick={() => setShowCreate(true)}>Write your first post →</button>}
        </div>
      )}

      {canEdit && draftPosts.length > 0 && (
        <>
          <h3 className="section-label">Drafts</h3>
          {draftPosts.map(p => (
            <div key={p.id} className="card card--hoverable post-card" onClick={() => onNavigate('post', blogId, p.slug)}>
              <div className="draft-row">
                <h2>{p.title}</h2>
                <span className="badge badge--draft">Draft</span>
              </div>
              <div className="post-meta">
                {p.author_name && <span>{p.author_name}</span>}
                <span>{formatDate(p.created_at)}</span>
                {p.reading_time_minutes > 0 && <span>· {p.reading_time_minutes} min read</span>}
              </div>
            </div>
          ))}
        </>
      )}

      {publishedPosts.length > 0 && (
        <>
          {canEdit && draftPosts.length > 0 && (
            <h3 className="section-label mt-16">Published</h3>
          )}
          {publishedPosts.map(p => (
            <div key={p.id} className="card card--hoverable post-card" onClick={() => onNavigate('post', blogId, p.slug)}>
              <h2>
                {p.is_pinned && <span className="post-card__pin" title="Pinned">📌</span>}
                {p.title}
              </h2>
              <div className="post-meta">
                {p.author_name && <span className="post-meta__author">{p.author_name}</span>}
                <span>{formatDate(p.published_at || p.created_at)}</span>
                {p.reading_time_minutes > 0 && <span>· {p.reading_time_minutes} min read</span>}
                {p.comment_count > 0 && <span>💬 {p.comment_count}</span>}
              </div>
              {p.summary && <p className="post-card__summary">{p.summary}</p>}
              {p.tags.length > 0 && (
                <div className="post-card__tags">
                  {p.tags.map((t, i) => (
                    <span
                      key={i}
                      onClick={(e) => { e.stopPropagation(); setFilterTag(filterTag === t ? null : t); }}
                      className={`tag tag--btn${filterTag === t ? ' tag--active' : ''}`}
                    >{t}</span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </>
      )}
    </div>
  );
}
