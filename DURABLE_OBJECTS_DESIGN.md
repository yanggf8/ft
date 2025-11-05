# 🔧 Durable Objects Design & Implementation

## 📋 Overview

Complete design specification for Durable Objects implementation in FortuneT, providing stateful storage and real-time capabilities that complement D1 database operations.

## 🎯 Design Goals

- **Stateful Caching**: Intelligent caching of expensive calculations
- **Real-time Features**: WebSocket support for live collaboration
- **Session Management**: Efficient user session handling
- **Rate Limiting**: Sophisticated abuse prevention
- **Performance Optimization**: Sub-10ms access times for cached data

## 🏗️ Durable Objects Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    DURABLE OBJECTS CLUSTER                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────┐  │
│  │Session DO   │  │Cache DO     │  │RateLimit DO │  │Realtime  │  │
│  │(per user)   │  │(per chart)  │  │(per IP)     │  │DO        │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └──────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌──────────┐  │
│  │Analytics DO │  │AI Cache DO  │  │Chart DO     │  │Collab DO │  │
│  │(global)     │  │(per model)  │  │(per chart)  │  │(shared)  │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └──────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

## 🔧 Core Durable Objects Implementation

### 1. Session Manager Durable Object

```typescript
// src/durable-objects/session-manager.ts
export interface SessionData {
  userId: string;
  email: string;
  subscriptionStatus: 'free' | 'premium' | 'professional';
  preferences: UserPreferences;
  activeCharts: string[];
  lastActivity: number;
  deviceInfo: DeviceInfo[];
  wsConnections: Set<WebSocket>;
  permissions: string[];
  metadata: Record<string, any>;
}

export interface DeviceInfo {
  id: string;
  userAgent: string;
  ipAddress: string;
  location?: string;
  lastSeen: number;
  platform: string;
}

export class SessionManagerDurableObject {
  private sessions: Map<string, SessionData> = new Map();
  private userSessions: Map<string, Set<string>> = new Map();
  private analytics: Map<string, AnalyticsData> = new Map();

  constructor(private state: DurableObjectState, private env: Env) {
    this.loadPersistedSessions();
  }

  // Handle WebSocket connections
  async handleWebSocket(ws: WebSocket, userId: string, deviceInfo: DeviceInfo) {
    ws.accept();

    let sessionData = this.sessions.get(userId);
    if (!sessionData) {
      sessionData = await this.loadSessionFromD1(userId);
      this.sessions.set(userId, sessionData);
    }

    // Add WebSocket connection
    if (!sessionData.wsConnections) {
      sessionData.wsConnections = new Set();
    }
    sessionData.wsConnections.add(ws);

    // Update device info
    this.updateDeviceInfo(sessionData, deviceInfo);
    sessionData.lastActivity = Date.now();

    // Setup WebSocket handlers
    this.setupWebSocketHandlers(ws, userId);

    // Send initial state
    ws.send(JSON.stringify({
      type: 'session_connected',
      data: {
        user: {
          id: sessionData.userId,
          email: sessionData.email,
          subscriptionStatus: sessionData.subscriptionStatus
        },
        activeCharts: sessionData.activeCharts,
        preferences: sessionData.preferences
      }
    }));

    return ws;
  }

  // Session CRUD operations
  async createSession(userId: string, userData: Partial<SessionData>): Promise<SessionData> {
    const session: SessionData = {
      userId,
      email: userData.email!,
      subscriptionStatus: userData.subscriptionStatus || 'free',
      preferences: userData.preferences || this.getDefaultPreferences(),
      activeCharts: [],
      lastActivity: Date.now(),
      deviceInfo: [],
      wsConnections: new Set(),
      permissions: this.getUserPermissions(userData.subscriptionStatus || 'free'),
      metadata: userData.metadata || {}
    };

    this.sessions.set(userId, session);

    // Track user sessions
    const userSessionSet = this.userSessions.get(userId) || new Set();
    userSessionSet.add(this.state.id.toString());
    this.userSessions.set(userId, userSessionSet);

    // Persist to storage
    await this.state.storage.put(`session:${userId}`, session);
    await this.state.storage.put(`analytics:${userId}`, {
      sessions: Array.from(userSessionSet),
      lastActivity: Date.now()
    });

    return session;
  }

  async getSession(userId: string): Promise<SessionData | null> {
    let session = this.sessions.get(userId);
    if (!session) {
      // Try loading from storage
      const stored = await this.state.storage.get<SessionData>(`session:${userId}`);
      if (stored) {
        session = stored;
        session.wsConnections = new Set(); // Don't persist WebSockets
        this.sessions.set(userId, session);
      }
    }
    return session || null;
  }

  async updateSession(userId: string, updates: Partial<SessionData>): Promise<void> {
    const session = await this.getSession(userId);
    if (session) {
      Object.assign(session, updates);
      session.lastActivity = Date.now();
      await this.state.storage.put(`session:${userId}`, session);

      // Broadcast to user's connected clients
      if (updates.activeCharts || updates.preferences) {
        this.broadcastToUser(userId, {
          type: 'session_updated',
          data: updates
        });
      }
    }
  }

  async invalidateSession(userId: string): Promise<void> {
    const session = this.sessions.get(userId);
    if (session) {
      // Close all WebSocket connections
      session.wsConnections.forEach(ws => {
        ws.close(1000, 'Session invalidated');
      });
      session.wsConnections.clear();
    }

    this.sessions.delete(userId);
    await this.state.storage.delete(`session:${userId}`);

    // Update user sessions tracking
    const userSessionSet = this.userSessions.get(userId);
    if (userSessionSet) {
      userSessionSet.delete(this.state.id.toString());
      if (userSessionSet.size === 0) {
        this.userSessions.delete(userId);
      }
    }
  }

  // Real-time messaging
  async broadcastToUser(userId: string, message: any): Promise<void> {
    const session = this.sessions.get(userId);
    if (session) {
      const messageStr = JSON.stringify(message);
      session.wsConnections.forEach(ws => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(messageStr);
        }
      });
    }
  }

  private setupWebSocketHandlers(ws: WebSocket, userId: string): void {
    ws.addEventListener('message', async (event) => {
      try {
        const message = JSON.parse(event.data);
        await this.handleWebSocketMessage(ws, userId, message);
      } catch (error) {
        console.error('Invalid WebSocket message:', error);
        ws.send(JSON.stringify({
          type: 'error',
          message: 'Invalid message format'
        }));
      }
    });

    ws.addEventListener('close', async () => {
      const session = this.sessions.get(userId);
      if (session) {
        session.wsConnections.delete(ws);

        // Clean up empty sessions
        if (session.wsConnections.size === 0) {
          await this.saveSessionToStorage(userId, session);
        }
      }
    });

    // Heartbeat to detect disconnections
    const heartbeatInterval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'heartbeat' }));
      } else {
        clearInterval(heartbeatInterval);
      }
    }, 30000); // 30 seconds
  }

  private async handleWebSocketMessage(ws: WebSocket, userId: string, message: any): Promise<void> {
    switch (message.type) {
      case 'heartbeat':
        await this.updateSession(userId, { lastActivity: Date.now() });
        ws.send(JSON.stringify({ type: 'heartbeat_ack' }));
        break;

      case 'update_preferences':
        await this.updateSession(userId, { preferences: message.preferences });
        ws.send(JSON.stringify({
          type: 'preferences_updated',
          preferences: message.preferences
        }));
        break;

      case 'add_chart':
        const session = await this.getSession(userId);
        if (session && !session.activeCharts.includes(message.chartId)) {
          session.activeCharts.push(message.chartId);
          await this.updateSession(userId, { activeCharts: session.activeCharts });
        }
        break;

      case 'chart_update':
        // Broadcast chart updates to user's other sessions
        this.broadcastToUser(userId, {
          type: 'chart_updated',
          data: message.data
        });
        break;
    }
  }

  private async loadSessionFromD1(userId: string): Promise<SessionData> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.subscription_status, up.preferences, up.stripe_customer_id
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.id = ?
    `).bind(userId).first();

    if (!result) {
      throw new Error(`User ${userId} not found`);
    }

    return {
      userId: result.id,
      email: result.email,
      subscriptionStatus: result.subscription_status || 'free',
      preferences: JSON.parse(result.preferences || '{}'),
      activeCharts: [],
      lastActivity: Date.now(),
      deviceInfo: [],
      wsConnections: new Set(),
      permissions: this.getUserPermissions(result.subscription_status || 'free'),
      metadata: {}
    };
  }

  private getUserPermissions(subscriptionStatus: string): string[] {
    const basePermissions = [
      'charts:read',
      'charts:create',
      'profile:read',
      'profile:update'
    ];

    if (subscriptionStatus === 'premium') {
      basePermissions.push(
        'ai:limited',
        'charts:export',
        'charts:basic'
      );
    } else if (subscriptionStatus === 'professional') {
      basePermissions.push(
        'ai:unlimited',
        'charts:export',
        'charts:collaborate',
        'analytics:advanced'
      );
    }

    return basePermissions;
  }

  private getDefaultPreferences(): UserPreferences {
    return {
      language: 'zh-TW',
      timezone: 'Asia/Taipei',
      theme: 'light',
      notifications: {
        email: true,
        push: false,
        astrology_insights: true,
        subscription_reminders: true
      },
      astrology: {
        default_system: 'ziwei',
        chart_preferences: {
          show_transitions: true,
          include_yearly_prediction: false,
          detail_level: 'comprehensive'
        },
        ai_settings: {
          analysis_depth: 'detailed',
          language: 'traditional_chinese'
        }
      }
    };
  }

  private updateDeviceInfo(session: SessionData, deviceInfo: DeviceInfo): void {
    // Remove old device info for this device
    session.deviceInfo = session.deviceInfo.filter(
      device => device.id !== deviceInfo.id
    );

    // Add updated device info
    session.deviceInfo.push({
      ...deviceInfo,
      lastSeen: Date.now()
    });

    // Keep only last 5 devices
    session.deviceInfo = session.deviceInfo
      .sort((a, b) => b.lastSeen - a.lastSeen)
      .slice(0, 5);
  }

  private async saveSessionToStorage(userId: string, session: SessionData): Promise<void> {
    // Don't store WebSockets
    const sessionToStore = {
      ...session,
      wsConnections: undefined
    };

    await this.state.storage.put(`session:${userId}`, sessionToStore);
  }

  private async loadPersistedSessions(): Promise<void> {
    // Load sessions from persistent storage on startup
    const sessionKeys = await this.state.storage.list({
      prefix: 'session:'
    });

    for (const key of sessionKeys.keys) {
      const sessionData = await this.state.storage.get(key.name);
      if (sessionData) {
        const session: SessionData = {
          ...sessionData,
          wsConnections: new Set()
        };
        this.sessions.set(session.userId, session);
      }
    }
  }
}
```

### 2. Chart Cache Durable Object

```typescript
// src/durable-objects/chart-cache.ts
export interface ChartCacheEntry {
  id: string;
  chartType: 'ziwei' | 'zodiac';
  birthData: any;
  calculatedData: any;
  aiAnalysis?: any;
  chartImageUrl?: string;
  calculationTime: number;
  accessCount: number;
  lastAccessed: number;
  createdAt: number;
  expiresAt: number;
  metadata: Record<string, any>;
}

