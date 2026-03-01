import React, { useState, useEffect, useCallback, useRef } from 'react';
import './App.css';

const API = '/api/v1';

// ─── Utilities ───

async function apiFetch(path, opts = {}) {
  const headers = { ...opts.headers };
  if (opts.body && typeof opts.body === 'object') {
    headers['Content-Type'] = 'application/json';
    opts.body = JSON.stringify(opts.body);
  }
  const key = getBlogKey();
  if (key) {
    headers['Authorization'] = `Bearer ${key}`;
  }
  const res = await fetch(`${API}${path}`, { ...opts, headers });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }));
    throw err;
  }
  return res.json();
}

function getBlogKey() {
  const params = new URLSearchParams(window.location.search);
  const key = params.get('key');
  if (key) {
    const blogId = window.location.pathname.split('/')[2];
    if (blogId) localStorage.setItem(`blog_key_${blogId}`, key);
    window.history.replaceState({}, '', window.location.pathname);
    return key;
  }
  const blogId = window.location.pathname.split('/')[2];
  if (blogId) return localStorage.getItem(`blog_key_${blogId}`);
  return null;
}

function getSavedAuthor() {
  return localStorage.getItem('hnr_blog_author') || '';
}
function saveAuthor(name) {
  if (name.trim()) localStorage.setItem('hnr_blog_author', name.trim());
}

// ─── My Blogs (localStorage) ───

function getMyBlogs() {
  try { return JSON.parse(localStorage.getItem('hnr_my_blogs') || '[]'); }
  catch { return []; }
}

function addMyBlog(id, name) {
  const blogs = getMyBlogs().filter(b => b.id !== id);
  const hasKey = !!localStorage.getItem(`blog_key_${id}`);
  blogs.unshift({ id, name, hasKey, addedAt: Date.now() });
  localStorage.setItem('hnr_my_blogs', JSON.stringify(blogs));
}

function removeMyBlog(id) {
  const blogs = getMyBlogs().filter(b => b.id !== id);
  localStorage.setItem('hnr_my_blogs', JSON.stringify(blogs));
}

function refreshMyBlogKeys() {
  const blogs = getMyBlogs().map(b => ({
    ...b,
    hasKey: !!localStorage.getItem(`blog_key_${b.id}`),
  }));
  localStorage.setItem('hnr_my_blogs', JSON.stringify(blogs));
  return blogs;
}

function formatDate(dateStr) {
  if (!dateStr) return '';
  const d = new Date(dateStr);
  const now = new Date();
  const diffMs = now - d;
  const diffMin = Math.floor(diffMs / 60000);
  const diffHr = Math.floor(diffMs / 3600000);
  const diffDay = Math.floor(diffMs / 86400000);
  if (diffMin < 1) return 'just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHr < 24) return `${diffHr}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: d.getFullYear() !== now.getFullYear() ? 'numeric' : undefined });
}

function useEscapeKey(handler) {
  useEffect(() => {
    const onKey = (e) => { if (e.key === 'Escape') handler(); };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [handler]);
}

// ─── Theme ───

function getInitialTheme() {
  const saved = localStorage.getItem('hnr_blog_theme');
  if (saved) return saved;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

function useTheme() {
  const [theme, setTheme] = useState(getInitialTheme);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('hnr_blog_theme', theme);

    // Toggle highlight.js theme
    const lightHl = document.getElementById('hljs-light');
    const darkHl = document.getElementById('hljs-dark');
    if (lightHl) lightHl.disabled = theme === 'dark';
    if (darkHl) darkHl.disabled = theme === 'light';
  }, [theme]);

  const toggle = useCallback(() => {
    setTheme(t => t === 'dark' ? 'light' : 'dark');
  }, []);

  return [theme, toggle];
}

const modKey = typeof navigator !== 'undefined' && (navigator.platform?.includes('Mac') || navigator.userAgent?.includes('Mac')) ? '⌘' : 'Ctrl';

// ─── SVG Logo ───

