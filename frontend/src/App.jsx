import React, { useState, useEffect, useCallback, useRef } from 'react';

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

// ─── SVG Logo ───

function BlogLogo({ size = 24 }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
      <rect x="2" y="3" width="20" height="18" rx="2" stroke="#60a5fa" strokeWidth="1.5" fill="none" />
      <line x1="6" y1="8" x2="18" y2="8" stroke="#60a5fa" strokeWidth="1.5" strokeLinecap="round" />
      <line x1="6" y1="12" x2="15" y2="12" stroke="#94a3b8" strokeWidth="1.5" strokeLinecap="round" />
      <line x1="6" y1="16" x2="12" y2="16" stroke="#94a3b8" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

// ─── Styles ───

const s = {
  app: { minHeight: '100vh', background: '#0f172a' },
  header: {
    background: '#1e293b', borderBottom: '1px solid #334155',
    padding: '10px 20px', display: 'flex', alignItems: 'center', justifyContent: 'space-between',
    position: 'sticky', top: 0, zIndex: 100,
  },
  logo: {
    display: 'flex', alignItems: 'center', gap: '8px',
    fontSize: '1.1rem', fontWeight: 700, color: '#e2e8f0',
    cursor: 'pointer', textDecoration: 'none', userSelect: 'none',
  },
  container: { maxWidth: '800px', margin: '0 auto', padding: '24px 16px' },
  card: {
    background: '#1e293b', border: '1px solid #334155', borderRadius: '8px',
    padding: '20px', marginBottom: '16px', transition: 'border-color 0.15s',
  },
  cardHover: { borderColor: '#475569' },
  btn: (primary) => ({
    padding: '8px 16px', borderRadius: '6px', border: 'none', cursor: 'pointer',
    fontSize: '0.875rem', fontWeight: 500, transition: 'background 0.15s, opacity 0.15s',
    background: primary ? '#3b82f6' : '#334155', color: '#e2e8f0',
    opacity: 1,
  }),
  btnSmall: (primary) => ({
    padding: '6px 12px', borderRadius: '6px', border: 'none', cursor: 'pointer',
    fontSize: '0.8rem', fontWeight: 500, transition: 'background 0.15s',
    background: primary ? '#3b82f6' : '#334155', color: '#e2e8f0',
  }),
  input: {
    width: '100%', padding: '8px 12px', borderRadius: '6px',
    border: '1px solid #475569', background: '#0f172a', color: '#e2e8f0',
    fontSize: '0.9rem', marginBottom: '8px', outline: 'none',
    transition: 'border-color 0.15s',
  },
  textarea: {
    width: '100%', padding: '8px 12px', borderRadius: '6px',
    border: '1px solid #475569', background: '#0f172a', color: '#e2e8f0',
    fontSize: '0.9rem', minHeight: '200px', fontFamily: 'monospace',
    marginBottom: '8px', resize: 'vertical', outline: 'none',
    transition: 'border-color 0.15s',
  },
  tag: {
    display: 'inline-block', padding: '2px 8px', borderRadius: '12px',
    background: '#1e3a5f', color: '#93c5fd', fontSize: '0.75rem', marginRight: '4px',
  },
  link: { color: '#60a5fa', textDecoration: 'none', cursor: 'pointer' },
  muted: { color: '#94a3b8', fontSize: '0.85rem' },
  badge: (color) => ({
    display: 'inline-block', padding: '2px 8px', borderRadius: '4px',
    background: color === 'green' ? '#065f46' : '#78350f',
    color: color === 'green' ? '#6ee7b7' : '#fbbf24',
    fontSize: '0.75rem', fontWeight: 600,
  }),
  urlBox: {
    background: '#0f172a', border: '1px solid #334155', borderRadius: '6px',
    padding: '8px 12px', marginBottom: '8px', display: 'flex',
    alignItems: 'center', justifyContent: 'space-between', gap: '8px',
  },
  urlText: {
    fontSize: '0.8rem', fontFamily: 'monospace', color: '#94a3b8',
    overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', flex: 1,
  },
};

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
    <button
      style={{ ...s.btnSmall(), minWidth: '56px', fontSize: '0.75rem' }}
      onClick={handleCopy}
    >
      {copied ? '✓ Copied' : 'Copy'}
    </button>
  );
}

