import { Link } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';

export function HomePage() {
  const { user } = useAuth();

  return (
    <div style={{ padding: '3rem 2rem', maxWidth: '1200px', margin: '0 auto' }}>
      <div style={{ textAlign: 'center', marginBottom: '3rem' }}>
        <h1 style={{ fontSize: '2.5rem', marginBottom: '1rem', color: '#1f2937' }}>FortuneT - AI 智能命理分析</h1>
        <p style={{ fontSize: '1.25rem', color: '#6b7280', marginBottom: '2rem' }}>紫微斗數與西洋占星的專業解讀</p>
        
        {user ? (
          <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center', flexWrap: 'wrap' }}>
            <Link to="/chart/new" style={{ background: '#4F46E5', color: 'white', padding: '0.75rem 2rem', borderRadius: '8px', textDecoration: 'none', fontWeight: '500' }}>建立命盤</Link>
            <Link to="/profile" style={{ background: 'white', color: '#4F46E5', padding: '0.75rem 2rem', borderRadius: '8px', textDecoration: 'none', fontWeight: '500', border: '2px solid #4F46E5' }}>查看我的命盤</Link>
          </div>
        ) : (
          <Link to="/login" style={{ background: '#4F46E5', color: 'white', padding: '0.75rem 2rem', borderRadius: '8px', textDecoration: 'none', fontWeight: '500', display: 'inline-block' }}>開始使用</Link>
        )}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: '2rem', marginTop: '3rem' }}>
        <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
          <h3 style={{ fontSize: '1.25rem', marginBottom: '0.5rem', color: '#1f2937' }}>🔮 紫微斗數</h3>
          <p style={{ color: '#6b7280' }}>傳統中國命理學，精準分析命盤格局與人生運勢</p>
        </div>
        <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
          <h3 style={{ fontSize: '1.25rem', marginBottom: '0.5rem', color: '#1f2937' }}>⭐ 西洋占星</h3>
          <p style={{ color: '#6b7280' }}>星座與行星位置分析，探索性格與天賦</p>
        </div>
        <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
          <h3 style={{ fontSize: '1.25rem', marginBottom: '0.5rem', color: '#1f2937' }}>🤖 AI 解讀</h3>
          <p style={{ color: '#6b7280' }}>智能 AI 提供專業且易懂的命理解讀</p>
        </div>
      </div>
    </div>
  );
}
