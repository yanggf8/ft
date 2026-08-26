export function computeBirthHash(data: {
  birth_year?: number; birth_month?: number; birth_day?: number;
  birth_hour?: number; birth_minute?: number; gender?: string;
  timezone?: string; latitude?: number; longitude?: number;
}): string {
  const str = [
    data.birth_year, data.birth_month, data.birth_day,
    data.birth_hour ?? 12, data.birth_minute ?? 0, data.gender ?? '',
    data.timezone ?? 'Asia/Taipei', data.latitude ?? '', data.longitude ?? '',
  ].join('-');
  let hash = 0;
  for (let i = 0; i < str.length; i++) {
    hash = ((hash << 5) - hash) + str.charCodeAt(i);
    hash |= 0;
  }
  return hash.toString(16);
}
