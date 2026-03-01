import React, { useState, useEffect, useCallback } from 'react';
import './App.css';
import { useTheme } from './hooks';
import Header from './components/Header';
import Home from './components/Home';
import CreateBlog from './components/CreateBlog';
import BlogView from './components/BlogView';
import PostView from './components/PostView';

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