export interface CacheStats {
  totalEntries: number;
  memoryUsage: number;
  hitRate: number;
  totalRequests: number;
  cacheHits: number;
  avgCalculationTime: number;
  expiredEntries: number;
}

export class ChartCacheDurableObject {
  private cache: Map<string, ChartCacheEntry> = new Map();
  private stats: CacheStats = {
    totalEntries: 0,
    memoryUsage: 0,
    hitRate: 0,
    totalRequests: 0,
    cacheHits: 0,
    avgCalculationTime: 0,
    expiredEntries: 0
  };

  constructor(private state: DurableObjectState, private env: Env) {
    this.loadFromStorage();
    this.startCleanupInterval();
  }

  async getChart(chartKey: string): Promise<ChartCacheEntry | null> {
    this.stats.totalRequests++;

    let entry = this.cache.get(chartKey);

    if (!entry) {
      // Try persistent storage
      entry = await this.state.storage.get<ChartCacheEntry>(`chart:${chartKey}`);
      if (entry) {
        this.cache.set(chartKey, entry);
      }
    }

    if (entry) {
      // Check TTL
      if (Date.now() > entry.expiresAt) {
        this.invalidateChart(chartKey);
        return null;
      }

      this.stats.cacheHits++;
      entry.lastAccessed = Date.now();
      entry.accessCount++;

      // Update access tracking
      await this.state.storage.put(`chart:${chartKey}`, entry);
    }

    this.updateHitRate();
    return entry || null;
  }

