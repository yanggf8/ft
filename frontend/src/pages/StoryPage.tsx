import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { api } from '../lib/api';

interface Chapter {
  heading: string;
  body: string;
}

function parseChapters(story: string): Chapter[] {
  const headingRegex = /^##\s+.*$/gm;
  const matches: RegExpExecArray[] = [];
  let m: RegExpExecArray | null;
  while ((m = headingRegex.exec(story)) !== null) {
    matches.push(m);
  }
  if (matches.length === 0) return [];

  const chapters: Chapter[] = [];
  for (let i = 0; i < matches.length; i++) {
    const start = matches[i].index!;
    const end = i + 1 < matches.length ? matches[i + 1].index! : story.length;
    const block = story.slice(start, end);
    const lineEnd = block.indexOf('\n');
    const heading = lineEnd === -1 ? block.trim() : block.slice(0, lineEnd).trim();
    const body = lineEnd === -1 ? '' : block.slice(lineEnd + 1).trim();
    chapters.push({ heading: heading.replace(/^##\s+/, ''), body });
  }
  return chapters;
}

export function StoryPage() {
  const navigate = useNavigate();
  const [story, setStory] = useState<string | null>(null);
  const [notGenerated, setNotGenerated] = useState(false);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    loadStory();
  }, []);

  const loadStory = async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api.getStory();
      setStory(typeof data.story === 'string' ? data.story : null);
      setNotGenerated(false);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('NO_STORY') || msg.includes('404')) {
        setNotGenerated(true);
      } else if (msg.includes('NO_BIRTH_DATA') || msg.includes('NO_GENDER')) {
        navigate('/profile');
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleGenerate = async () => {
    setGenerating(true);
    setError('');
    try {
      const result = await api.generateStory();
      setStory(typeof result.story === 'string' ? result.story : null);
      setNotGenerated(false);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('RATE_LIMIT')) {
        setError('請求過於頻繁，請稍後再試');
      } else if (msg.includes('AI_UNAVAILABLE')) {
        setError('AI 服務暫時無法使用，請稍後再試');
      } else if (msg.includes('NO_BIRTH_DATA') || msg.includes('NO_GENDER')) {
        navigate('/profile');
      } else {
        setError(msg);
      }
    } finally {
      setGenerating(false);
    }
  };

  const cardStyle = { background: 'white', padding: '1.5rem', borderRadius: '8px', boxShadow: '0 1px 3px rgba(0,0,0,0.1)', marginBottom: '1.5rem' };

  if (loading) return <div style={{ padding: '2rem', textAlign: 'center' }}>載入中...</div>;

  const chapters = story ? parseChapters(story) : [];

  return (
    <div style={{ padding: '1.5rem', maxWidth: '800px', margin: '0 auto' }}>
      <button onClick={() => navigate('/profile')} style={{ background: 'none', border: 'none', color: '#4F46E5', cursor: 'pointer', marginBottom: '1rem' }}>
        ← 返回
      </button>

      <h1 style={{ marginBottom: '1.5rem' }}>合盤故事</h1>

      {error && <p style={{ color: '#ef4444', marginBottom: '1rem' }}>{error}</p>}

      {notGenerated && !story && (
        <div style={cardStyle}>
          <p style={{ color: '#6b7280', marginBottom: '1rem' }}>將您的紫微斗數與西洋占星命盤融合成一篇專屬的合盤故事。</p>
          <button onClick={handleGenerate} disabled={generating}
            style={{ background: '#4F46E5', color: 'white', padding: '0.75rem 1.5rem', borderRadius: '6px', border: 'none', cursor: 'pointer' }}>
            {generating ? '故事生成中…（約 10–30 秒）' : '生成合盤故事'}
          </button>
        </div>
      )}

      {story && chapters.length > 0 && chapters.map((ch, i) => (
        <div key={i} style={cardStyle}>
          <h2 style={{ marginBottom: '1rem' }}>{ch.heading}</h2>
          <div style={{ whiteSpace: 'pre-wrap', lineHeight: 1.8 }}>{ch.body}</div>
        </div>
      ))}

      {story && chapters.length === 0 && (
        <div style={cardStyle}>
          <div style={{ whiteSpace: 'pre-wrap', lineHeight: 1.8 }}>{story}</div>
        </div>
      )}
    </div>
  );
}
