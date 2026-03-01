import React, { useState, useEffect, useCallback, useRef } from 'react';
import { apiFetch, getSavedAuthor, saveAuthor, modKey } from '../utils';
import { useEscapeKey } from '../hooks';

const API = '/api/v1';

export default function PostEditor({ blogId, post, onDone, onCancel }) {
  const [title, setTitle] = useState(post?.title || '');
  const [content, setContent] = useState(post?.content || '');
  const [summary, setSummary] = useState(post?.summary || '');
  const [tags, setTags] = useState(post?.tags?.join(', ') || '');
  const [status, setStatus] = useState(post?.status || 'draft');
  const [authorName, setAuthorName] = useState(post?.author_name || getSavedAuthor());
  const [showPreview, setShowPreview] = useState(false);
  const [previewHtml, setPreviewHtml] = useState('');
  const [previewLoading, setPreviewLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const titleRef = useRef(null);

  useEffect(() => {
    if (titleRef.current && !post) titleRef.current.focus();
  }, [post]);

  useEscapeKey(useCallback(() => { onCancel(); }, [onCancel]));

  useEffect(() => {
    if (!showPreview || !content.trim()) {
      setPreviewHtml('');
      return;
    }
    setPreviewLoading(true);
    const timer = setTimeout(() => {
      fetch(`${API}/preview`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content }),
      })
        .then(r => r.json())
        .then(d => { setPreviewHtml(d.html); setPreviewLoading(false); })
        .catch(() => setPreviewLoading(false));
    }, 300);
    return () => clearTimeout(timer);
  }, [content, showPreview]);

  useEffect(() => {
    if (previewHtml && window.hljs) {
      setTimeout(() => {
        document.querySelectorAll('.preview-content pre code').forEach(el => {
          window.hljs.highlightElement(el);
        });
      }, 50);
    }
  }, [previewHtml]);

  const handleSave = async () => {
    setSaving(true);
    try {
      const body = {
        title, content, summary,
        tags: tags.split(',').map(t => t.trim()).filter(Boolean),
        status, author_name: authorName,
      };
      if (post) {
        await apiFetch(`/blogs/${blogId}/posts/${post.id}`, { method: 'PATCH', body });
      } else {
        await apiFetch(`/blogs/${blogId}/posts`, { method: 'POST', body });
      }
      saveAuthor(authorName);
      onDone();
    } catch (err) {
      alert(err.error || 'Failed to save');
    } finally {
      setSaving(false);
    }
  };

  const handleKeyDown = (e) => {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault();
      if (title.trim()) handleSave();
    }
  };

  return (
    <div className="card editor-card" onKeyDown={handleKeyDown}>
      <input ref={titleRef} className="input" placeholder="Post title" value={title} onChange={e => setTitle(e.target.value)} />
      <input className="input" placeholder="Author name" value={authorName} onChange={e => setAuthorName(e.target.value)} />

      <div className="editor-tabs">
        <button className={`editor-tab${!showPreview ? ' editor-tab--active' : ''}`} onClick={() => setShowPreview(false)}>Write</button>
        <button className={`editor-tab${showPreview ? ' editor-tab--active' : ''}`} onClick={() => setShowPreview(true)}>Preview</button>
        <span className="editor-hint">{modKey}+S to save · Esc to cancel</span>
      </div>

      {!showPreview ? (
        <textarea className="textarea" placeholder="Write your post in Markdown..." value={content} onChange={e => setContent(e.target.value)} />
      ) : (
        <div className="preview-pane post-content preview-content">
          {previewLoading ? (
            <span className="muted">Rendering preview...</span>
          ) : previewHtml ? (
            <div dangerouslySetInnerHTML={{ __html: previewHtml }} />
          ) : (
            <span className="muted">Nothing to preview</span>
          )}
        </div>
      )}

      <input className="input" placeholder="Summary (optional, shown in post listings)" value={summary} onChange={e => setSummary(e.target.value)} />
      <input className="input" placeholder="Tags (comma-separated)" value={tags} onChange={e => setTags(e.target.value)} />
      <div className="editor-actions">
        <select className="input select select--inline" value={status} onChange={e => setStatus(e.target.value)}>
          <option value="draft">Draft</option>
          <option value="published">Published</option>
        </select>
        <button className="btn btn--primary" onClick={handleSave} disabled={!title.trim() || saving}>
          {saving ? 'Saving...' : post ? 'Update' : 'Create'} {saving ? '' : 'Post'}
        </button>
        <button className="btn" onClick={onCancel}>Cancel</button>
      </div>
    </div>
  );
}