  async cacheChart(
    chartKey: string,
    chartData: ChartCacheEntry,
    ttl: number = 86400000 // 24 hours default
  ): Promise<void> {
    const entry: ChartCacheEntry = {
      ...chartData,
      id: chartKey,
      createdAt: Date.now(),
      lastAccessed: Date.now(),
      accessCount: 0,
      expiresAt: Date.now() + ttl
    };

    this.cache.set(chartKey, entry);
    await this.state.storage.put(`chart:${chartKey}`, entry);

    this.stats.totalEntries++;
    this.updateMemoryUsage();

    // Trigger cleanup if needed
    await this.cleanupExpiredEntries();
  }

  async invalidateChart(chartKey: string): Promise<void> {
    this.cache.delete(chartKey);
    await this.state.storage.delete(`chart:${chartKey}`);
    this.stats.totalEntries = Math.max(0, this.stats.totalEntries - 1);
    this.updateMemoryUsage();
  }

  async invalidateByPattern(pattern: string): Promise<number> {
    let deleted = 0;
    const keysToDelete: string[] = [];

    for (const [key] of this.cache) {
      if (key.includes(pattern)) {
        keysToDelete.push(key);
      }
    }

    for (const key of keysToDelete) {
      await this.invalidateChart(key);
      deleted++;
    }

    return deleted;
  }

  // Batch operations for performance
  async getBatch(chartKeys: string[]): Promise<Map<string, ChartCacheEntry | null>> {
    const results = new Map<string, ChartCacheEntry | null>();

    const promises = chartKeys.map(async (key) => {
      const entry = await this.getChart(key);
      return [key, entry];
    });

    const entries = await Promise.all(promises);
    entries.forEach(([key, entry]) => results.set(key, entry));

    return results;
  }

  // Cache statistics and monitoring
  async getStats(): Promise<CacheStats> {
    return { ...this.stats };
  }

  async clearCache(): Promise<void> {
    this.cache.clear();
    await this.state.storage.deleteAll();
    this.stats = {
      totalEntries: 0,
      memoryUsage: 0,
      hitRate: 0,
      totalRequests: 0,
      cacheHits: 0,
      avgCalculationTime: 0,
      expiredEntries: 0
    };
  }

  // AI Analysis caching with longer TTL
  async cacheAIAnalysis(
    chartHash: string,
    analysis: any,
    model: string,
    cost: number = 0
  ): Promise<void> {
    const key = `ai_analysis:${model}:${chartHash}`;
    const entry: ChartCacheEntry = {
      id: key,
      chartType: 'ai_analysis' as any,
      birthData: null,
      calculatedData: analysis,
      calculationTime: 0,
      createdAt: Date.now(),
      lastAccessed: Date.now(),
      accessCount: 0,
      expiresAt: Date.now() + (86400000 * 7), // 7 days
      metadata: { model, cost }
    };

    await this.state.storage.put(key, entry);

    // Update AI-specific stats
    this.stats.totalEntries++;
    this.updateMemoryUsage();
  }

  async getAIAnalysis(chartHash: string, model: string): Promise<any | null> {
    const key = `ai_analysis:${model}:${chartHash}`;
    const entry = await this.state.storage.get<ChartCacheEntry>(key);

    if (entry && Date.now() <= entry.expiresAt) {
      entry.lastAccessed = Date.now();
      entry.accessCount++;
      await this.state.storage.put(key, entry);
      return entry.calculatedData;
    }

    return null;
  }

  private async cleanupExpiredEntries(): Promise<void> {
    const now = Date.now();
    const expiredKeys: string[] = [];

    for (const [key, entry] of this.cache) {
      if (now > entry.expiresAt) {
        expiredKeys.push(key);
      }
    }

    for (const key of expiredKeys) {
      this.cache.delete(key);
      await this.state.storage.delete(`chart:${key}`);
      this.stats.totalEntries--;
      this.stats.expiredEntries++;
    }

    this.updateMemoryUsage();
  }

  private updateHitRate(): void {
    this.stats.hitRate = this.stats.totalRequests > 0
      ? this.stats.cacheHits / this.stats.totalRequests
      : 0;
  }

  private updateMemoryUsage(): void {
    // Estimate memory usage based on cache size
    const avgEntrySize = 1024; // 1KB average per entry
    this.stats.memoryUsage = this.stats.totalEntries * avgEntrySize;
  }

  private startCleanupInterval(): void {
    // Cleanup expired entries every hour
    setInterval(async () => {
      await this.cleanupExpiredEntries();
    }, 3600000); // 1 hour
  }

  private async loadFromStorage(): Promise<void> {
    // Load cache statistics
    const storedStats = await this.state.storage.get<CacheStats>('cache_stats');
    if (storedStats) {
      this.stats = storedStats;
    }

    // Clean up expired entries on startup
    await this.cleanupExpiredEntries();
  }
}
```

### 3. Rate Limiter Durable Object

```typescript
// src/durable-objects/rate-limiter.ts
export interface RateLimitRule {
  windowMs: number;
  maxRequests: number;
  skipSuccessfulRequests?: boolean;
  skipFailedRequests?: boolean;
}

export interface RateLimitResult {
  allowed: boolean;
  remaining: number;
  resetTime: number;
  retryAfter?: number;
  totalHits: number;
  planType: 'free' | 'premium' | 'professional';
}

export class RateLimiterDurableObject {
  private limits: Map<string, RateLimitRule> = new Map();
  private requests: Map<string, number[]> = new Map();
  private analytics: Map<string, AnalyticsData> = new Map();

