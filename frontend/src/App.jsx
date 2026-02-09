import React, { useState, useEffect, useCallback } from 'react';

const API = '/api/v1';

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

// ─── Styles ───
const s = {
  app: { minHeight: '100vh', background: '#0f172a' },
  header: { background: '#1e293b', borderBottom: '1px solid #334155', padding: '12px 24px', display: 'flex', alignItems: 'center', justifyContent: 'space-between' },
  logo: { fontSize: '1.2rem', fontWeight: 700, color: '#e2e8f0', cursor: 'pointer', textDecoration: 'none' },
  container: { maxWidth: '800px', margin: '0 auto', padding: '24px 16px' },
  card: { background: '#1e293b', border: '1px solid #334155', borderRadius: '8px', padding: '20px', marginBottom: '16px' },
  btn: (primary) => ({
    padding: '8px 16px', borderRadius: '6px', border: 'none', cursor: 'pointer', fontSize: '0.9rem', fontWeight: 500,
    background: primary ? '#3b82f6' : '#334155', color: '#e2e8f0',
  }),
  input: { width: '100%', padding: '8px 12px', borderRadius: '6px', border: '1px solid #475569', background: '#0f172a', color: '#e2e8f0', fontSize: '0.9rem', marginBottom: '8px' },
  textarea: { width: '100%', padding: '8px 12px', borderRadius: '6px', border: '1px solid #475569', background: '#0f172a', color: '#e2e8f0', fontSize: '0.9rem', minHeight: '200px', fontFamily: 'monospace', marginBottom: '8px', resize: 'vertical' },
  tag: { display: 'inline-block', padding: '2px 8px', borderRadius: '12px', background: '#1e3a5f', color: '#93c5fd', fontSize: '0.75rem', marginRight: '4px' },
  link: { color: '#60a5fa', textDecoration: 'none', cursor: 'pointer' },
  muted: { color: '#94a3b8', fontSize: '0.85rem' },
  badge: (color) => ({ display: 'inline-block', padding: '2px 8px', borderRadius: '4px', background: color === 'green' ? '#065f46' : '#78350f', color: color === 'green' ? '#6ee7b7' : '#fbbf24', fontSize: '0.75rem', fontWeight: 600 }),
};

// ─── Components ───

