import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { api } from '../lib/api';

const TITLES: Record<string, string> = { ziwei: '紫微斗數', western: '西洋占星' };

export function DivinationPage() {
  const { type } = useParams<{ type: 'ziwei' | 'western' }>();
  const navigate = useNavigate();
  const [chart, setChart] = useState<Record<string, unknown> | null>(null);
  const [interpretation, setInterpretation] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [interpreting, setInterpreting] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!type || !['ziwei', 'western'].includes(type)) {
      navigate('/profile');
      return;
    }
    loadChart();
  }, [type]);

  const loadChart = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api.getChart(type as 'ziwei' | 'western');
      setChart(typeof data.chart_data === 'string' ? JSON.parse(data.chart_data) : data.chart_data);
      setInterpretation(data.ai_interpretation || null);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('NO_BIRTH_DATA') || msg.includes('NO_GENDER')) {
        navigate('/profile');
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleInterpret = async () => {
    setInterpreting(true);
    setError('');
    try {
      const result = await api.interpret(type as 'ziwei' | 'western');
      setInterpretation(result.interpretation);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('404') || msg.includes('not found') || msg.includes('Not found')) {
        // Chart may have been invalidated — reload it first, then retry
        await loadChart();
      } else {
        setError(msg);
      }
    } finally {
      setInterpreting(false);
    }
  };

  const cardStyle = { background: 'white', padding: '1.5rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '1.5rem' };

  if (loading) return <div style={{ padding: '2rem', textAlign: 'center' }}>載入中...</div>;

  return (
    <div style={{ padding: '1.5rem', maxWidth: '800px', margin: '0 auto' }}>
      <button onClick={() => navigate('/profile')} style={{ background: 'none', border: 'none', color: '#4F46E5', cursor: 'pointer', marginBottom: '1rem' }}>
        ← 返回
      </button>

      <h1 style={{ marginBottom: '1.5rem' }}>{TITLES[type!] || type}</h1>

      {error && <p style={{ color: '#ef4444', marginBottom: '1rem' }}>{error}</p>}

      {/* Chart Data */}
      {chart && (
        <div style={cardStyle}>
          <h2 style={{ marginBottom: '1rem' }}>命盤資料</h2>
          {type === 'ziwei' && <ZiWeiDisplay data={chart} />}
          {type === 'western' && <WesternDisplay data={chart} />}
        </div>
      )}

      {/* AI Interpretation */}
      <div style={cardStyle}>
        <h2 style={{ marginBottom: '1rem' }}>AI 解讀</h2>
        {interpretation ? (
          <div style={{ whiteSpace: 'pre-wrap', lineHeight: 1.8 }}>{interpretation}</div>
        ) : (
          <div>
            <p style={{ color: '#6b7280', marginBottom: '1rem' }}>尚未產生 AI 解讀</p>
            <button onClick={handleInterpret} disabled={interpreting}
              style={{ background: '#4F46E5', color: 'white', padding: '0.75rem 1.5rem', borderRadius: '6px', border: 'none', cursor: 'pointer' }}>
              {interpreting ? '解讀中...' : '產生 AI 解讀'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

function ZiWeiDisplay({ data }: { data: Record<string, unknown> }) {
  const d = data as { lunarDate?: { year: number; month: number; day: number }; lifePalace?: string; bodyPalace?: string; fiveElement?: string; mainStars?: string[] };
  return (
    <div style={{ display: 'grid', gap: '0.5rem' }}>
      {d.lunarDate && <p><strong>農曆:</strong> {d.lunarDate.year}年{d.lunarDate.month}月{d.lunarDate.day}日</p>}
      {d.lifePalace && <p><strong>命宮:</strong> {d.lifePalace}</p>}
      {d.bodyPalace && <p><strong>身宮:</strong> {d.bodyPalace}</p>}
      {d.fiveElement && <p><strong>五行局:</strong> {d.fiveElement}</p>}
      {d.mainStars && <p><strong>主星:</strong> {d.mainStars.join('、')}</p>}
    </div>
  );
}

function WesternDisplay({ data }: { data: Record<string, unknown> }) {
  const d = data as { sunSign?: string; moonSign?: string; planets?: Record<string, string> };
  return (
    <div style={{ display: 'grid', gap: '0.5rem' }}>
      {d.sunSign && <p><strong>太陽星座:</strong> {d.sunSign}</p>}
      {d.moonSign && <p><strong>月亮星座:</strong> {d.moonSign} (約略)</p>}
      {d.planets && Object.entries(d.planets).map(([k, v]) => (
        <p key={k}><strong>{k}:</strong> {v}</p>
      ))}
    </div>
  );
}