  constructor(private state: DurableObjectState, private env: Env) {
    this.initializeDefaultRules();
    this.startCleanupInterval();
  }

  private initializeDefaultRules(): void {
    // Default rate limiting rules
    this.limits.set('default', { windowMs: 3600000, maxRequests: 100 }); // 100/hour
    this.limits.set('ai_analysis', { windowMs: 3600000, maxRequests: 10 }); // 10 AI analyses/hour
    this.limits.set('chart_calculation', { windowMs: 3600000, maxRequests: 50 }); // 50 charts/hour
    this.limits.set('premium_user', { windowMs: 3600000, maxRequests: 1000 }); // 1000/hour for premium
    this.limits.set('professional_user', { windowMs: 3600000, maxRequests: 5000 }); // 5000/hour for professional
    this.limits.set('api_upload', { windowMs: 86400000, maxRequests: 100 }); // 100 uploads/day
    this.limits.set('export_download', { windowMs: 86400000, maxRequests: 50 }); // 50 exports/day
  }

  async checkLimit(
    identifier: string,
    endpoint: string,
    planType: 'free' | 'premium' | 'professional' = 'free'
  ): Promise<RateLimitResult> {
    const ruleKey = this.getRuleKey(endpoint, planType);
    const rule = this.limits.get(ruleKey) || this.limits.get('default')!;

    const key = `${identifier}:${endpoint}`;
    const now = Date.now();
    const windowStart = now - rule.windowMs;

    // Get existing requests for this identifier
    let timestamps = this.requests.get(key) || [];

    // Filter out old requests outside the window
    timestamps = timestamps.filter(timestamp => timestamp > windowStart);

    // Add current request
    timestamps.push(now);

    // Store updated timestamps
    this.requests.set(key, timestamps);
    await this.state.storage.put(`rate_limit:${key}`, {
      timestamps,
      ruleKey,
      lastUpdated: now,
      planType
    });

    const totalHits = timestamps.length;
    const remaining = Math.max(0, rule.maxRequests - totalHits);
    const allowed = totalHits <= rule.maxRequests;
    const resetTime = Math.min(...timestamps) + rule.windowMs;

    // Update analytics
    this.updateAnalytics(identifier, endpoint, planType, totalHits);

    const result: RateLimitResult = {
      allowed,
      remaining,
      resetTime,
      totalHits,
      planType
    };

    if (!allowed) {
      result.retryAfter = Math.ceil((resetTime - now) / 1000);
    }

    return result;
  }

  async getUsage(identifier: string, endpoint: string): Promise<{
    current: number;
    limit: number;
    windowMs: number;
    resetTime: number;
    planType: string;
  }> {
    const key = `${identifier}:${endpoint}`;
    const stored = await this.state.storage.get<{
      timestamps: number[];
      ruleKey: string;
      planType: string;
    }>(`rate_limit:${key}`);

    if (!stored) {
      return {
        current: 0,
        limit: 100,
        windowMs: 3600000,
        resetTime: Date.now() + 3600000,
        planType: 'free'
      };
    }

    const rule = this.limits.get(stored.ruleKey) || this.limits.get('default')!;
    const now = Date.now();
    const windowStart = now - rule.windowMs;

    const currentTimestamps = stored.timestamps.filter(t => t > windowStart);

    return {
      current: currentTimestamps.length,
      limit: rule.maxRequests,
      windowMs: rule.windowMs,
      resetTime: Math.min(...currentTimestamps) + rule.windowMs,
      planType: stored.planType
    };
  }

  async resetLimit(identifier: string, endpoint: string): Promise<void> {
    const key = `${identifier}:${endpoint}`;
    this.requests.delete(key);
    await this.state.storage.delete(`rate_limit:${key}`);
  }

  async upgradeLimit(identifier: string, planType: 'premium' | 'professional'): Promise<void> {
    // Update all existing limits for user
    const keysToUpdate: string[] = [];

    for (const [key] of this.requests) {
      if (key.startsWith(identifier)) {
        keysToUpdate.push(key);
      }
    }

    for (const key of keysToUpdate) {
      const stored = await this.state.storage.get<{
        timestamps: number[];
        ruleKey: string;
      }>(`rate_limit:${key}`);

      if (stored) {
        const newRuleKey = this.getRuleKeyForPlan(key.split(':')[1], planType);
        await this.state.storage.put(`rate_limit:${key}`, {
          ...stored,
          ruleKey: newRuleKey,
          planType,
          lastUpdated: Date.now()
        });
      }
    }
  }

  // Admin functions
  async blockIdentifier(identifier: string, durationMs: number = 86400000): Promise<void> {
    // Temporary block for abusive users
    const blockRule: RateLimitRule = {
      windowMs: durationMs,
      maxRequests: 0
    };

    await this.state.storage.put(`block:${identifier}`, {
      until: Date.now() + durationMs,
      rule: blockRule
    });
  }

  async isBlocked(identifier: string): Promise<boolean> {
    const block = await this.state.storage.get<{
      until: number;
      rule: RateLimitRule;
    }>(`block:${identifier}`);

    if (block && block.until > Date.now()) {
      return true;
    }

    if (block && block.until <= Date.now()) {
      await this.state.storage.delete(`block:${identifier}`);
    }

    return false;
  }

  // Analytics and reporting
  async getAnalytics(timeRange: string = '24h'): Promise<AnalyticsReport> {
    const now = Date.now();
    const startTime = this.getStartTimeRange(timeRange, now);

    let totalRequests = 0;
    let blockedRequests = 0;
    const topUsers = new Map<string, number>();
    const topEndpoints = new Map<string, number>();

    for (const [key, analytics] of this.analytics) {
      if (analytics.lastActivity > startTime) {
        totalRequests += analytics.totalRequests;
        blockedRequests += analytics.blockedRequests;

        // Track top users
        const userId = key.split(':')[0];
        topUsers.set(userId, (topUsers.get(userId) || 0) + analytics.totalRequests);

        // Track top endpoints
        for (const [endpoint, count] of Object.entries(analytics.endpointUsage)) {
          topEndpoints.set(endpoint, (topEndpoints.get(endpoint) || 0) + count);
        }
      }
    }

    return {
      timeRange,
      totalRequests,
      blockedRequests,
      blockRate: totalRequests > 0 ? blockedRequests / totalRequests : 0,
      topUsers: Array.from(topUsers.entries())
        .sort(([,a], [,b]) => b - a)
        .slice(0, 10),
      topEndpoints: Array.from(topEndpoints.entries())
        .sort(([,a], [,b]) => b - a)
        .slice(0, 10),
      generatedAt: new Date(now).toISOString()
    };
  }

