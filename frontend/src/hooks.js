import { useState, useEffect, useCallback } from 'react';

// ─── Theme Hook ───

function getInitialTheme() {
  const saved = localStorage.getItem('hnr_blog_theme');
  if (saved) return saved;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
}

export function useTheme() {
  const [theme, setTheme] = useState(getInitialTheme);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
    localStorage.setItem('hnr_blog_theme', theme);

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

// ─── Escape Key Hook ───

export function useEscapeKey(handler) {
  useEffect(() => {
    const onKey = (e) => { if (e.key === 'Escape') handler(); };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [handler]);
}
