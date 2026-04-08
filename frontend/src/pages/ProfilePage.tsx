import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../contexts/AuthContext';
import { BirthDataForm } from '../components/BirthDataForm';

export function ProfilePage() {
  const { user, refreshUser } = useAuth();
  const navigate = useNavigate();
  const [showForm, setShowForm] = useState(false);

  useEffect(() => {
    if (user && !user.hasBirthData) setShowForm(true);
  }, [user]);

  const handleSaved = async () => {
    await refreshUser(true);
    setShowForm(false);
  };

  const cardStyle = { background: 'white', padding: '1.5rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '1.5rem' };
  const btnStyle = { background: '#4F46E5', color: 'white', padding: '1rem 1.5rem', borderRadius: '8px', border: 'none', cursor: 'pointer', fontSize: '1rem', width: '100%' };

  return (
    <div style={{ padding: '1.5rem', maxWidth: '600px', margin: '0 auto' }}>
      {/* Account Info */}
      <div style={cardStyle}>
        <h2 style={{ marginBottom: '1rem' }}>帳號資訊</h2>
        <p><strong>Email:</strong> {user?.email}</p>
        <p>
          <strong>方案:</strong> {user?.billing.tier === 'free' ? '免費' : user?.billing.tier}
          {user?.billing.isTrialing && <span style={{ color: '#10b981', marginLeft: '0.5rem' }}>✓ 試用中</span>}
        </p>
      </div>

      {/* Birth Data */}
      <div style={cardStyle}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
          <h2>出生資料</h2>
          {user?.hasBirthData && !showForm && (
            <button onClick={() => setShowForm(true)} style={{ background: 'none', border: 'none', color: '#4F46E5', cursor: 'pointer' }}>
              編輯
            </button>
          )}
        </div>

        {showForm ? (
          <BirthDataForm
            initial={{
              birth_year: user?.birth_year,
              birth_month: user?.birth_month,
              birth_day: user?.birth_day,
              birth_hour: user?.birth_hour,
              gender: user?.gender
            }}
            onSaved={handleSaved}
          />
        ) : user?.hasBirthData ? (
          <p style={{ color: '#374151' }}>
            {user.birth_year}年{user.birth_month}月{user.birth_day}日
            {user.birth_hour !== null && user.birth_hour !== undefined ? ` ${user.birth_hour}時` : ' (時辰不詳)'}
            {user.gender && ` · ${user.gender === 'male' ? '男' : '女'}`}
          </p>
        ) : (
          <p style={{ color: '#6b7280' }}>請先填寫出生資料以開始算命</p>
        )}
      </div>

      {/* Divination Options */}
      {user?.hasBirthData && (
        <div style={cardStyle}>
          <h2 style={{ marginBottom: '1rem' }}>選擇算命方式</h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <button onClick={() => navigate('/divination/ziwei')} style={btnStyle}>
              紫微斗數
            </button>
            <button onClick={() => navigate('/divination/western')} style={{ ...btnStyle, background: '#7C3AED' }}>
              西洋占星
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