  private getRuleKey(endpoint: string, planType: string): string {
    if (planType === 'premium') return 'premium_user';
    if (planType === 'professional') return 'professional_user';

    if (endpoint.includes('/ai/')) return 'ai_analysis';
    if (endpoint.includes('/calculate')) return 'chart_calculation';
    if (endpoint.includes('/upload')) return 'api_upload';
    if (endpoint.includes('/export') || endpoint.includes('/download')) return 'export_download';

    return 'default';
  }

  private getRuleKeyForPlan(endpoint: string, planType: string): string {
    return this.getRuleKey(endpoint, planType);
  }

  private updateAnalytics(
    identifier: string,
    endpoint: string,
    planType: string,
    requestCount: number
  ): void {
    const key = `${identifier}:${planType}`;
    let analytics = this.analytics.get(key);

    if (!analytics) {
      analytics = {
        identifier,
        planType,
        totalRequests: 0,
        blockedRequests: 0,
        endpointUsage: {},
        lastActivity: Date.now()
      };
      this.analytics.set(key, analytics);
    }

    analytics.totalRequests += requestCount;
    analytics.endpointUsage[endpoint] = (analytics.endpointUsage[endpoint] || 0) + requestCount;
    analytics.lastActivity = Date.now();

    // Persist analytics periodically
    if (analytics.totalRequests % 10 === 0) {
      this.state.storage.put(`analytics:${key}`, analytics);
    }
  }

  private getStartTimeRange(timeRange: string, now: number): number {
    const ranges = {
      '1h': 3600000,
      '24h': 86400000,
      '7d': 604800000,
      '30d': 2592000000
    };

    return now - (ranges[timeRange as keyof typeof ranges] || ranges['24h']);
  }

  private startCleanupInterval(): void {
    // Cleanup old data every hour
    setInterval(async () => {
      await this.cleanupExpiredData();
    }, 3600000);
  }

  private async cleanupExpiredData(): Promise<void> {
    const now = Date.now();
    const windowMs = 3600000; // 1 hour window

    // Clean up old request timestamps
    for (const [key, timestamps] of this.requests) {
      const validTimestamps = timestamps.filter(t => now - t < windowMs);
      if (validTimestamps.length !== timestamps.length) {
        this.requests.set(key, validTimestamps);
        if (validTimestamps.length === 0) {
          this.requests.delete(key);
        }
      }
    }

    // Clean up old analytics
    const cutoffTime = now - (86400000 * 7); // 7 days
    for (const [key, analytics] of this.analytics) {
      if (analytics.lastActivity < cutoffTime) {
        this.analytics.delete(key);
        this.state.storage.delete(`analytics:${key}`);
      }
    }
  }
}

interface AnalyticsData {
  identifier: string;
  planType: string;
  totalRequests: number;
  blockedRequests: number;
  endpointUsage: Record<string, number>;
  lastActivity: number;
}

interface AnalyticsReport {
  timeRange: string;
  totalRequests: number;
  blockedRequests: number;
  blockRate: number;
  topUsers: Array<[string, number]>;
  topEndpoints: Array<[string, number]>;
  generatedAt: string;
}
```

### 4. Real-time Collaboration Durable Object

```typescript
// src/durable-objects/realtime-collaboration.ts
export interface CollaborationSession {
  id: string;
  chartId: string;
  participants: Map<string, Participant>;
  sharedState: any;
  permissions: {
    public: boolean;
    allowedUsers: string[];
    editPassword?: string;
  };
  created: number;
  lastActivity: number;
  version: number;
}

export interface Participant {
  id: string;
  userId: string;
  name: string;
  role: 'owner' | 'editor' | 'viewer';
  cursor?: { x: number; y: number };
  selection?: any;
  ws: WebSocket;
  joinedAt: number;
  lastSeen: number;
  permissions: string[];
  color: string; // User color for collaboration
}

export interface RealtimeEvent {
  type: 'cursor_move' | 'selection_change' | 'chart_update' | 'analysis_request' |
        'participant_join' | 'participant_leave' | 'state_sync' | 'version_conflict';
  userId: string;
  data: any;
  timestamp: number;
  version?: number;
}

export class RealtimeCollaborationDurableObject {
  private sessions: Map<string, CollaborationSession> = new Map();
  private userSessions: Map<string, Set<string>> = new Map();
  private chartSessions: Map<string, string> = new Map(); // chartId -> sessionId

  constructor(private state: DurableObjectState, private env: Env) {
    this.loadPersistedSessions();
    this.startCleanupInterval();
  }

  async joinSession(
    chartId: string,
    userId: string,
    userName: string,
    ws: WebSocket,
    role: 'owner' | 'editor' | 'viewer' = 'viewer',
    password?: string
  ): Promise<CollaborationSession> {
    const sessionId = this.getSessionId(chartId);
    let session = this.sessions.get(sessionId);

    if (!session) {
      session = await this.createSession(chartId, userId);
    } else {
      // Validate permissions
      if (!await this.validateSessionAccess(session, userId, role, password)) {
        ws.close(4003, 'Access denied');
        throw new Error('Access denied');
      }
    }

    const participant: Participant = {
      id: crypto.randomUUID(),
      userId,
      name: userName,
      role,
      ws,
      joinedAt: Date.now(),
      lastSeen: Date.now(),
      permissions: this.getParticipantPermissions(role),
      color: this.generateUserColor(session.participants.size)
    };

    session.participants.set(participant.id, participant);
    session.lastActivity = Date.now();
    session.version++;

    // Track user's active sessions
    const userSessionSet = this.userSessions.get(userId) || new Set();
    userSessionSet.add(sessionId);
    this.userSessions.set(userId, userSessionSet);

    // Setup WebSocket handlers
    this.setupWebSocketHandlers(session, participant);

    // Notify other participants
    this.broadcastToSession(session, {
      type: 'participant_join',
      userId,
      data: {
        participantId: participant.id,
        name: userName,
        role,
        color: participant.color,
        participantCount: session.participants.size
      },
      timestamp: Date.now(),
      version: session.version
    }, participant.id);

    // Send current session state to new participant
    ws.send(JSON.stringify({
      type: 'session_joined',
      data: {
        sessionId,
        participants: Array.from(session.participants.values()).map(p => ({
          id: p.id,
          userId: p.userId,
          name: p.name,
          role: p.role,
          color: p.color,
          cursor: p.cursor,
          isOnline: true
        })),
        sharedState: session.sharedState,
        version: session.version,
        permissions: session.permissions
      }
    }));

    await this.persistSession(session);
    return session;
  }

