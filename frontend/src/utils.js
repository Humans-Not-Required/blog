const API = '/api/v1';

// ─── API ───

export async function apiFetch(path, opts = {}) {
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

// ─── Auth ───

export function getBlogKey() {
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

export function getSavedAuthor() {
  return localStorage.getItem('hnr_blog_author') || '';
}

export function saveAuthor(name) {
  if (name.trim()) localStorage.setItem('hnr_blog_author', name.trim());
}

// ─── My Blogs (localStorage) ───

export function getMyBlogs() {
  try { return JSON.parse(localStorage.getItem('hnr_my_blogs') || '[]'); }
  catch { return []; }
}

export function addMyBlog(id, name) {
  const blogs = getMyBlogs().filter(b => b.id !== id);
  const hasKey = !!localStorage.getItem(`blog_key_${id}`);
  blogs.unshift({ id, name, hasKey, addedAt: Date.now() });
  localStorage.setItem('hnr_my_blogs', JSON.stringify(blogs));
}

export function removeMyBlog(id) {
  const blogs = getMyBlogs().filter(b => b.id !== id);
  localStorage.setItem('hnr_my_blogs', JSON.stringify(blogs));
}

export function refreshMyBlogKeys() {
  const blogs = getMyBlogs().map(b => ({
    ...b,
    hasKey: !!localStorage.getItem(`blog_key_${b.id}`),
  }));
  localStorage.setItem('hnr_my_blogs', JSON.stringify(blogs));
  return blogs;
}

// ─── Formatting ───

export function formatDate(dateStr) {
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

export const modKey = typeof navigator !== 'undefined' && (navigator.platform?.includes('Mac') || navigator.userAgent?.includes('Mac')) ? '⌘' : 'Ctrl';
