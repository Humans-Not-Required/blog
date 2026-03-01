import React, { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { apiFetch, getSavedAuthor, saveAuthor, modKey } from '../utils';
import { useEscapeKey } from '../hooks';

const API = '/api/v1';

function countWords(text) {
  if (!text || !text.trim()) return 0;
  return text.trim().split(/\s+/).length;
}

function wrapSelection(textarea, before, after, placeholder) {
  const start = textarea.selectionStart;
  const end = textarea.selectionEnd;
  const text = textarea.value;
  const selected = text.slice(start, end);
  const replacement = selected || placeholder || '';
  const newText = text.slice(0, start) + before + replacement + after + text.slice(end);
  const cursorStart = start + before.length;
  const cursorEnd = cursorStart + replacement.length;
  return { value: newText, cursorStart, cursorEnd };
}

function insertAtCursor(textarea, prefix, placeholder) {
  const start = textarea.selectionStart;
  const text = textarea.value;
  const lineStart = text.lastIndexOf('\n', start - 1) + 1;
  const currentLine = text.slice(lineStart, start);
  let newText, cursorPos;
  if (currentLine.trim() === '') {
    newText = text.slice(0, lineStart) + prefix + placeholder + text.slice(start);
    cursorPos = lineStart + prefix.length;
  } else {
    newText = text.slice(0, start) + '\n' + prefix + placeholder + text.slice(start);
    cursorPos = start + 1 + prefix.length;
  }
  return { value: newText, cursorStart: cursorPos, cursorEnd: cursorPos + placeholder.length };
}

const TOOLBAR_ACTIONS = [
  { key: 'bold', label: 'B', title: `Bold (${modKey}+B)`, className: 'md-toolbar__btn--bold' },
  { key: 'italic', label: 'I', title: `Italic (${modKey}+I)`, className: 'md-toolbar__btn--italic' },
  { key: 'heading', label: 'H', title: 'Heading' },
  { key: 'sep1', sep: true },
  { key: 'link', label: '🔗', title: 'Link' },
  { key: 'code', label: '<>', title: 'Inline code' },
  { key: 'codeblock', label: '```', title: 'Code block' },
  { key: 'sep2', sep: true },
  { key: 'ul', label: '•', title: 'Bullet list' },
  { key: 'ol', label: '1.', title: 'Numbered list' },
  { key: 'quote', label: '❝', title: 'Blockquote' },
];

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
  const textareaRef = useRef(null);

  const initialContent = useRef({
    title: post?.title || '', content: post?.content || '',
    summary: post?.summary || '', tags: post?.tags?.join(', ') || '',
    status: post?.status || 'draft', authorName: post?.author_name || getSavedAuthor(),
  });

  const hasChanges = useMemo(() => {
    const init = initialContent.current;
    return title !== init.title || content !== init.content || summary !== init.summary
      || tags !== init.tags || status !== init.status || authorName !== init.authorName;
  }, [title, content, summary, tags, status, authorName]);

  useEffect(() => {
    const handler = (e) => {
      if (hasChanges) { e.preventDefault(); e.returnValue = ''; }
    };
    window.addEventListener('beforeunload', handler);
    return () => window.removeEventListener('beforeunload', handler);
  }, [hasChanges]);

  const wordCount = useMemo(() => countWords(content), [content]);

  useEffect(() => {
    if (titleRef.current && !post) titleRef.current.focus();
  }, [post]);

  useEscapeKey(useCallback(() => { onCancel(); }, [onCancel]));

  useEffect(() => {
    if (!showPreview || !content.trim()) { setPreviewHtml(''); return; }
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

  const applyToolbar = useCallback((action) => {
    const ta = textareaRef.current;
    if (!ta) return;
    let result;
    switch (action) {
      case 'bold':
        result = wrapSelection(ta, '**', '**', 'bold text');
        break;
      case 'italic':
        result = wrapSelection(ta, '_', '_', 'italic text');
        break;
      case 'heading':
        result = insertAtCursor(ta, '## ', 'Heading');
        break;
      case 'link': {
        if (ta.selectionStart !== ta.selectionEnd) {
          const selected = ta.value.slice(ta.selectionStart, ta.selectionEnd);
          result = wrapSelection(ta, '[', '](url)', '');
          result.cursorStart = ta.selectionStart + 1 + selected.length + 2;
          result.cursorEnd = result.cursorStart + 3;
        } else {
          result = wrapSelection(ta, '[', '](url)', 'link text');
        }
        break;
      }
      case 'code':
        result = wrapSelection(ta, '`', '`', 'code');
        break;
      case 'codeblock':
        result = wrapSelection(ta, '```\n', '\n```', 'code here');
        break;
      case 'ul':
        result = insertAtCursor(ta, '- ', 'list item');
        break;
      case 'ol':
        result = insertAtCursor(ta, '1. ', 'list item');
        break;
      case 'quote':
        result = insertAtCursor(ta, '> ', 'quote');
        break;
      default:
        return;
    }
    setContent(result.value);
    setTimeout(() => { ta.focus(); ta.setSelectionRange(result.cursorStart, result.cursorEnd); }, 0);
  }, []);

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
      initialContent.current = { title, content, summary, tags, status, authorName };
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
    if ((e.metaKey || e.ctrlKey) && e.key === 'b') {
      e.preventDefault();
      applyToolbar('bold');
    }
    if ((e.metaKey || e.ctrlKey) && e.key === 'i') {
      e.preventDefault();
      applyToolbar('italic');
    }
  };

  const handleTextareaKeyDown = (e) => {
    if (e.key === 'Tab') {
      e.preventDefault();
      const ta = e.target;
      const start = ta.selectionStart;
      const end = ta.selectionEnd;
      const newValue = content.slice(0, start) + '  ' + content.slice(end);
      setContent(newValue);
      setTimeout(() => { ta.selectionStart = ta.selectionEnd = start + 2; }, 0);
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
        <>
          <div className="md-toolbar" role="toolbar" aria-label="Markdown formatting">
            {TOOLBAR_ACTIONS.map(a =>
              a.sep ? (
                <span key={a.key} className="md-toolbar__sep" />
              ) : (
                <button
                  key={a.key}
                  className={`md-toolbar__btn${a.className ? ' ' + a.className : ''}`}
                  title={a.title}
                  onClick={() => applyToolbar(a.key)}
                  type="button"
                >
                  {a.label}
                </button>
              )
            )}
          </div>
          <textarea
            ref={textareaRef}
            className="textarea"
            placeholder="Write your post in Markdown..."
            value={content}
            onChange={e => setContent(e.target.value)}
            onKeyDown={handleTextareaKeyDown}
          />
          <div className="editor-wordcount">
            {wordCount > 0 && (
              <span className="muted">
                {wordCount.toLocaleString()} word{wordCount !== 1 ? 's' : ''} · ~{Math.max(1, Math.ceil(wordCount / 200))} min read
              </span>
            )}
          </div>
        </>
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
        {hasChanges && <span className="editor-unsaved">● Unsaved changes</span>}
      </div>
    </div>
  );
}