  async leaveSession(sessionId: string, participantId: string): Promise<void> {
    const session = this.sessions.get(sessionId);
    if (!session) return;

    const participant = session.participants.get(participantId);
    if (participant) {
      // Update user sessions tracking
      const userSessionSet = this.userSessions.get(participant.userId);
      if (userSessionSet) {
        userSessionSet.delete(sessionId);
        if (userSessionSet.size === 0) {
          this.userSessions.delete(participant.userId);
        }
      }

      session.participants.delete(participantId);
      session.lastActivity = Date.now();
      session.version++;

      // Notify remaining participants
      this.broadcastToSession(session, {
        type: 'participant_leave',
        userId: participant.userId,
        data: {
          participantId,
          participantCount: session.participants.size
        },
        timestamp: Date.now(),
        version: session.version
      });

      // Clean up empty sessions
      if (session.participants.size === 0) {
        this.sessions.delete(sessionId);
        this.chartSessions.delete(session.chartId);
        await this.state.storage.delete(`session:${sessionId}`);
      } else {
        await this.persistSession(session);
      }
    }
  }

  async handleRealtimeEvent(sessionId: string, participantId: string, event: RealtimeEvent): Promise<void> {
    const session = this.sessions.get(sessionId);
    const participant = session?.participants.get(participantId);

    if (!session || !participant) return;

    // Validate permissions
    if (!this.validateEventPermission(participant, event)) {
      participant.ws.send(JSON.stringify({
        type: 'error',
        message: 'Insufficient permissions for this operation'
      }));
      return;
    }

    participant.lastSeen = Date.now();
    session.lastActivity = Date.now();
    session.version++;

    switch (event.type) {
      case 'cursor_move':
        participant.cursor = event.data;
        event.version = session.version;
        break;

      case 'selection_change':
        participant.selection = event.data;
        event.version = session.version;
        break;

      case 'chart_update':
        // Update shared state and persist
        session.sharedState = { ...session.sharedState, ...event.data };
        event.version = session.version;
        await this.persistSession(session);

        // Trigger async save to D1
        this.saveChartToD1(session.chartId, session.sharedState);
        break;

      case 'analysis_request':
        // Handle AI analysis requests collaboratively
        await this.handleCollaborativeAnalysis(session, participant, event.data);
        return; // Don't broadcast this event

      case 'state_sync':
        // Handle version conflicts
        await this.handleVersionConflict(session, participant, event);
        return;
    }

    // Broadcast to other participants
    this.broadcastToSession(session, event, participant.id);
  }

  async startSharedAnalysis(
    sessionId: string,
    participantId: string,
    analysisType: string
  ): Promise<void> {
    const session = this.sessions.get(sessionId);
    if (!session) return;

    // Notify all participants that analysis is starting
    this.broadcastToSession(session, {
      type: 'analysis_request',
      userId: session.participants.get(participantId)?.userId || '',
      data: {
        analysisType,
        status: 'starting',
        initiatedBy: session.participants.get(participantId)?.name
      },
      timestamp: Date.now()
    });

    // Trigger AI analysis in background
    this.triggerAIAnalysis(session, analysisType);
  }

  async shareChart(
    sessionId: string,
    participantId: string,
    shareSettings: {
      isPublic: boolean;
      allowedUsers?: string[];
      editPassword?: string;
      expiresAt?: number;
    }
  ): Promise<string> {
    const session = this.sessions.get(sessionId);
    if (!session) throw new Error('Session not found');

    const participant = session.participants.get(participantId);
    if (!participant || participant.role !== 'owner') {
      throw new Error('Only session owners can change sharing settings');
    }

    // Update session permissions
    session.permissions = shareSettings;
    session.lastActivity = Date.now();

    // Generate share link if public
    let shareLink = null;
    if (shareSettings.isPublic) {
      shareLink = this.generateShareLink(sessionId);
      await this.state.storage.put(`share:${shareLink}`, {
        sessionId,
        permissions: shareSettings,
        createdAt: Date.now()
      });
    }

    await this.persistSession(session);

    // Notify participants
    this.broadcastToSession(session, {
      type: 'sharing_updated',
      userId: participant.userId,
      data: {
        isPublic: shareSettings.isPublic,
        shareLink,
        participantCount: session.participants.size
      },
      timestamp: Date.now()
    });

    return shareLink;
  }

  private async createSession(chartId: string, ownerId: string): Promise<CollaborationSession> {
    const sessionId = this.getSessionId(chartId);
    const session: CollaborationSession = {
      id: sessionId,
      chartId,
      participants: new Map(),
      sharedState: {},
      permissions: {
        public: false,
        allowedUsers: [ownerId]
      },
      created: Date.now(),
      lastActivity: Date.now(),
      version: 1
    };

    this.sessions.set(sessionId, session);
    this.chartSessions.set(chartId, sessionId);
    await this.persistSession(session);
    return session;
  }

