const API_URL = import.meta.env.VITE_API_URL || 'https://fortunet-api.yanggf.workers.dev';

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

  async getMe() {
    return this.request('/api/users/me');
  }

  async createChart(chartType: 'ziwei' | 'western', birthData: any) {
    // First calculate the chart
    const calcEndpoint = chartType === 'ziwei' ? '/api/charts/calculate/ziwei' : '/api/charts/calculate/western';
    const chartData = await this.request(calcEndpoint, {
      method: 'POST',
      body: JSON.stringify(birthData)
    });

    // Then save it
    return this.request('/api/charts', {
      method: 'POST',
      body: JSON.stringify({
        chart_type: chartType,
        chart_name: `${chartType} - ${new Date().toLocaleDateString()}`,
        birth_data: birthData,
        chart_data: chartData
      })
    });
  }

  async getCharts() {
    return this.request('/api/charts');
  }

  async getChart(id: string) {
    return this.request(`/api/charts/${id}`);
  }

  async interpretChart(chartId: string) {
    // Get the chart first
    const chart = await this.getChart(chartId);
    
    // Call interpret with correct payload
    const result = await this.request('/api/charts/interpret', {
      method: 'POST',
      body: JSON.stringify({
        chartType: chart.chart_type,
        chartData: chart.chart_data,
        language: 'zh'
      })
    });

    // Return the interpretation
    return { ...chart, ai_interpretation: result.interpretation };
  }
}

export const api = new ApiClient();
