import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../lib/api';

export function ChartForm() {
  const navigate = useNavigate();
  const [chartType, setChartType] = useState<'ziwei' | 'western'>('ziwei');
  const [formData, setFormData] = useState({
    year: new Date().getFullYear() - 30,
    month: 1,
    day: 1,
    hour: 12,
    minute: 0,
    gender: 'male' as 'male' | 'female',
    timezone: 'Asia/Taipei'
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError('');
    try {
      const chart = await api.createChart(chartType, formData);
      navigate(`/chart/${chart.id}`);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} style={{ maxWidth: '500px' }}>
      <div style={{ marginBottom: '1rem' }}>
        <label style={{ display: 'block', marginBottom: '0.5rem' }}>命盤類型</label>
        <select value={chartType} onChange={(e) => setChartType(e.target.value as any)} style={{ width: '100%', padding: '0.5rem' }}>
          <option value="ziwei">紫微斗數</option>
          <option value="western">西洋占星</option>
        </select>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem', marginBottom: '1rem' }}>
        <div>
          <label style={{ display: 'block', marginBottom: '0.5rem' }}>年</label>
          <input type="number" value={formData.year} onChange={(e) => setFormData({...formData, year: +e.target.value})} min="1900" max="2100" required style={{ width: '100%', padding: '0.5rem' }} />
        </div>
        <div>
          <label style={{ display: 'block', marginBottom: '0.5rem' }}>月</label>
          <input type="number" value={formData.month} onChange={(e) => setFormData({...formData, month: +e.target.value})} min="1" max="12" required style={{ width: '100%', padding: '0.5rem' }} />
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: '1rem', marginBottom: '1rem' }}>
        <div>
          <label style={{ display: 'block', marginBottom: '0.5rem' }}>日</label>
          <input type="number" value={formData.day} onChange={(e) => setFormData({...formData, day: +e.target.value})} min="1" max="31" required style={{ width: '100%', padding: '0.5rem' }} />
        </div>
        <div>
          <label style={{ display: 'block', marginBottom: '0.5rem' }}>時</label>
          <input type="number" value={formData.hour} onChange={(e) => setFormData({...formData, hour: +e.target.value})} min="0" max="23" required style={{ width: '100%', padding: '0.5rem' }} />
        </div>
        <div>
          <label style={{ display: 'block', marginBottom: '0.5rem' }}>分</label>
          <input type="number" value={formData.minute} onChange={(e) => setFormData({...formData, minute: +e.target.value})} min="0" max="59" required style={{ width: '100%', padding: '0.5rem' }} />
        </div>
      </div>

      <div style={{ marginBottom: '1rem' }}>
        <label style={{ display: 'block', marginBottom: '0.5rem' }}>性別</label>
        <select value={formData.gender} onChange={(e) => setFormData({...formData, gender: e.target.value as any})} style={{ width: '100%', padding: '0.5rem' }}>
          <option value="male">男</option>
          <option value="female">女</option>
        </select>
      </div>

      {error && <p style={{ color: 'red', marginBottom: '1rem' }}>{error}</p>}
      <button type="submit" disabled={loading} style={{ width: '100%', padding: '0.75rem', background: '#4F46E5', color: 'white', border: 'none', borderRadius: '4px', cursor: 'pointer' }}>
        {loading ? '計算中...' : '建立命盤'}
      </button>
    </form>
  );
}
