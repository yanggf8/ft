type Star = { name: string; type: string; brightness?: string; sihua?: string };
type Palace = { index: number; name: string; branch: string; stem: string; stars: Star[]; isLifePalace?: boolean; isBodyPalace?: boolean };

export function ZiWeiPalaceGrid({ palaces }: { palaces: Palace[] }) {
  const sihuaLabel: Record<string, string> = { lu: '祿', quan: '權', ke: '科', ji: '忌' };
  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '0.5rem' }}>
      {palaces.map((p) => (
        <div
          key={p.branch}
          style={{
            border: p.isLifePalace ? '2px solid #4F46E5' : '1px solid #e5e7eb',
            borderRadius: 8,
            padding: '0.5rem',
            background: p.isLifePalace ? '#EEF2FF' : 'white',
          }}
        >
          <div style={{ fontWeight: 600, fontSize: '0.85rem' }}>
            {p.branch} {p.stem} · {p.name} {p.isLifePalace && '★命宮'} {p.isBodyPalace && '·身宮'}
          </div>
          <div style={{ marginTop: '0.25rem', display: 'flex', flexWrap: 'wrap', gap: '0.25rem' }}>
            {p.stars.map((s) => (
              <span
                key={s.name}
                style={{
                  fontSize: '0.8rem',
                  padding: '0.15rem 0.35rem',
                  borderRadius: 4,
                  background: s.type === 'main' ? '#FEF3C7' : s.type === 'transformation' ? '#FEE2E2' : '#F3F4F6',
                }}
              >
                {s.name}
                {s.brightness ? `(${s.brightness})` : ''}
                {s.sihua ? `化${sihuaLabel[s.sihua]}` : ''}
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
