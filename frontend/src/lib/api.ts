import type { Interpretation } from '../types';

const API_URL = import.meta.env.VITE_API_URL || 'https://fortunet-api.yanggf.workers.dev';

export interface BirthData {
  birth_year: number;
  birth_month: number;
  birth_day: number;
  birth_hour?: number | null;
  birth_minute?: number | null;
  gender?: 'male' | 'female';
  timezone?: string;
}

class ApiClient {
  private sessionId: string | null = null;

  setSession(sessionId: string | null) {
    this.sessionId = sessionId;
    if (sessionId) localStorage.setItem('sessionId', sessionId);
    else localStorage.removeItem('sessionId');
  }

  getSession() {
    if (!this.sessionId) this.sessionId = localStorage.getItem('sessionId');
    return this.sessionId;
  }

  private async request(path: string, options: RequestInit = {}) {
    const sessionId = this.getSession();
    const headers: Record<string, string> = { 'Content-Type': 'application/json', ...(options.headers as Record<string, string>) };
    if (sessionId) headers['Authorization'] = `Bearer ${sessionId}`;

    const res = await fetch(`${API_URL}${path}`, { ...options, headers });
    if (!res.ok) throw new Error(await res.text());
    return res.json();
  }

  async register(email: string, fullName?: string) {
    const data = await this.request('/api/auth/register', {
      method: 'POST',
      body: JSON.stringify({ email, full_name: fullName })
    });
    this.setSession(data.sessionId);
    return data;
  }

  async login(email: string) {
    const data = await this.request('/api/auth/login', {
      method: 'POST',
      body: JSON.stringify({ email })
    });
    this.setSession(data.sessionId);
    return data;
  }

  async logout() {
    await this.request('/api/auth/logout', { method: 'POST' });
    this.setSession(null);
  }

  async getMe() {
    return this.request('/api/users/me');
  }

  // Update birth data
  async updateBirthData(data: BirthData) {
    return this.request('/api/users/me/birth', {
      method: 'PUT',
      body: JSON.stringify(data)
    });
  }

  // Get interpretations list
  async getInterpretations(): Promise<Interpretation[]> {
    const data = await this.request('/api/charts');
    return Array.isArray(data?.interpretations) ? data.interpretations : [];
  }

  // Get or calculate chart for divination type
  async getChart(type: 'ziwei' | 'western') {
    return this.request(`/api/charts/${type}`);
  }

  // Request AI interpretation
  async interpret(type: 'ziwei' | 'western') {
    return this.request(`/api/charts/${type}/interpret`, { method: 'POST' });
  }
}

export const api = new ApiClient();