  private setupWebSocketHandlers(session: CollaborationSession, participant: Participant): void {
    const ws = participant.ws;

    ws.addEventListener('message', async (event) => {
      try {
        const message = JSON.parse(event.data);
        await this.handleRealtimeEvent(session.id, participant.id, message);
      } catch (error) {
        console.error('Invalid message format:', error);
        ws.send(JSON.stringify({
          type: 'error',
          message: 'Invalid message format'
        }));
      }
    });

    ws.addEventListener('close', async () => {
      await this.leaveSession(session.id, participant.id);
    });

    // Heartbeat to detect disconnections
    const heartbeatInterval = setInterval(() => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'heartbeat' }));
      } else {
        clearInterval(heartbeatInterval);
        this.leaveSession(session.id, participant.id);
      }
    }, 30000); // 30 seconds
  }

  private broadcastToSession(
    session: CollaborationSession,
    event: RealtimeEvent,
    excludeParticipantId?: string
  ): void {
    const message = JSON.stringify(event);

    session.participants.forEach((participant) => {
      if (participant.id !== excludeParticipantId && participant.ws.readyState === WebSocket.OPEN) {
        participant.ws.send(message);
      }
    });
  }

  private async handleCollaborativeAnalysis(
    session: CollaborationSession,
    participant: Participant,
    analysisData: any
  ): Promise<void> {
    // Notify that analysis is in progress
    this.broadcastToSession(session, {
      type: 'analysis_request',
      userId: participant.userId,
      data: {
        status: 'processing',
        initiatedBy: participant.name
      },
      timestamp: Date.now()
    });

    try {
      // Trigger AI analysis (this would call your AI service)
      const analysis = await this.triggerAIAnalysis(session, analysisData.type);

      // Update shared state with analysis
      session.sharedState.aiAnalysis = analysis;
      session.version++;

      // Broadcast results
      this.broadcastToSession(session, {
        type: 'analysis_complete',
        userId: participant.userId,
        data: {
          status: 'completed',
          result: analysis,
          version: session.version
        },
        timestamp: Date.now(),
        version: session.version
      });

      await this.persistSession(session);

    } catch (error) {
      this.broadcastToSession(session, {
        type: 'analysis_error',
        userId: participant.userId,
        data: {
          status: 'failed',
          error: error.message
        },
        timestamp: Date.now()
      });
    }
  }

  private async handleVersionConflict(
    session: CollaborationSession,
    participant: Participant,
    event: RealtimeEvent
  ): Promise<void> {
    // Version conflict resolution logic
    if (event.version && event.version < session.version) {
      // Send current state to requesting participant
      participant.ws.send(JSON.stringify({
        type: 'state_sync_response',
        data: {
          sharedState: session.sharedState,
          version: session.version,
          conflict: true
        }
      }));
    }
  }

  private validateSessionAccess(
    session: CollaborationSession,
    userId: string,
    role: string,
    password?: string
  ): boolean {
    // Check if user is in allowed list
    if (session.permissions.allowedUsers.length > 0 &&
        !session.permissions.allowedUsers.includes(userId)) {
      return false;
    }

    // Check password protection
    if (session.permissions.editPassword &&
        session.permissions.editPassword !== password) {
      return false;
    }

    // Check public access
    if (session.permissions.public && role === 'viewer') {
      return true;
    }

    return session.permissions.allowedUsers.includes(userId);
  }

  private validateEventPermission(participant: Participant, event: RealtimeEvent): boolean {
    const writeEvents = ['chart_update', 'analysis_request', 'sharing_updated'];
    const isWriteEvent = writeEvents.includes(event.type);

    if (isWriteEvent && !participant.permissions.includes('write')) {
      return false;
    }

    return true;
  }

  private getParticipantPermissions(role: string): string[] {
    const permissions = {
      owner: ['read', 'write', 'share', 'delete'],
      editor: ['read', 'write'],
      viewer: ['read']
    };

    return permissions[role as keyof typeof permissions] || ['read'];
  }

  private generateUserColor(userCount: number): string {
    const colors = [
      '#FF6B6B', '#4ECDC4', '#45B7D1', '#96CEB4', '#FFEAA7',
      '#DDA0DD', '#98D8C8', '#F7DC6F', '#F8B739', '#C7ECEE'
    ];

    return colors[userCount % colors.length];
  }

  private getSessionId(chartId: string): string {
    return `chart_${chartId}`;
  }

  private generateShareLink(sessionId: string): string {
    return crypto.randomUUID().replace(/-/g, '');
  }

  private async saveChartToD1(chartId: string, state: any): Promise<void> {
    // Background save to D1 database
    try {
      await this.env.DB.prepare(`
        UPDATE charts SET chart_data = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
      `).bind(JSON.stringify(state), chartId).run();
    } catch (error) {
      console.error('Failed to save chart to D1:', error);
    }
  }

  private async persistSession(session: CollaborationSession): Promise<void> {
    const serializableSession = {
      ...session,
      participants: Array.from(session.participants.entries()).map(([id, participant]) => [
        id,
        {
          ...participant,
          ws: undefined // Don't serialize WebSocket
        }
      ])
    };

    await this.state.storage.put(`session:${session.id}`, serializableSession);
  }

  private async loadPersistedSessions(): Promise<void> {
    // Load sessions from persistent storage on startup
    const sessionKeys = await this.state.storage.list({
      prefix: 'session:'
    });

    for (const key of sessionKeys.keys) {
      const sessionData = await this.state.storage.get(key.name);
      if (sessionData) {
        // Restore session structure (without WebSockets)
        const session: CollaborationSession = {
          ...sessionData,
          participants: new Map(sessionData.participants || [])
        };
        this.sessions.set(session.id, session);
        this.chartSessions.set(session.chartId, session.id);
      }
    }
  }

  private startCleanupInterval(): void {
    // Cleanup inactive sessions every hour
    setInterval(async () => {
      await this.cleanupInactiveSessions();
    }, 3600000);
  }

  private async cleanupInactiveSessions(): Promise<void> {
    const now = Date.now();
    const inactiveThreshold = 86400000; // 24 hours

    for (const [sessionId, session] of this.sessions) {
      if (now - session.lastActivity > inactiveThreshold && session.participants.size === 0) {
        this.sessions.delete(sessionId);
        this.chartSessions.delete(session.chartId);
        await this.state.storage.delete(`session:${sessionId}`);
      }
    }
  }
}
```

## 🔧 Workers Integration

### Main Worker with Durable Objects

```typescript
// src/index.ts
import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';

