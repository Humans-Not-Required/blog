import React, { useState, useCallback } from 'react';
import { apiFetch, addMyBlog } from '../utils';
import { useEscapeKey } from '../hooks';
import CopyButton from './CopyButton';

export default function CreateBlog({ onNavigate }) {
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
