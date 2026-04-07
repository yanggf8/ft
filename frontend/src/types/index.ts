export interface User {
  id: string;
  email: string;
  full_name?: string;
  birth_year?: number;
  birth_month?: number;
  birth_day?: number;
  birth_hour?: number;
  birth_minute?: number;
  gender?: 'male' | 'female';
  timezone?: string;
  hasBirthData: boolean;
  billing: {
    tier: 'free' | 'premium' | 'professional';
    isTrialing: boolean;
    trialEndsAt: string | null;
    hasAccess: boolean;
  };
}

export interface Interpretation {
  id: string;
  user_id: string;
  divination_type: 'ziwei' | 'western';
  chart_data: Record<string, unknown>;
  ai_interpretation?: string;
  birth_data_hash: string;
  created_at: string;
}