type App = Hono<{
  Bindings: Env;
  Variables: {
    user: User;
    session: SessionData;
  };
}>;

const app = new App();

// Middleware
app.use('*', cors());
app.use('*', logger());
app.use('*', authMiddleware());

// Durable Objects bindings
app.use('*', async (c, next) => {
  // Attach DO instances to context
  c.set('sessionManager', c.env.SESSION_MANAGER);
  c.set('chartCache', c.env.CHART_CACHE);
  c.set('rateLimiter', c.env.RATE_LIMITER);
  c.set('realtimeCollab', c.env.REALTIME_COLLAB);
  await next();
});

// WebSocket upgrade for real-time collaboration
app.upgradeWebSocket('/ws/collaborate/:chartId', async (c) => {
  const chartId = c.req.param('chartId');
  const user = c.get('user');

  if (!user) {
    return new Response('Unauthorized', { status: 401 });
  }

  const pair = new WebSocketPair();
  const [client, server] = pair;

  await c.env.REALTIME_COLLAB.joinSession(
    chartId,
    user.id,
    user.fullName || 'Anonymous',
    server,
    'editor'
  );

  return new Response(null, {
    status: 101,
    webSocket: client
  });
});

// API Routes using Durable Objects
app.post('/api/v2/charts/calculate', async (c) => {
  const user = c.get('user');
  const body = await c.req.json();

  // Check rate limit
  const rateLimit = await c.env.RATE_LIMITER.checkLimit(
    user?.id || c.req.header('CF-Connecting-IP'),
    'chart_calculation',
    user?.subscriptionStatus
  );

  if (!rateLimit.allowed) {
    return c.json({
      error: 'Rate limit exceeded',
      retryAfter: rateLimit.retryAfter,
      planType: rateLimit.planType
    }, 429);
  }

  // Check cache first
  const chartKey = generateChartKey(body);
  const cachedChart = await c.env.CHART_CACHE.getChart(chartKey);

  if (cachedChart) {
    return c.json({
      success: true,
      data: cachedChart.calculatedData,
      cached: true,
      calculationTime: cachedChart.calculationTime
    });
  }

  // Calculate new chart
  const startTime = Date.now();
  const chartData = await calculateChart(body);
  const calculationTime = Date.now() - startTime;

  // Cache the result
  await c.env.CHART_CACHE.cacheChart(chartKey, {
    id: chartKey,
    chartType: body.chartType,
    birthData: body,
    calculatedData: chartData,
    calculationTime
  });

  // Update user session
  if (user) {
    await c.env.SESSION_MANAGER.updateSession(user.id, {
      activeCharts: [...(c.get('session')?.activeCharts || []), chartKey]
    });
  }

  return c.json({
    success: true,
    data: chartData,
    cached: false,
    calculationTime
  });
});

export default app;
```

## 📊 Performance Monitoring

### Durable Objects Metrics

```typescript
// src/monitoring/do-metrics.ts
export class DurableObjectsMonitor {
  async getDurableObjectStats(env: Env): Promise<DOStats> {
    const stats = {
      sessionManager: await this.getSessionManagerStats(env.SESSION_MANAGER),
      chartCache: await this.getChartCacheStats(env.CHART_CACHE),
      rateLimiter: await this.getRateLimiterStats(env.RATE_LIMITER),
      realtimeCollab: await this.getRealtimeStats(env.REALTIME_COLLAB)
    };

    return stats;
  }

  private async getSessionManagerStats(sessionManager: SessionManagerDurableObject): Promise<SessionStats> {
    // Get session manager statistics
    return {
      activeSessions: 0, // Would need to be tracked in the DO
      totalConnections: 0,
      averageSessionDuration: 0,
      peakConcurrentSessions: 0
    };
  }

  private async getChartCacheStats(chartCache: ChartCacheDurableObject): Promise<CacheStats> {
    return await chartCache.getStats();
  }

  private async getRateLimiterStats(rateLimiter: RateLimiterDurableObject): Promise<RateLimitStats> {
    const analytics = await rateLimiter.getAnalytics('24h');

    return {
      totalRequests: analytics.totalRequests,
      blockedRequests: analytics.blockedRequests,
      blockRate: analytics.blockRate,
      topEndpoints: analytics.topEndpoints.slice(0, 5)
    };
  }
}

interface DOStats {
  sessionManager: SessionStats;
  chartCache: CacheStats;
  rateLimiter: RateLimitStats;
  realtimeCollab: RealtimeStats;
}
```

## 🎯 Best Practices

### Performance Optimization

1. **Efficient Data Structures**: Use Maps and Sets for O(1) lookups
2. **Batch Operations**: Group multiple operations for efficiency
3. **Memory Management**: Regular cleanup of expired data
4. **Connection Pooling**: Reuse WebSocket connections efficiently
5. **Async Operations**: Use non-blocking operations throughout

### Security Considerations

1. **Input Validation**: Validate all incoming WebSocket messages
2. **Rate Limiting**: Implement per-user and per-IP limits
3. **Access Control**: Validate permissions for all operations
4. **Data Sanitization**: Sanitize data before storage
5. **Connection Security**: Use secure WebSocket connections

### Reliability Patterns

1. **Graceful Degradation**: Continue operating when individual DOs fail
2. **Automatic Recovery**: Implement self-healing mechanisms
3. **State Persistence**: Regularly persist critical state
4. **Health Monitoring**: Implement comprehensive health checks
5. **Error Handling**: Robust error handling and logging

---

## 📚 Related Documentation

- [Migration Plan](./MIGRATION_PLAN.md) - Complete migration strategy
- [Cloudflare Architecture](./CLOUDFLARE_ARCHITECTURE.md) - Technical architecture overview
- [D1 Database Schema](./D1_DATABASE_SCHEMA.md) - Database design and migrations
- [Authentication Migration](./AUTHENTICATION_MIGRATION.md) - OAuth implementation
- [Testing Strategy](./TESTING_STRATEGY.md) - Durable Objects testing approach

This Durable Objects design provides stateful capabilities that complement D1's data storage, enabling real-time features, intelligent caching, and sophisticated session management for FortuneT's cloud-native architecture.