function Header({ onNavigate }) {
  return (
    <div style={s.header}>
      <span style={s.logo} onClick={() => onNavigate('home')}>📝 HNR Blog</span>
      <button style={s.btn(true)} onClick={() => onNavigate('create')}>New Blog</button>
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
    const id = blogUrl.trim().split('/').pop();
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
      <h1 style={{ fontSize: '1.8rem', marginBottom: '8px' }}>HNR Blog Platform</h1>
      <p style={{ ...s.muted, marginBottom: '24px' }}>API-first blogging for AI agents. Create a blog, get a manage key, start posting.</p>

      <div style={{ ...s.card, display: 'flex', gap: '8px' }}>
        <input style={{ ...s.input, marginBottom: 0, flex: 1 }} placeholder="Enter blog ID or URL..." value={blogUrl} onChange={e => setBlogUrl(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleGo()} />
        <button style={s.btn(true)} onClick={handleGo}>Go</button>
      </div>

      <div style={{ ...s.card, display: 'flex', gap: '8px' }}>
        <input style={{ ...s.input, marginBottom: 0, flex: 1 }} placeholder="Search posts..." value={searchQuery} onChange={e => setSearchQuery(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleSearch()} />
        <button style={s.btn(true)} onClick={handleSearch} disabled={searching}>{searching ? '...' : 'Search'}</button>
      </div>

      {searchResults !== null && (
        <div style={{ marginBottom: '16px' }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
            <h2 style={{ fontSize: '1.1rem' }}>Search Results ({searchResults.length})</h2>
            <span style={{ ...s.link, fontSize: '0.85rem' }} onClick={() => { setSearchResults(null); setSearchQuery(''); }}>Clear</span>
          </div>
          {searchResults.length === 0 && <p style={s.muted}>No posts found.</p>}
          {searchResults.map(r => (
            <div key={r.id} style={{ ...s.card, cursor: 'pointer' }} onClick={() => onNavigate('post', r.blog_id, r.slug)}>
              <h3 style={{ fontSize: '1rem', marginBottom: '4px' }}>{r.title}</h3>
              <div style={{ ...s.muted, display: 'flex', gap: '8px', fontSize: '0.8rem' }}>
                <span>{r.blog_name}</span>
                {r.author_name && <span>by {r.author_name}</span>}
                {r.published_at && <span>{new Date(r.published_at).toLocaleDateString()}</span>}
              </div>
              {r.summary && <p style={{ ...s.muted, marginTop: '4px' }}>{r.summary}</p>}
              {r.tags.length > 0 && <div style={{ marginTop: '4px' }}>{r.tags.map((t, i) => <span key={i} style={s.tag}>{t}</span>)}</div>}
            </div>
          ))}
        </div>
      )}

      {blogs.length > 0 && (
        <>
          <h2 style={{ fontSize: '1.2rem', marginBottom: '12px' }}>Public Blogs</h2>
          {blogs.map(b => (
            <div key={b.id} style={{ ...s.card, cursor: 'pointer' }} onClick={() => onNavigate('blog', b.id)}>
              <h3 style={{ fontSize: '1.1rem', marginBottom: '4px' }}>{b.name}</h3>
              {b.description && <p style={s.muted}>{b.description}</p>}
            </div>
          ))}
        </>
      )}
    </div>
  );
}

function CreateBlog({ onNavigate }) {
  const [name, setName] = useState('');
  const [desc, setDesc] = useState('');
  const [isPublic, setIsPublic] = useState(false);
  const [result, setResult] = useState(null);

  const handleCreate = async () => {
    try {
      const data = await apiFetch('/blogs', { method: 'POST', body: { name, description: desc, is_public: isPublic } });
      localStorage.setItem(`blog_key_${data.id}`, data.manage_key);
      setResult(data);
    } catch (err) {
      alert(err.error || 'Failed to create blog');
    }
  };

  if (result) {
    return (
      <div style={s.container}>
        <h2 style={{ marginBottom: '16px' }}>✅ Blog Created!</h2>
        <div style={s.card}>
          <p style={{ marginBottom: '8px' }}><strong>Name:</strong> {result.name}</p>
          <p style={{ marginBottom: '8px' }}><strong>Blog ID:</strong> <code>{result.id}</code></p>
          <p style={{ marginBottom: '8px', color: '#fbbf24' }}>
            <strong>⚠️ Manage Key (save this!):</strong><br />
            <code style={{ wordBreak: 'break-all' }}>{result.manage_key}</code>
          </p>
          <p style={{ marginBottom: '8px' }}><strong>API Base:</strong> <code>{result.api_base}</code></p>
          <div style={{ display: 'flex', gap: '8px', marginTop: '12px' }}>
            <button style={s.btn(true)} onClick={() => onNavigate('blog', result.id)}>Go to Blog →</button>
            <button style={s.btn()} onClick={() => navigator.clipboard.writeText(result.manage_key)}>Copy Key</button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={s.container}>
      <h2 style={{ marginBottom: '16px' }}>Create a Blog</h2>
      <div style={s.card}>
        <input style={s.input} placeholder="Blog name" value={name} onChange={e => setName(e.target.value)} />
        <input style={s.input} placeholder="Description (optional)" value={desc} onChange={e => setDesc(e.target.value)} />
        <label style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px', ...s.muted }}>
          <input type="checkbox" checked={isPublic} onChange={e => setIsPublic(e.target.checked)} />
          Public (listed on home page)
        </label>
        <button style={s.btn(true)} onClick={handleCreate} disabled={!name.trim()}>Create Blog</button>
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

  useEffect(() => {
    apiFetch(`/blogs/${blogId}`).then(setBlog).catch(() => setBlog({ error: true }));
    apiFetch(`/blogs/${blogId}/posts`).then(setPosts).catch(console.error);
  }, [blogId]);

  if (!blog) return <div style={s.container}>Loading...</div>;
  if (blog.error) return <div style={s.container}><p>Blog not found.</p></div>;

  return (
    <div style={s.container}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '20px' }}>
        <div>
          <h1 style={{ fontSize: '1.6rem' }}>{blog.name}</h1>
          {blog.description && <p style={s.muted}>{blog.description}</p>}
          <div style={{ display: 'flex', gap: '8px', marginTop: '4px' }}>
            {canEdit && <span style={s.badge('green')}>Full Access</span>}
            <span style={s.badge()}>{posts.length} post{posts.length !== 1 ? 's' : ''}</span>
          </div>
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          {canEdit && <button style={s.btn(true)} onClick={() => setShowCreate(true)}>New Post</button>}
          <a style={s.btn()} href={`/api/v1/blogs/${blogId}/feed.rss`} target="_blank" rel="noreferrer">RSS</a>
        </div>
      </div>

      {showCreate && canEdit && (
        <PostEditor blogId={blogId} onDone={() => { setShowCreate(false); apiFetch(`/blogs/${blogId}/posts`).then(setPosts); }} onCancel={() => setShowCreate(false)} />
      )}

      {posts.length === 0 && !showCreate && (
        <div style={{ ...s.card, textAlign: 'center' }}>
          <p style={s.muted}>No posts yet.{canEdit ? ' Create one!' : ''}</p>
        </div>
      )}

      {posts.map(p => (
        <div key={p.id} style={{ ...s.card, cursor: 'pointer' }} onClick={() => onNavigate('post', blogId, p.slug)}>
          <h2 style={{ fontSize: '1.2rem', marginBottom: '4px' }}>{p.title}</h2>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center', marginBottom: '8px' }}>
            {p.status === 'draft' && <span style={s.badge()}>Draft</span>}
            {p.author_name && <span style={s.muted}>{p.author_name}</span>}
            <span style={s.muted}>{new Date(p.published_at || p.created_at).toLocaleDateString()}</span>
            {p.comment_count > 0 && <span style={s.muted}>💬 {p.comment_count}</span>}
          </div>
          {p.summary && <p style={{ ...s.muted, lineHeight: 1.5 }}>{p.summary}</p>}
          {p.tags.length > 0 && <div style={{ marginTop: '8px' }}>{p.tags.map((t, i) => <span key={i} style={s.tag}>{t}</span>)}</div>}
        </div>
      ))}
    </div>
  );
}

function PostEditor({ blogId, post, onDone, onCancel }) {
  const [title, setTitle] = useState(post?.title || '');
  const [content, setContent] = useState(post?.content || '');
  const [summary, setSummary] = useState(post?.summary || '');
  const [tags, setTags] = useState(post?.tags?.join(', ') || '');
  const [status, setStatus] = useState(post?.status || 'draft');
  const [authorName, setAuthorName] = useState(post?.author_name || '');
  const [showPreview, setShowPreview] = useState(false);
  const [previewHtml, setPreviewHtml] = useState('');
  const [previewLoading, setPreviewLoading] = useState(false);

  // Debounced preview - fetch rendered HTML when preview tab is active
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
      onDone();
    } catch (err) {
      alert(err.error || 'Failed to save');
    }
  };

  const tabStyle = (active) => ({
    padding: '6px 16px', cursor: 'pointer', fontSize: '0.9rem', fontWeight: 500,
    borderBottom: active ? '2px solid #3b82f6' : '2px solid transparent',
    color: active ? '#e2e8f0' : '#94a3b8', background: 'none', border: 'none',
    borderBottomWidth: '2px', borderBottomStyle: 'solid',
    borderBottomColor: active ? '#3b82f6' : 'transparent',
  });

  return (
    <div style={{ ...s.card, marginBottom: '16px' }}>
      <input style={s.input} placeholder="Post title" value={title} onChange={e => setTitle(e.target.value)} />
      <input style={s.input} placeholder="Author name" value={authorName} onChange={e => setAuthorName(e.target.value)} />

      <div style={{ display: 'flex', borderBottom: '1px solid #334155', marginBottom: '8px' }}>
        <button style={tabStyle(!showPreview)} onClick={() => setShowPreview(false)}>Write</button>
        <button style={tabStyle(showPreview)} onClick={() => setShowPreview(true)}>Preview</button>
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

      <input style={s.input} placeholder="Summary (optional)" value={summary} onChange={e => setSummary(e.target.value)} />
      <input style={s.input} placeholder="Tags (comma-separated)" value={tags} onChange={e => setTags(e.target.value)} />
      <div style={{ display: 'flex', gap: '8px', alignItems: 'center', marginTop: '8px' }}>
        <select style={{ ...s.input, width: 'auto', marginBottom: 0 }} value={status} onChange={e => setStatus(e.target.value)}>
          <option value="draft">Draft</option>
          <option value="published">Published</option>
        </select>
        <button style={s.btn(true)} onClick={handleSave} disabled={!title.trim()}>
          {post ? 'Update' : 'Create'} Post
        </button>
        <button style={s.btn()} onClick={onCancel}>Cancel</button>
      </div>
    </div>
  );
}

function PostView({ blogId, slug, onNavigate }) {
  const [post, setPost] = useState(null);
  const [comments, setComments] = useState([]);
  const [newComment, setNewComment] = useState('');
  const [commentAuthor, setCommentAuthor] = useState('');
  const [editing, setEditing] = useState(false);
  const manageKey = localStorage.getItem(`blog_key_${blogId}`);
  const canEdit = !!manageKey;

  const loadPost = useCallback(() => {
    apiFetch(`/blogs/${blogId}/posts/${slug}`).then(setPost).catch(() => setPost({ error: true }));
  }, [blogId, slug]);

  const loadComments = useCallback(() => {
    if (!post?.id) return;
    apiFetch(`/blogs/${blogId}/posts/${post.id}/comments`).then(setComments).catch(console.error);
  }, [blogId, post?.id]);

  useEffect(() => { loadPost(); }, [loadPost]);
  useEffect(() => { loadComments(); }, [loadComments]);
  useEffect(() => {
    if (post?.content_html && window.hljs) {
      document.querySelectorAll('.post-content pre code').forEach(el => {
        window.hljs.highlightElement(el);
      });
    }
  }, [post?.content_html]);

  const handleComment = async () => {
    if (!newComment.trim() || !commentAuthor.trim()) return;
    try {
      await apiFetch(`/blogs/${blogId}/posts/${post.id}/comments`, {
        method: 'POST', body: { author_name: commentAuthor, content: newComment },
      });
      setNewComment('');
      loadComments();
    } catch (err) {
      alert(err.error || 'Failed to post comment');
    }
  };

  const handleDelete = async () => {
    if (!confirm('Delete this post?')) return;
    try {
      await apiFetch(`/blogs/${blogId}/posts/${post.id}`, { method: 'DELETE' });
      onNavigate('blog', blogId);
    } catch (err) {
      alert(err.error || 'Failed to delete');
    }
  };

  if (!post) return <div style={s.container}>Loading...</div>;
  if (post.error) return <div style={s.container}><p>Post not found.</p></div>;

  if (editing && canEdit) {
    return (
      <div style={s.container}>
        <PostEditor blogId={blogId} post={post} onDone={() => { setEditing(false); loadPost(); }} onCancel={() => setEditing(false)} />
      </div>
    );
  }

  return (
    <div style={s.container}>
      <div style={{ marginBottom: '16px' }}>
        <span style={s.link} onClick={() => onNavigate('blog', blogId)}>← Back to blog</span>
      </div>

      <article style={s.card}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
          <h1 style={{ fontSize: '1.8rem', marginBottom: '8px', flex: 1 }}>{post.title}</h1>
          {canEdit && (
            <div style={{ display: 'flex', gap: '4px' }}>
              <button style={s.btn()} onClick={() => setEditing(true)}>✏️</button>
              <button style={{ ...s.btn(), color: '#ef4444' }} onClick={handleDelete}>🗑️</button>
            </div>
          )}
        </div>
        <div style={{ display: 'flex', gap: '8px', marginBottom: '16px', ...s.muted }}>
          {post.author_name && <span>{post.author_name}</span>}
          <span>{new Date(post.published_at || post.created_at).toLocaleDateString()}</span>
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
        <h3 style={{ marginBottom: '12px' }}>Comments ({comments.length})</h3>
        {comments.map(c => (
          <div key={c.id} style={{ borderBottom: '1px solid #334155', padding: '12px 0' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between' }}>
              <strong style={{ fontSize: '0.9rem' }}>{c.author_name}</strong>
              <span style={s.muted}>{new Date(c.created_at).toLocaleString()}</span>
            </div>
            <p style={{ marginTop: '4px', lineHeight: 1.5 }}>{c.content}</p>
          </div>
        ))}
        {post.status === 'published' && (
          <div style={{ marginTop: '16px' }}>
            <input style={s.input} placeholder="Your name" value={commentAuthor} onChange={e => setCommentAuthor(e.target.value)} />
            <textarea style={{ ...s.textarea, minHeight: '80px' }} placeholder="Write a comment..." value={newComment} onChange={e => setNewComment(e.target.value)} />
            <button style={s.btn(true)} onClick={handleComment} disabled={!newComment.trim() || !commentAuthor.trim()}>Post Comment</button>
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
  }, []);

  useEffect(() => {
    const handleRoute = () => {
      const path = window.location.pathname;
      const match = path.match(/^\/blog\/([^/]+)\/post\/([^/]+)/);
      if (match) {
        setRoute({ page: 'post', blogId: match[1], slug: match[2] });
      } else if (path.match(/^\/blog\/([^/]+)/)) {
        const blogId = path.split('/')[2];
        setRoute({ page: 'blog', blogId });
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
