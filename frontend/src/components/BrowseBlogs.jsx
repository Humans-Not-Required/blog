import React, { useState, useEffect } from 'react';
import { apiFetch, formatDate, refreshMyBlogKeys, getMyBlogs, removeMyBlog } from '../utils';
import { useDocTitle } from '../hooks';
import BlogLogo from './BlogLogo';

export default function BrowseBlogs({ onNavigate }) {
  const [blogs, setBlogs] = useState([]);
  const [blogUrl, setBlogUrl] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState(null);
  const [myBlogs, setMyBlogs] = useState(() => refreshMyBlogKeys());
  const [searching, setSearching] = useState(false);

  useDocTitle(null); // default title on home page

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
