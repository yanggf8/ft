import { useState, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { api } from '../lib/api';
import { ChartForm } from '../components/ChartForm';
import type { Chart } from '../types';

export function ChartPage() {
  const { id } = useParams<{ id: string }>();
  const [chart, setChart] = useState<Chart | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (id && id !== 'new') {
      api.getChart(id).then(setChart);
    }
  }, [id]);

  const handleInterpret = async () => {
    if (!chart) return;
    setLoading(true);
    try {
      const updated = await api.interpretChart(chart.id);
      setChart(updated);
    } finally {
      setLoading(false);
    }
  };

  if (id === 'new') {
    return (
      <div style={{ padding: '2rem', maxWidth: '800px', margin: '0 auto' }}>
        <h2>建立新命盤</h2>
        <ChartForm />
      </div>
    );
  }

  if (!chart) return <div style={{ padding: '2rem', textAlign: 'center' }}>載入中...</div>;

  return (
    <div style={{ padding: '2rem', maxWidth: '1000px', margin: '0 auto' }}>
      <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '2rem' }}>
        <h2 style={{ marginBottom: '1rem' }}>{chart.chart_type === 'ziwei' ? '紫微斗數命盤' : '西洋占星命盤'}</h2>
        <p style={{ fontSize: '0.875rem', color: '#6b7280' }}>建立時間: {new Date(chart.created_at).toLocaleString('zh-TW')}</p>
      </div>

      <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '2rem' }}>
        <h3 style={{ marginBottom: '1rem' }}>命盤資料</h3>
        <pre style={{ background: '#f9fafb', padding: '1rem', borderRadius: '6px', overflow: 'auto', fontSize: '0.875rem' }}>
          {JSON.stringify(chart.chart_data, null, 2)}
        </pre>
      </div>

      {chart.ai_interpretation ? (
        <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)' }}>
          <h3 style={{ marginBottom: '1rem' }}>🤖 AI 智能解讀</h3>
          <div style={{ whiteSpace: 'pre-wrap', lineHeight: '1.8', color: '#374151' }}>
            {chart.ai_interpretation}
          </div>
        </div>
      ) : (
        <div style={{ background: 'white', padding: '2rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', textAlign: 'center' }}>
          <p style={{ marginBottom: '1rem', color: '#6b7280' }}>尚未獲取 AI 解讀</p>
          <button 
            onClick={handleInterpret} 
            disabled={loading}
            style={{ 
              background: loading ? '#9ca3af' : '#4F46E5', 
              color: 'white', 
              padding: '0.75rem 2rem', 
              border: 'none', 
              borderRadius: '6px',
              fontSize: '1rem'
            }}
          >
            {loading ? '解讀中...' : '獲取 AI 解讀'}
          </button>
        </div>
      )}
    </div>
  );
}