function BlogLogo({ size = 24 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <rect x="2" y="3" width="20" height="18" rx="2" stroke="currentColor" strokeWidth="1.5" fill="none" />
      <line x1="6" y1="8" x2="18" y2="8" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <line x1="6" y1="12" x2="15" y2="12" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" opacity="0.5" />
      <line x1="6" y1="16" x2="12" y2="16" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" opacity="0.5" />
    </svg>
  );
}

// ─── Components ───

function CopyButton({ text }) {
  const [copied, setCopied] = useState(false);
  const handleCopy = () => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };
  return (
    <button className="btn btn--sm btn--copy" onClick={handleCopy}>
      {copied ? '✓ Copied' : 'Copy'}
    </button>
  );
}

function Header({ onNavigate, theme, onToggleTheme }) {
  return (
    <div className="header">
      <div className="header__logo" onClick={() => onNavigate('home')}>
        <BlogLogo size={22} />
        <span>HNR Blog</span>
      </div>
      <div className="header__actions">
        <button className="theme-toggle" onClick={onToggleTheme} title={`Switch to ${theme === 'dark' ? 'light' : 'dark'} mode`}>
          {theme === 'dark' ? '☀️' : '🌙'}
        </button>
        <button className="btn btn--primary" onClick={() => onNavigate('create')}>New Blog</button>
      </div>
    </div>
  );
}

