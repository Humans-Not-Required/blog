import React, { useState, useEffect, useCallback, useRef } from 'react';
import { apiFetch, formatDate, getSavedAuthor, saveAuthor, modKey } from '../utils';
import { useEscapeKey, useDocTitle } from '../hooks';
import PostEditor from './PostEditor';

function slugify(text) {
  return text.toLowerCase().replace(/[^\w\s-]/g, '').replace(/\s+/g, '-').replace(/-+/g, '-').trim();
}

function enhancePostContent(containerEl) {
  if (!containerEl) return;

  // Add copy buttons to code blocks
  containerEl.querySelectorAll('pre > code').forEach(codeEl => {
    const pre = codeEl.parentElement;
    if (pre.querySelector('.code-copy-btn')) return;
    const btn = document.createElement('button');
    btn.className = 'code-copy-btn';
    btn.textContent = 'Copy';
    btn.setAttribute('aria-label', 'Copy code');
    btn.addEventListener('click', () => {
      navigator.clipboard.writeText(codeEl.textContent).then(() => {
        btn.textContent = 'Copied!';
        btn.classList.add('code-copy-btn--success');
        setTimeout(() => { btn.textContent = 'Copy'; btn.classList.remove('code-copy-btn--success'); }, 1500);
      });
    });
    pre.style.position = 'relative';
    pre.appendChild(btn);
  });

  // Add anchor links to headings
  const headings = containerEl.querySelectorAll('h1, h2, h3, h4');
  const seen = {};
  headings.forEach(h => {
    if (h.querySelector('.heading-anchor')) return;
    let id = slugify(h.textContent);
    if (seen[id]) { seen[id]++; id += '-' + seen[id]; } else { seen[id] = 1; }
    h.id = id;
    const anchor = document.createElement('a');
    anchor.className = 'heading-anchor';
    anchor.href = '#' + id;
    anchor.textContent = '#';
    anchor.setAttribute('aria-label', 'Link to this heading');
    anchor.addEventListener('click', (e) => {
      e.preventDefault();
      navigator.clipboard.writeText(window.location.origin + window.location.pathname + '#' + id);
      h.scrollIntoView({ behavior: 'smooth' });
      window.history.replaceState(null, '', '#' + id);
    });
    h.appendChild(anchor);
  });
}

