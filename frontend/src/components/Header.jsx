import React from 'react';
import BlogLogo from './BlogLogo';

export default function Header({ onNavigate, theme, onToggleTheme }) {
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
