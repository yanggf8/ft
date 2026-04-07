import { useState } from 'react';
import { api, BirthData } from '../lib/api';

interface Props {
  initial?: Partial<BirthData>;
  onSaved: () => void;
}

export function BirthDataForm({ initial, onSaved }: Props) {
  const [form, setForm] = useState({
    birth_year: initial?.birth_year || new Date().getFullYear() - 30,
    birth_month: initial?.birth_month || 1,
    birth_day: initial?.birth_day || 1,
    birth_hour: initial?.birth_hour ?? null as number | null,
    gender: initial?.gender || '' as 'male' | 'female' | '',
    unknownHour: initial?.birth_hour === null || initial?.birth_hour === undefined
  });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setSaving(true);
    try {
      await api.updateBirthData({
        birth_year: form.birth_year,
        birth_month: form.birth_month,
        birth_day: form.birth_day,
        birth_hour: form.unknownHour ? null : form.birth_hour,
        gender: form.gender || undefined
      });
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const inputStyle = { padding: '0.5rem', border: '1px solid #d1d5db', borderRadius: '4px', width: '100%' };

  return (
    <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: '0.5rem' }}>
        <div>
          <label style={{ fontSize: '0.875rem', color: '#6b7280' }}>出生年</label>
          <input type="number" min={1900} max={2100} value={form.birth_year}
            onChange={e => setForm({ ...form, birth_year: +e.target.value })} style={inputStyle} required />
        </div>
        <div>
          <label style={{ fontSize: '0.875rem', color: '#6b7280' }}>月</label>
          <input type="number" min={1} max={12} value={form.birth_month}
            onChange={e => setForm({ ...form, birth_month: +e.target.value })} style={inputStyle} required />
        </div>
        <div>
          <label style={{ fontSize: '0.875rem', color: '#6b7280' }}>日</label>
          <input type="number" min={1} max={31} value={form.birth_day}
            onChange={e => setForm({ ...form, birth_day: +e.target.value })} style={inputStyle} required />
        </div>
      </div>

      <div>
        <label style={{ fontSize: '0.875rem', color: '#6b7280' }}>出生時辰</label>
        <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
          <input type="number" min={0} max={23} value={form.birth_hour ?? 12}
            onChange={e => setForm({ ...form, birth_hour: +e.target.value })}
            disabled={form.unknownHour} style={{ ...inputStyle, flex: 1 }} />
          <label style={{ display: 'flex', alignItems: 'center', gap: '0.25rem', fontSize: '0.875rem' }}>
            <input type="checkbox" checked={form.unknownHour}
              onChange={e => setForm({ ...form, unknownHour: e.target.checked })} />
            不確定
          </label>
        </div>
      </div>

      <div>
        <label style={{ fontSize: '0.875rem', color: '#6b7280' }}>性別 (紫微斗數需要)</label>
        <select value={form.gender} onChange={e => setForm({ ...form, gender: e.target.value as 'male' | 'female' })}
          style={inputStyle}>
          <option value="">-- 選擇 --</option>
          <option value="male">男</option>
          <option value="female">女</option>
        </select>
      </div>

      {error && <p style={{ color: '#ef4444', fontSize: '0.875rem' }}>{error}</p>}

      <button type="submit" disabled={saving}
        style={{ background: '#4F46E5', color: 'white', padding: '0.75rem', borderRadius: '6px', border: 'none', cursor: 'pointer' }}>
        {saving ? '儲存中...' : '儲存出生資料'}
      </button>
    </form>
  );
}