function extractTOC(containerEl) {
  if (!containerEl) return [];
  const headings = containerEl.querySelectorAll('h1, h2, h3');
  return Array.from(headings).map(h => ({
    id: h.id,
    text: h.textContent.replace(/#$/, '').trim(),
    level: parseInt(h.tagName[1]),
  }));
}

export default function PostView({ blogId, slug, onNavigate }) {
  const [post, setPost] = useState(null);
  const [blog, setBlog] = useState(null);
  const [comments, setComments] = useState([]);
  const [relatedPosts, setRelatedPosts] = useState([]);
  const [newComment, setNewComment] = useState('');
  const [commentAuthor, setCommentAuthor] = useState(getSavedAuthor());
  const [editing, setEditing] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const [toc, setToc] = useState([]);
  const manageKey = localStorage.getItem(`blog_key_${blogId}`);
  const canEdit = !!manageKey;
  const commentsEndRef = useRef(null);
  const contentRef = useRef(null);

  useDocTitle(post?.title || null);

  const loadPost = useCallback(() => {
    apiFetch(`/blogs/${blogId}/posts/${slug}`).then(setPost).catch(() => setPost({ error: true }));
  }, [blogId, slug]);

  const loadComments = useCallback(() => {
    if (!post?.id) return;
    apiFetch(`/blogs/${blogId}/posts/${post.id}/comments`).then(setComments).catch(console.error);
  }, [blogId, post?.id]);

  useEffect(() => { loadPost(); }, [loadPost]);
  useEffect(() => {
    apiFetch(`/blogs/${blogId}`).then(setBlog).catch(console.error);
  }, [blogId]);
  useEffect(() => { loadComments(); }, [loadComments]);

  useEffect(() => {
    if (!post?.id) return;
    apiFetch(`/blogs/${blogId}/posts/${post.id}/related?limit=5`)
      .then(setRelatedPosts)
      .catch(() => setRelatedPosts([]));
  }, [blogId, post?.id]);

  useEffect(() => {
    if (post?.content_html && contentRef.current) {
      setTimeout(() => {
        if (window.hljs) {
          contentRef.current.querySelectorAll('pre code').forEach(el => {
            window.hljs.highlightElement(el);
          });
        }
        enhancePostContent(contentRef.current);
        setToc(extractTOC(contentRef.current));
      }, 50);
    }
  }, [post?.content_html]);

  useEscapeKey(useCallback(() => {
    if (editing) setEditing(false);
  }, [editing]));

  const handleComment = async () => {
    if (!newComment.trim() || !commentAuthor.trim()) return;
    setSubmitting(true);
    try {
      await apiFetch(`/blogs/${blogId}/posts/${post.id}/comments`, {
        method: 'POST', body: { author_name: commentAuthor, content: newComment },
      });
      saveAuthor(commentAuthor);
      setNewComment('');
      loadComments();
      setTimeout(() => commentsEndRef.current?.scrollIntoView({ behavior: 'smooth' }), 100);
    } catch (err) {
      alert(err.error || 'Failed to post comment');
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!confirm('Delete this post? This cannot be undone.')) return;
    try {
      await apiFetch(`/blogs/${blogId}/posts/${post.id}`, { method: 'DELETE' });
      onNavigate('blog', blogId);
    } catch (err) {
      alert(err.error || 'Failed to delete');
    }
  };

  const handlePin = async () => {
    try {
      const endpoint = post.is_pinned ? 'unpin' : 'pin';
      const updated = await apiFetch(`/blogs/${blogId}/posts/${post.id}/${endpoint}`, { method: 'POST' });
      setPost(updated);
    } catch (err) {
      alert(err.error || 'Failed to update pin status');
    }
  };

  const handleDeleteComment = async (commentId) => {
    if (!confirm('Delete this comment?')) return;
    try {
      await apiFetch(`/blogs/${blogId}/posts/${post.id}/comments/${commentId}`, { method: 'DELETE' });
      loadComments();
    } catch (err) {
      alert(err.error || 'Failed to delete comment');
    }
  };

  if (!post) return <div className="container"><p className="muted">Loading...</p></div>;
  if (post.error) return (
    <div className="container">
      <div className="card card--empty">
        <p className="mb-8">Post not found.</p>
        <span className="link" onClick={() => onNavigate('blog', blogId)}>← Back to blog</span>
      </div>
    </div>
  );

  if (editing && canEdit) {
    return (
      <div className="container">
        <PostEditor blogId={blogId} post={post} onDone={() => { setEditing(false); loadPost(); }} onCancel={() => setEditing(false)} />
      </div>
    );
  }

  return (
    <div className="container">
      <div className="post-nav">
        <span className="link" onClick={() => onNavigate('blog', blogId)}>
          ← {blog?.name || 'Back to blog'}
        </span>
      </div>

      <article className="card">
        <div className="post-title-row">
          <h1>
            {post.is_pinned && <span title="Pinned" className="pinned-icon">📌</span>}
            {post.title}
          </h1>
          {canEdit && (
            <div className="post-title-row__actions">
              <button className="btn btn--sm" onClick={handlePin} title={post.is_pinned ? 'Unpin post' : 'Pin post'}>{post.is_pinned ? '📌' : '📍'}</button>
              <button className="btn btn--sm" onClick={() => setEditing(true)} title="Edit post">✏️</button>
              <button className="btn btn--sm btn--danger" onClick={handleDelete} title="Delete post">🗑️</button>
            </div>
          )}
        </div>
        <div className="post-detail-meta">
          {post.author_name && <span className="post-detail-meta__author">{post.author_name}</span>}
          <span>{formatDate(post.published_at || post.created_at)}</span>
          {post.reading_time_minutes > 0 && <span>· {post.reading_time_minutes} min read</span>}
          {post.word_count > 0 && <span className="post-detail-meta__words">({post.word_count.toLocaleString()} words)</span>}
          {post.view_count > 0 && <span title="Views">👁 {post.view_count.toLocaleString()}</span>}
          {post.status === 'draft' && <span className="badge badge--draft">Draft</span>}
        </div>
        {post.tags.length > 0 && <div className="post-tags">{post.tags.map((t, i) => <span key={i} className="tag">{t}</span>)}</div>}

        {toc.length >= 3 && (
          <nav className="toc" aria-label="Table of contents">
            <div className="toc__title">Contents</div>
            <ul className="toc__list">
              {toc.map((item, i) => (
                <li key={i} className={`toc__item toc__item--${item.level}`}>
                  <a href={'#' + item.id} onClick={(e) => {
                    e.preventDefault();
                    document.getElementById(item.id)?.scrollIntoView({ behavior: 'smooth' });
                    window.history.replaceState(null, '', '#' + item.id);
                  }}>{item.text}</a>
                </li>
              ))}
            </ul>
          </nav>
        )}

        <div
          ref={contentRef}
          className="post-content"
          dangerouslySetInnerHTML={{ __html: post.content_html }}
        />
      </article>

      {relatedPosts.length > 0 && (
        <div className="card">
          <h3 className="mb-16">Related Posts</h3>
          <div className="related-posts-list">
            {relatedPosts.map(rp => (
              <div
                key={rp.id}
                className="related-post"
                onClick={() => onNavigate('post', blogId, rp.slug)}
              >
                <div className="related-post__title">{rp.title}</div>
                <div className="related-post__meta">
                  {rp.author_name && <span>{rp.author_name}</span>}
                  {rp.published_at && <span>{formatDate(rp.published_at)}</span>}
                  {rp.reading_time_minutes > 0 && <span>· {rp.reading_time_minutes} min read</span>}
                </div>
                {rp.tags.length > 0 && (
                  <div className="related-post__tags">
                    {rp.tags.map((t, i) => <span key={i} className="tag tag--sm">{t}</span>)}
                  </div>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="card">
        <h3 className="mb-16">Comments ({comments.length})</h3>

        {comments.length === 0 && (
          <p className="muted mb-16">No comments yet. Be the first!</p>
        )}

        {comments.map(c => (
          <div key={c.id} className="comment">
            <div className="comment__header">
              <strong className="comment__author">{c.author_name}</strong>
              <div className="comment__meta">
                <span className="muted comment-date">{formatDate(c.created_at)}</span>
                {canEdit && (
                  <button onClick={() => handleDeleteComment(c.id)} className="btn--ghost" title="Delete comment">✕</button>
                )}
              </div>
            </div>
            <p className="comment__body">{c.content}</p>
          </div>
        ))}
        <div ref={commentsEndRef} />

        {post.status === 'published' && (
          <div className={`comment-form${comments.length > 0 ? ' comment-form--bordered' : ''}`}>
            <input
              className="input"
              placeholder="Your name"
              value={commentAuthor}
              onChange={e => setCommentAuthor(e.target.value)}
            />
            <textarea
              className="textarea textarea--short"
              placeholder="Write a comment..."
              value={newComment}
              onChange={e => setNewComment(e.target.value)}
              onKeyDown={e => {
                if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                  e.preventDefault();
                  handleComment();
                }
              }}
            />
            <div className="comment-form__footer">
              <button className="btn btn--primary" onClick={handleComment} disabled={!newComment.trim() || !commentAuthor.trim() || submitting}>
                {submitting ? 'Posting...' : 'Post Comment'}
              </button>
              <span className="comment-submit-hint">{modKey}+Enter to submit</span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