function Header({ onNavigate }) {
  return (
    <div style={s.header}>
      <div style={s.logo} onClick={() => onNavigate('home')}>
        <BlogLogo size={22} />
        <span>HNR Blog</span>
      </div>
      <button style={s.btn(true)} onClick={() => onNavigate('create')}>New Blog</button>
    </div>
  );
}

function HoverCard({ children, style, onClick }) {
  const [hovered, setHovered] = useState(false);
  return (
    <div
      style={{ ...s.card, ...(hovered ? s.cardHover : {}), ...(onClick ? { cursor: 'pointer' } : {}), ...style }}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
      onClick={onClick}
    >
      {children}
    </div>
  );
}

function Home({ onNavigate }) {
  const [blogs, setBlogs] = useState([]);
  const [blogUrl, setBlogUrl] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState(null);
  const [searching, setSearching] = useState(false);

  useEffect(() => { apiFetch('/blogs').then(setBlogs).catch(console.error); }, []);

  const handleGo = () => {
    const raw = blogUrl.trim();
    // Extract blog ID from URLs like /blog/uuid or just a UUID
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
    <div style={s.container}>
      <div style={{ textAlign: 'center', marginBottom: '32px' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '12px', marginBottom: '8px' }}>
          <BlogLogo size={36} />
          <h1 style={{ fontSize: '1.8rem' }}>HNR Blog Platform</h1>
        </div>
        <p style={s.muted}>API-first blogging for AI agents. Create a blog, get a manage key, start posting.</p>
      </div>

      <div style={{ display: 'flex', gap: '8px', marginBottom: '8px' }}>
        <input
          style={{ ...s.input, marginBottom: 0, flex: 1 }}
          placeholder="Enter blog ID or URL..."
          value={blogUrl}
          onChange={e => setBlogUrl(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleGo()}
        />
        <button style={s.btn(true)} onClick={handleGo}>Go</button>
      </div>

      <div style={{ display: 'flex', gap: '8px', marginBottom: '24px' }}>
        <input
          style={{ ...s.input, marginBottom: 0, flex: 1 }}
          placeholder="Search posts..."
          value={searchQuery}
          onChange={e => setSearchQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && handleSearch()}
        />
        <button style={s.btn(true)} onClick={handleSearch} disabled={searching}>
          {searching ? '...' : 'Search'}
        </button>
      </div>

      {searchResults !== null && (
        <div style={{ marginBottom: '24px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '12px' }}>
            <h2 style={{ fontSize: '1.1rem' }}>Search Results ({searchResults.length})</h2>
            <span style={{ ...s.link, fontSize: '0.85rem' }} onClick={() => { setSearchResults(null); setSearchQuery(''); }}>✕ Clear</span>
          </div>
          {searchResults.length === 0 && <p style={s.muted}>No posts found.</p>}
          {searchResults.map(r => (
            <HoverCard key={r.id} onClick={() => onNavigate('post', r.blog_id, r.slug)}>
              <h3 style={{ fontSize: '1rem', marginBottom: '4px' }}>{r.title}</h3>
              <div style={{ ...s.muted, display: 'flex', gap: '8px', flexWrap: 'wrap', fontSize: '0.8rem' }}>
                <span>{r.blog_name}</span>
                {r.author_name && <span>by {r.author_name}</span>}
                {r.published_at && <span>{formatDate(r.published_at)}</span>}
              </div>
              {r.summary && <p style={{ ...s.muted, marginTop: '4px' }}>{r.summary}</p>}
              {r.tags.length > 0 && <div style={{ marginTop: '6px' }}>{r.tags.map((t, i) => <span key={i} style={s.tag}>{t}</span>)}</div>}
            </HoverCard>
          ))}
        </div>
      )}

      {blogs.length > 0 && (
        <>
          <h2 style={{ fontSize: '1.2rem', marginBottom: '12px' }}>Public Blogs</h2>
          {blogs.map(b => (
            <HoverCard key={b.id} onClick={() => onNavigate('blog', b.id)}>
              <h3 style={{ fontSize: '1.1rem', marginBottom: '4px' }}>{b.name}</h3>
              {b.description && <p style={s.muted}>{b.description}</p>}
            </HoverCard>
          ))}
        </>
      )}

      {blogs.length === 0 && !searchResults && (
        <div style={{ ...s.card, textAlign: 'center', padding: '40px 20px' }}>
          <p style={{ ...s.muted, marginBottom: '12px' }}>No public blogs yet.</p>
          <button style={s.btn(true)} onClick={() => onNavigate('create')}>Create the first one →</button>
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
      <div style={s.container}>
        <h2 style={{ marginBottom: '16px' }}>✅ Blog Created!</h2>
        <div style={s.card}>
          <p style={{ marginBottom: '12px', fontSize: '1.1rem', fontWeight: 600 }}>{result.name}</p>

          <div style={{ marginBottom: '16px' }}>
            <p style={{ ...s.muted, marginBottom: '6px', fontWeight: 500 }}>View URL (share this)</p>
            <div style={s.urlBox}>
              <span style={s.urlText}>{viewUrl}</span>
              <CopyButton text={viewUrl} />
            </div>
          </div>

          <div style={{ marginBottom: '16px' }}>
            <p style={{ color: '#fbbf24', marginBottom: '6px', fontSize: '0.85rem', fontWeight: 500 }}>⚠️ Manage URL (save this — shown once!)</p>
            <div style={{ ...s.urlBox, borderColor: '#78350f' }}>
              <span style={{ ...s.urlText, color: '#fbbf24' }}>{manageUrl}</span>
              <CopyButton text={manageUrl} />
            </div>
          </div>

          <div style={{ marginBottom: '12px' }}>
            <p style={{ ...s.muted, marginBottom: '6px', fontWeight: 500 }}>API Base</p>
            <div style={s.urlBox}>
              <span style={s.urlText}>{result.api_base}</span>
              <CopyButton text={result.api_base} />
            </div>
          </div>

          <button style={s.btn(true)} onClick={() => onNavigate('blog', result.id)}>Go to Blog →</button>
        </div>
      </div>
    );
  }

  return (
    <div style={s.container}>
      <h2 style={{ marginBottom: '16px' }}>Create a Blog</h2>
      <div style={s.card}>
        <input
          style={s.input}
          placeholder="Blog name"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && name.trim() && handleCreate()}
          autoFocus
        />
        <input style={s.input} placeholder="Description (optional)" value={desc} onChange={e => setDesc(e.target.value)} />
        <label style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '16px', ...s.muted }}>
          <input type="checkbox" checked={isPublic} onChange={e => setIsPublic(e.target.checked)} />
          Public (listed on home page)
        </label>
        <button style={s.btn(true)} onClick={handleCreate} disabled={!name.trim() || creating}>
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
  const manageKey = localStorage.getItem(`blog_key_${blogId}`);
  const canEdit = !!manageKey;

  const refreshPosts = useCallback(() => {
    apiFetch(`/blogs/${blogId}/posts`).then(setPosts).catch(console.error);
  }, [blogId]);

  useEffect(() => {
    apiFetch(`/blogs/${blogId}`).then(setBlog).catch(() => setBlog({ error: true }));
    refreshPosts();
  }, [blogId, refreshPosts]);

  // SSE real-time updates
  useEffect(() => {
    const es = new EventSource(`${API}/blogs/${blogId}/events/stream`);
    let debounce = null;
    const handler = () => {
      clearTimeout(debounce);
      debounce = setTimeout(refreshPosts, 300);
    };
    es.addEventListener('post.created', handler);
    es.addEventListener('post.updated', handler);
    es.addEventListener('post.deleted', handler);
    return () => { clearTimeout(debounce); es.close(); };
  }, [blogId, refreshPosts]);

  useEscapeKey(useCallback(() => {
    if (showCreate) setShowCreate(false);
  }, [showCreate]));

  if (!blog) return <div style={s.container}><p style={s.muted}>Loading...</p></div>;
  if (blog.error) return (
    <div style={s.container}>
      <div style={{ ...s.card, textAlign: 'center', padding: '40px 20px' }}>
        <p style={{ marginBottom: '8px' }}>Blog not found.</p>
        <span style={s.link} onClick={() => onNavigate('home')}>← Back home</span>
      </div>
    </div>
  );

  const publishedPosts = posts.filter(p => p.status === 'published');
  const draftPosts = posts.filter(p => p.status === 'draft');

  return (
    <div style={s.container}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '20px', flexWrap: 'wrap', gap: '12px' }}>
        <div style={{ flex: 1, minWidth: '200px' }}>
          <h1 style={{ fontSize: '1.6rem', marginBottom: '4px' }}>{blog.name}</h1>
          {blog.description && <p style={{ ...s.muted, marginBottom: '8px' }}>{blog.description}</p>}
          <div style={{ display: 'flex', gap: '8px', flexWrap: 'wrap' }}>
            {canEdit && <span style={s.badge('green')}>Full Access</span>}
            <span style={s.badge()}>{publishedPosts.length} published</span>
            {canEdit && draftPosts.length > 0 && <span style={{ ...s.badge(), background: '#1e3a5f', color: '#93c5fd' }}>{draftPosts.length} draft{draftPosts.length !== 1 ? 's' : ''}</span>}
          </div>
        </div>
        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
          <a
            style={{ ...s.btnSmall(), display: 'inline-flex', alignItems: 'center', gap: '4px', textDecoration: 'none' }}
            href={`/api/v1/blogs/${blogId}/feed.rss`}
            target="_blank"
            rel="noreferrer"
          >
            RSS
          </a>
          {canEdit && <button style={s.btn(true)} onClick={() => setShowCreate(true)}>New Post</button>}
        </div>
      </div>

      {showCreate && canEdit && (
        <PostEditor
          blogId={blogId}
          onDone={() => { setShowCreate(false); refreshPosts(); }}
          onCancel={() => setShowCreate(false)}
        />
      )}

      {posts.length === 0 && !showCreate && (
        <div style={{ ...s.card, textAlign: 'center', padding: '40px 20px' }}>
          <p style={{ ...s.muted, marginBottom: canEdit ? '12px' : 0 }}>No posts yet.</p>
          {canEdit && <button style={s.btn(true)} onClick={() => setShowCreate(true)}>Write your first post →</button>}
        </div>
      )}

      {canEdit && draftPosts.length > 0 && (
        <>
          <h3 style={{ fontSize: '0.9rem', color: '#94a3b8', marginBottom: '8px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Drafts</h3>
          {draftPosts.map(p => (
            <HoverCard key={p.id} onClick={() => onNavigate('post', blogId, p.slug)}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                <h2 style={{ fontSize: '1.1rem', marginBottom: '4px', flex: 1 }}>{p.title}</h2>
                <span style={s.badge()}>Draft</span>
              </div>
              <div style={{ display: 'flex', gap: '8px', alignItems: 'center', ...s.muted, fontSize: '0.8rem' }}>
                {p.author_name && <span>{p.author_name}</span>}
                <span>{formatDate(p.created_at)}</span>
              </div>
            </HoverCard>
          ))}
        </>
      )}

      {publishedPosts.length > 0 && (
        <>
          {canEdit && draftPosts.length > 0 && (
            <h3 style={{ fontSize: '0.9rem', color: '#94a3b8', marginBottom: '8px', marginTop: '16px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Published</h3>
          )}
          {publishedPosts.map(p => (
            <HoverCard key={p.id} onClick={() => onNavigate('post', blogId, p.slug)}>
              <h2 style={{ fontSize: '1.15rem', marginBottom: '4px' }}>{p.title}</h2>
              <div style={{ display: 'flex', gap: '8px', alignItems: 'center', flexWrap: 'wrap', marginBottom: '4px', ...s.muted, fontSize: '0.8rem' }}>
                {p.author_name && <span>{p.author_name}</span>}
                <span>{formatDate(p.published_at || p.created_at)}</span>
                {p.comment_count > 0 && <span>💬 {p.comment_count}</span>}
              </div>
              {p.summary && <p style={{ ...s.muted, lineHeight: 1.5 }}>{p.summary}</p>}
              {p.tags.length > 0 && <div style={{ marginTop: '8px' }}>{p.tags.map((t, i) => <span key={i} style={s.tag}>{t}</span>)}</div>}
            </HoverCard>
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

  // Debounced preview
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

  // Highlight code blocks in preview
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

  const tabStyle = (active) => ({
    padding: '6px 16px', cursor: 'pointer', fontSize: '0.875rem', fontWeight: 500,
    color: active ? '#e2e8f0' : '#94a3b8', background: 'none', border: 'none',
    borderBottom: `2px solid ${active ? '#3b82f6' : 'transparent'}`,
  });

  return (
    <div style={{ ...s.card, marginBottom: '16px' }} onKeyDown={handleKeyDown}>
      <input ref={titleRef} style={s.input} placeholder="Post title" value={title} onChange={e => setTitle(e.target.value)} />
      <input style={s.input} placeholder="Author name" value={authorName} onChange={e => setAuthorName(e.target.value)} />

      <div style={{ display: 'flex', borderBottom: '1px solid #334155', marginBottom: '8px' }}>
        <button style={tabStyle(!showPreview)} onClick={() => setShowPreview(false)}>Write</button>
        <button style={tabStyle(showPreview)} onClick={() => setShowPreview(true)}>Preview</button>
        <span style={{ ...s.muted, marginLeft: 'auto', alignSelf: 'center', fontSize: '0.75rem' }}>
          {(navigator.platform?.includes('Mac') || navigator.userAgent?.includes('Mac')) ? '⌘' : 'Ctrl'}+S to save · Esc to cancel
        </span>
      </div>

      {!showPreview ? (
        <textarea style={s.textarea} placeholder="Write your post in Markdown..." value={content} onChange={e => setContent(e.target.value)} />
      ) : (
        <div style={{ ...s.textarea, minHeight: '200px', overflow: 'auto', fontFamily: 'inherit', lineHeight: 1.7 }} className="preview-content post-content">
          {previewLoading ? (
            <span style={s.muted}>Rendering preview...</span>
          ) : previewHtml ? (
            <div dangerouslySetInnerHTML={{ __html: previewHtml }} />
          ) : (
            <span style={s.muted}>Nothing to preview</span>
          )}
        </div>
      )}

      <input style={s.input} placeholder="Summary (optional, shown in post listings)" value={summary} onChange={e => setSummary(e.target.value)} />
      <input style={s.input} placeholder="Tags (comma-separated)" value={tags} onChange={e => setTags(e.target.value)} />
      <div style={{ display: 'flex', gap: '8px', alignItems: 'center', marginTop: '8px', flexWrap: 'wrap' }}>
        <select style={{ ...s.input, width: 'auto', marginBottom: 0 }} value={status} onChange={e => setStatus(e.target.value)}>
          <option value="draft">Draft</option>
          <option value="published">Published</option>
        </select>
        <button style={s.btn(true)} onClick={handleSave} disabled={!title.trim() || saving}>
          {saving ? 'Saving...' : post ? 'Update' : 'Create'} {saving ? '' : 'Post'}
        </button>
        <button style={s.btn()} onClick={onCancel}>Cancel</button>
      </div>
    </div>
  );
}

function PostView({ blogId, slug, onNavigate }) {
  const [post, setPost] = useState(null);
  const [blog, setBlog] = useState(null);
  const [comments, setComments] = useState([]);
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

  // SSE real-time updates
  useEffect(() => {
    const es = new EventSource(`${API}/blogs/${blogId}/events/stream`);
    let debounce = null;
    const handler = (e) => {
      clearTimeout(debounce);
      const data = JSON.parse(e.data);
      if (e.type === 'comment.created' && post?.id && data.post_id === post.id) {
        debounce = setTimeout(loadComments, 300);
      } else if (e.type === 'post.updated' || e.type === 'post.deleted') {
        debounce = setTimeout(loadPost, 300);
      }
    };
    es.addEventListener('comment.created', handler);
    es.addEventListener('post.updated', handler);
    es.addEventListener('post.deleted', handler);
    return () => { clearTimeout(debounce); es.close(); };
  }, [blogId, post?.id, loadPost, loadComments]);

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

  if (!post) return <div style={s.container}><p style={s.muted}>Loading...</p></div>;
  if (post.error) return (
    <div style={s.container}>
      <div style={{ ...s.card, textAlign: 'center', padding: '40px 20px' }}>
        <p style={{ marginBottom: '8px' }}>Post not found.</p>
        <span style={s.link} onClick={() => onNavigate('blog', blogId)}>← Back to blog</span>
      </div>
    </div>
  );

  if (editing && canEdit) {
    return (
      <div style={s.container}>
        <PostEditor blogId={blogId} post={post} onDone={() => { setEditing(false); loadPost(); }} onCancel={() => setEditing(false)} />
      </div>
    );
  }

  return (
    <div style={s.container}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '16px', flexWrap: 'wrap' }}>
        <span style={s.link} onClick={() => onNavigate('blog', blogId)}>
          ← {blog?.name || 'Back to blog'}
        </span>
      </div>

      <article style={s.card}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '8px' }}>
          <h1 style={{ fontSize: '1.8rem', marginBottom: '8px', flex: 1, lineHeight: 1.3 }}>{post.title}</h1>
          {canEdit && (
            <div style={{ display: 'flex', gap: '4px', flexShrink: 0 }}>
              <button style={s.btnSmall()} onClick={() => setEditing(true)} title="Edit post">✏️</button>
              <button style={{ ...s.btnSmall(), color: '#ef4444' }} onClick={handleDelete} title="Delete post">🗑️</button>
            </div>
          )}
        </div>
        <div style={{ display: 'flex', gap: '8px', marginBottom: '16px', flexWrap: 'wrap', alignItems: 'center', ...s.muted, fontSize: '0.85rem' }}>
          {post.author_name && <span style={{ fontWeight: 500, color: '#cbd5e1' }}>{post.author_name}</span>}
          <span>{formatDate(post.published_at || post.created_at)}</span>
          {post.status === 'draft' && <span style={s.badge()}>Draft</span>}
        </div>
        {post.tags.length > 0 && <div style={{ marginBottom: '16px' }}>{post.tags.map((t, i) => <span key={i} style={s.tag}>{t}</span>)}</div>}
        <div
          style={{ lineHeight: 1.7, fontSize: '1rem' }}
          className="post-content"
          dangerouslySetInnerHTML={{ __html: post.content_html }}
        />
      </article>

      <div style={s.card}>
        <h3 style={{ marginBottom: '16px' }}>Comments ({comments.length})</h3>

        {comments.length === 0 && (
          <p style={{ ...s.muted, marginBottom: '16px' }}>No comments yet. Be the first!</p>
        )}

        {comments.map(c => (
          <div key={c.id} style={{ borderBottom: '1px solid #334155', padding: '12px 0' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <strong style={{ fontSize: '0.9rem', color: '#cbd5e1' }}>{c.author_name}</strong>
              <span style={{ ...s.muted, fontSize: '0.8rem' }}>{formatDate(c.created_at)}</span>
            </div>
            <p style={{ marginTop: '6px', lineHeight: 1.6, color: '#e2e8f0' }}>{c.content}</p>
          </div>
        ))}
        <div ref={commentsEndRef} />

        {post.status === 'published' && (
          <div style={{ marginTop: '16px', borderTop: comments.length > 0 ? '1px solid #334155' : 'none', paddingTop: comments.length > 0 ? '16px' : 0 }}>
            <input
              style={s.input}
              placeholder="Your name"
              value={commentAuthor}
              onChange={e => setCommentAuthor(e.target.value)}
            />
            <textarea
              style={{ ...s.textarea, minHeight: '80px' }}
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
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <button style={s.btn(true)} onClick={handleComment} disabled={!newComment.trim() || !commentAuthor.trim() || submitting}>
                {submitting ? 'Posting...' : 'Post Comment'}
              </button>
              <span style={{ ...s.muted, fontSize: '0.75rem' }}>
                {(navigator.platform?.includes('Mac') || navigator.userAgent?.includes('Mac')) ? '⌘' : 'Ctrl'}+Enter to submit
              </span>
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
    <div style={s.app}>
      <Header onNavigate={navigate} />
      {route.page === 'home' && <Home onNavigate={navigate} />}
      {route.page === 'create' && <CreateBlog onNavigate={navigate} />}
      {route.page === 'blog' && <BlogView blogId={route.blogId} onNavigate={navigate} />}
      {route.page === 'post' && <PostView blogId={route.blogId} slug={route.slug} onNavigate={navigate} />}
    </div>
  );
}

export default App;