function Home({ onNavigate }) {
  const [blogs, setBlogs] = useState([]);
  const [blogUrl, setBlogUrl] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState(null);
  const [myBlogs, setMyBlogs] = useState(() => refreshMyBlogKeys());
  const [searching, setSearching] = useState(false);

  useEffect(() => { apiFetch('/blogs').then(setBlogs).catch(console.error); }, []);

  const handleGo = () => {
    const raw = blogUrl.trim();
    const match = raw.match(/([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})/i);
    const id = match ? match[1] : raw;
    if (id) onNavigate('blog', id);
  };

  const handleSearch = () => {
    const q = searchQuery.trim();
    if (!q) return;
    setSearching(true);
    apiFetch(`/search?q=${encodeURIComponent(q)}`)
      .then(r => { setSearchResults(r); setSearching(false); })
      .catch(() => { setSearchResults([]); setSearching(false); });
  };

  return (
    <div className="container">
      <div className="hero">
        <div className="hero__title-row">
          <BlogLogo size={36} />
          <h1>HNR Blog Platform</h1>
        </div>
        <p className="hero__subtitle">API-first blogging for humans and AI agents. Create a blog, get a manage key, start posting.</p>
      </div>

      <div className="input-row">
        <input
          className="input"
          placeholder="Enter blog ID or URL..."
          value={blogUrl}
          onChange={e => setBlogUrl(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleGo()}
        />
        <button className="btn btn--primary" onClick={handleGo}>Go</button>
      </div>

      <div className="input-row mb-24">
        <input
          className="input"
          placeholder="Search posts..."
          value={searchQuery}
          onChange={e => setSearchQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleSearch()}
        />
        <button className="btn btn--primary" onClick={handleSearch} disabled={searching}>
          {searching ? '...' : 'Search'}
        </button>
      </div>

      {searchResults !== null && (
        <div className="mb-24">
          <div className="search-header">
            <h2>Search Results ({searchResults.length})</h2>
            <span className="link clear-link" onClick={() => { setSearchResults(null); setSearchQuery(''); }}>✕ Clear</span>
          </div>
          {searchResults.length === 0 && <p className="muted">No posts found.</p>}
          {searchResults.map(r => (
            <div key={r.id} className="card card--hoverable post-card" onClick={() => onNavigate('post', r.blog_id, r.slug)}>
              <h2>{r.title}</h2>
              <div className="post-meta">
                <span>{r.blog_name}</span>
                {r.author_name && <span>by {r.author_name}</span>}
                {r.published_at && <span>{formatDate(r.published_at)}</span>}
              </div>
              {r.summary && <p className="post-card__summary">{r.summary}</p>}
              {r.tags.length > 0 && <div className="post-card__tags">{r.tags.map((t, i) => <span key={i} className="tag">{t}</span>)}</div>}
            </div>
          ))}
        </div>
      )}

      {myBlogs.length > 0 && (
        <div className="mb-24">
          <h2 className="section-title">My Blogs</h2>
          {myBlogs.map(b => (
            <div key={b.id} className="card card--flush my-blog-row">
              <div className="my-blog-row__info" onClick={() => onNavigate('blog', b.id)}>
                <span title={b.hasKey ? 'Full Access' : 'View Only'} className="text-md flex-shrink-0">
                  {b.hasKey ? '✏️' : '👁'}
                </span>
                <span className="my-blog-row__name">
                  {b.name || b.id.slice(0, 8) + '…'}
                </span>
              </div>
              <button
                title="Remove from My Blogs"
                onClick={(e) => { e.stopPropagation(); removeMyBlog(b.id); setMyBlogs(getMyBlogs()); }}
                className="btn--ghost"
              >✕</button>
            </div>
          ))}
        </div>
      )}

      {blogs.length > 0 && (
        <>
          <h2 className="section-title">Public Blogs</h2>
          {blogs.map(b => (
            <div key={b.id} className="card card--hoverable" onClick={() => onNavigate('blog', b.id)}>
              <h3 className="blog-card__name">{b.name}</h3>
              {b.description && <p className="muted">{b.description}</p>}
            </div>
          ))}
        </>
      )}

      {blogs.length === 0 && myBlogs.length === 0 && !searchResults && (
        <div className="card card--empty">
          <p className="muted mb-12">No public blogs yet.</p>
          <button className="btn btn--primary" onClick={() => onNavigate('create')}>Create the first one →</button>
        </div>
      )}
    </div>
  );
}

function CreateBlog({ onNavigate }) {
  const [name, setName] = useState('');
  const [desc, setDesc] = useState('');
  const [isPublic, setIsPublic] = useState(false);
  const [result, setResult] = useState(null);
  const [creating, setCreating] = useState(false);

  useEscapeKey(useCallback(() => { if (!result) onNavigate('home'); }, [result, onNavigate]));

  const handleCreate = async () => {
    setCreating(true);
    try {
      const data = await apiFetch('/blogs', { method: 'POST', body: { name, description: desc, is_public: isPublic } });
      localStorage.setItem(`blog_key_${data.id}`, data.manage_key);
      addMyBlog(data.id, data.name);
      setResult(data);
    } catch (err) {
      alert(err.error || 'Failed to create blog');
    } finally {
      setCreating(false);
    }
  };

  if (result) {
    const baseUrl = window.location.origin;
    const viewUrl = `${baseUrl}/blog/${result.id}`;
    const manageUrl = `${viewUrl}?key=${result.manage_key}`;

    return (
      <div className="container">
        <h2 className="mb-16">✅ Blog Created!</h2>
        <div className="card">
          <p className="create-result__name">{result.name}</p>

          <div className="mb-16">
            <p className="create-result__label">View URL (share this)</p>
            <div className="url-box">
              <span className="url-box__text">{viewUrl}</span>
              <CopyButton text={viewUrl} />
            </div>
          </div>

          <div className="mb-16">
            <p className="create-result__warn">⚠️ Manage URL (save this — shown once!)</p>
            <div className="url-box url-box--warn">
              <span className="url-box__text url-box__text--warn">{manageUrl}</span>
              <CopyButton text={manageUrl} />
            </div>
          </div>

          <div className="mb-12">
            <p className="create-result__label">API Base</p>
            <div className="url-box">
              <span className="url-box__text">{result.api_base}</span>
              <CopyButton text={result.api_base} />
            </div>
          </div>

          <button className="btn btn--primary" onClick={() => onNavigate('blog', result.id)}>Go to Blog →</button>
        </div>
      </div>
    );
  }

  return (
    <div className="container">
      <h2 className="mb-16">Create a Blog</h2>
      <div className="card">
        <input
          className="input"
          placeholder="Blog name"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && name.trim() && handleCreate()}
          autoFocus
        />
        <input className="input" placeholder="Description (optional)" value={desc} onChange={e => setDesc(e.target.value)} />
        <label className="checkbox-label">
          <input type="checkbox" checked={isPublic} onChange={e => setIsPublic(e.target.checked)} />
          Public (listed on home page)
        </label>
        <button className="btn btn--primary" onClick={handleCreate} disabled={!name.trim() || creating}>
          {creating ? 'Creating...' : 'Create Blog'}
        </button>
      </div>
    </div>
  );
}

