import { useState, useEffect } from 'react';
import { Link } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { api } from '../lib/api';
import type { Chart } from '../types';

export function ProfilePage() {
  const { user } = useAuth();
  const [charts, setCharts] = useState<Chart[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.getCharts().then(setCharts).finally(() => setLoading(false));
  }, []);

  return (
    <div style={{ padding: '2rem', maxWidth: '1000px', margin: '0 auto' }}>
      <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '2rem' }}>
        <h2 style={{ marginBottom: '1rem' }}>帳號資訊</h2>
        <p style={{ marginBottom: '0.5rem' }}><strong>Email:</strong> {user?.email}</p>
        <p style={{ marginBottom: '0.5rem' }}>
          <strong>方案:</strong> {user?.billing.tier === 'free' ? '免費' : user?.billing.tier}
          {user?.billing.isTrialing && <span style={{ color: '#10b981', marginLeft: '0.5rem' }}>✓ 試用中</span>}
        </p>
        {user?.billing.trialEndsAt && (
          <p style={{ fontSize: '0.875rem', color: '#6b7280' }}>
            試用期至: {new Date(user.billing.trialEndsAt).toLocaleDateString('zh-TW')}
          </p>
        )}
      </div>

      <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem', flexWrap: 'wrap', gap: '1rem' }}>
          <h2>我的命盤</h2>
          <Link to="/chart/new" style={{ background: '#4F46E5', color: 'white', padding: '0.5rem 1.5rem', borderRadius: '6px', textDecoration: 'none' }}>+ 建立新命盤</Link>
        </div>

        {loading ? (
          <p>載入中...</p>
        ) : charts.length === 0 ? (
          <p style={{ color: '#6b7280', textAlign: 'center', padding: '2rem' }}>尚無命盤，立即建立第一個命盤吧！</p>
        ) : (
          <div style={{ display: 'grid', gap: '1rem' }}>
            {charts.map(chart => (
              <Link 
                key={chart.id} 
                to={`/chart/${chart.id}`}
                style={{ 
                  display: 'block', 
                  padding: '1rem', 
                  border: '1px solid #e5e7eb', 
                  borderRadius: '6px', 
                  textDecoration: 'none',
                  transition: 'all 0.2s'
                }}
                onMouseEnter={(e) => e.currentTarget.style.borderColor = '#4F46E5'}
                onMouseLeave={(e) => e.currentTarget.style.borderColor = '#e5e7eb'}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.5rem' }}>
                  <div>
                    <span style={{ fontWeight: '500', color: '#1f2937' }}>
                      {chart.chart_type === 'ziwei' ? '紫微斗數' : '西洋占星'}
                    </span>
                    {chart.ai_interpretation && <span style={{ marginLeft: '0.5rem', fontSize: '0.875rem', color: '#10b981' }}>✓ 已解讀</span>}
                  </div>
                  <span style={{ fontSize: '0.875rem', color: '#6b7280' }}>
                    {new Date(chart.created_at).toLocaleDateString('zh-TW')}
                  </span>
                </div>
              </Link>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
