import { Link } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';

export function Layout({ children }: { children: React.ReactNode }) {
  const { user, logout } = useAuth();

  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      <nav style={{ background: '#4F46E5', color: 'white', padding: '1rem 2rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
        <Link to="/" style={{ color: 'white', fontSize: '1.25rem', fontWeight: 'bold', textDecoration: 'none' }}>FortuneT</Link>
        <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
          {user ? (
            <>
              <Link to="/profile" style={{ color: 'white' }}>我的命盤</Link>
              <Link to="/divination/ziwei" style={{ color: 'white' }}>紫微斗數</Link>
              <Link to="/divination/western" style={{ color: 'white' }}>西洋占星</Link>
              <button onClick={logout} style={{ background: 'transparent', border: '1px solid white', color: 'white', padding: '0.25rem 0.75rem', borderRadius: '4px' }}>登出</button>
            </>
          ) : (
            <Link to="/login" style={{ color: 'white' }}>登入</Link>
          )}
        </div>
      </nav>
      <main style={{ flex: 1 }}>{children}</main>
      <footer style={{ background: '#f3f4f6', padding: '1rem', textAlign: 'center', fontSize: '0.875rem', color: '#6b7280' }}>
        © 2025 FortuneT - AI 智能命理分析
      </footer>
    </div>
  );
}