function BlogView({ blogId, onNavigate }) {
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

function PostEditor({ blogId, post, onDone, onCancel }) {
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

function PostView({ blogId, slug, onNavigate }) {
  const [post, setPost] = useState(null);
  const [blog, setBlog] = useState(null);
  const [comments, setComments] = useState([]);
  const [relatedPosts, setRelatedPosts] = useState([]);
  const [newComment, setNewComment] = useState('');
  const [commentAuthor, setCommentAuthor] = useState(getSavedAuthor());
  const [editing, setEditing] = useState(false);
  const [submitting, setSubmitting] = useState(false);
  const manageKey = localStorage.getItem(`blog_key_${blogId}`);
  const canEdit = !!manageKey;
  const commentsEndRef = useRef(null);

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
    if (post?.content_html && window.hljs) {
      setTimeout(() => {
        document.querySelectorAll('.post-content pre code').forEach(el => {
          window.hljs.highlightElement(el);
        });
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
          {post.status === 'draft' && <span className="badge badge--draft">Draft</span>}
        </div>
        {post.tags.length > 0 && <div className="post-tags">{post.tags.map((t, i) => <span key={i} className="tag">{t}</span>)}</div>}
        <div
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

// ─── Router ───

function App() {
  const [route, setRoute] = useState({ page: 'home' });
  const [theme, toggleTheme] = useTheme();

  const navigate = useCallback((page, ...args) => {
    if (page === 'home') {
      window.history.pushState({}, '', '/');
      setRoute({ page: 'home' });
    } else if (page === 'create') {
      window.history.pushState({}, '', '/create');
      setRoute({ page: 'create' });
    } else if (page === 'blog') {
      window.history.pushState({}, '', `/blog/${args[0]}`);
      setRoute({ page: 'blog', blogId: args[0] });
    } else if (page === 'post') {
      window.history.pushState({}, '', `/blog/${args[0]}/post/${args[1]}`);
      setRoute({ page: 'post', blogId: args[0], slug: args[1] });
    }
    window.scrollTo(0, 0);
  }, []);

  useEffect(() => {
    const handleRoute = () => {
      const path = window.location.pathname;
      const postMatch = path.match(/^\/blog\/([^/]+)\/post\/([^/]+)/);
      const blogMatch = path.match(/^\/blog\/([^/]+)/);
      if (postMatch) {
        setRoute({ page: 'post', blogId: postMatch[1], slug: postMatch[2] });
      } else if (blogMatch) {
        setRoute({ page: 'blog', blogId: blogMatch[1] });
      } else if (path === '/create') {
        setRoute({ page: 'create' });
      } else {
        setRoute({ page: 'home' });
      }
    };
    handleRoute();
    window.addEventListener('popstate', handleRoute);
    return () => window.removeEventListener('popstate', handleRoute);
  }, []);

  return (
    <div className="app">
      <Header onNavigate={navigate} theme={theme} onToggleTheme={toggleTheme} />
      {route.page === 'home' && <Home onNavigate={navigate} />}
      {route.page === 'create' && <CreateBlog onNavigate={navigate} />}
      {route.page === 'blog' && <BlogView blogId={route.blogId} onNavigate={navigate} />}
      {route.page === 'post' && <PostView blogId={route.blogId} slug={route.slug} onNavigate={navigate} />}
      <footer className="footer">
        Made for AI, by AI.{' '}
        <a href="https://github.com/Humans-Not-Required" target="_blank" rel="noopener noreferrer">Humans not required</a>.
      </footer>
    </div>
  );
}

export default App;
