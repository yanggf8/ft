export interface User {
  id: string;
  email: string;
  full_name?: string;
  billing: {
    tier: 'free' | 'premium' | 'professional';
    isTrialing: boolean;
    trialEndsAt: string | null;
    hasAccess: boolean;
  };
}

export interface Chart {
  id: string;
  user_id: string;
  chart_type: 'ziwei' | 'western';
  chart_name: string;
  birth_data: Record<string, any>;
  chart_data: Record<string, any>;
  ai_interpretation?: string;
  created_at: string;
}

export interface BirthData {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute?: number;
  gender: 'male' | 'female';
  timezone?: string;
}
