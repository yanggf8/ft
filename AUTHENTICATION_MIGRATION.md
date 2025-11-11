# 🔐 Authentication Migration to Cloudflare Native

## 📋 Overview

Complete migration guide for replacing Current Database authentication with Cloudflare-native solutions, including both Cloudflare Access integration and custom OAuth implementation options.

## 🎯 Migration Goals

- **Zero Current Database Dependency**: Complete authentication independence
- **Enterprise Security**: Cloudflare Access Zero Trust model
- **Cost Reduction**: Eliminate $25-75/month Current Database Auth costs
- **Better Performance**: Edge authentication with sub-50ms latency
- **User Experience Preservation**: Seamless migration for existing users

## 🏗️ Authentication Architecture Options

### Option A: Cloudflare Access (Recommended)

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│   User          │───►│  Cloudflare      │───►│   Google OAuth      │
│ (Browser)       │    │  Access Login    │    │   Provider         │
└─────────────────┘    └──────────────────┘    └─────────────────────┘
                           ▲                         │
                           │    Authorization Code   │
                           │◄────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Cloudflare Workers (Auth Service)              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   OAuth     │  │   Token     │  │     User Profile        │  │
│  │   Handler   │  │   Manager   │  │     Service             │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     D1 Database + Durable Objects               │
│  • User Profiles  • Sessions  • OAuth Tokens  • Preferences     │
└─────────────────────────────────────────────────────────────────┘
```

**Benefits:**
- Enterprise-grade Zero Trust security
- Built-in multi-provider SSO (Google, GitHub, Azure AD)
- No OAuth client management required
- Advanced security features (device posture, location-based access)
- Comprehensive audit logging

### Option B: Custom OAuth Implementation

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────────┐
│   User          │───►│  Custom OAuth    │───►│   Google OAuth      │
│ (Browser)       │    │  Implementation  │    │   Provider         │
└─────────────────┘    └──────────────────┘    └─────────────────────┘
                           ▲                         │
                           │    Authorization Code   │
                           │◄────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                  Cloudflare Workers (Auth Service)              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │   OAuth     │  │   Token     │  │     User Profile        │  │
│  │   Flow      │  │   Manager   │  │     Service             │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│                     D1 Database + Durable Objects               │
│  • User Profiles  • Sessions  • OAuth Tokens  • Preferences     │
└─────────────────────────────────────────────────────────────────┘
```

**Benefits:**
- Full control over authentication flow
- Portable to other providers
- Custom user experience
- No vendor lock-in

## 🔧 Option A: Cloudflare Access Implementation

### 1. Cloudflare Access Setup

#### Dashboard Configuration

```bash
# 1. Login to Cloudflare Dashboard
# 2. Navigate to Access > Applications
# 3. Create new application: "FortuneT V2"
# 4. Configure settings:
#    - Domain: v2.ahexagram.com
#    - Session duration: 24 hours
#    - Enable device posture (optional)
#    - Configure logout redirect
```

#### Identity Provider Configuration

```typescript
// cloudflare-access-config.ts
export const AccessConfig = {
  application: {
    name: "FortuneT V2",
    domain: "v2.ahexagram.com",
    type: "SelfHosted",
    sessionDuration: "24h"
  },

  identityProviders: [
    {
      type: "google",
      name: "Google",
      config: {
        client_id: process.env.GOOGLE_CLIENT_ID,
        client_secret: process.env.GOOGLE_CLIENT_SECRET
      }
    },
    {
      type: "github",
      name: "GitHub",
      config: {
        client_id: process.env.GITHUB_CLIENT_ID,
        client_secret: process.env.GITHUB_CLIENT_SECRET
      }
    }
  ],

  policies: [
    {
      name: "Allow All Users",
      decision: "allow",
      include: [
        {
          email: {
            domain: "*"
          }
        }
      ]
    }
  ],

  cors: {
    allowedOrigins: [
      "https://v2.ahexagram.com",
      "https://ziwei.ahexagram.com",
      "https://zodiac.ahexagram.com"
    ]
  }
};
```

### 2. Workers Authentication Service

```typescript
// src/services/cloudflare-access-auth.ts
export interface CloudflareAccessUser {
  sub: string;              // UUID from Cloudflare Access
  email: string;
  name: string;
  picture?: string;
  email_verified: boolean;
  aud: string;              // Application audience
  iss: string;              // Cloudflare Access issuer
  exp: number;
  iat: number;
  custom_attributes?: Record<string, any>;
}

export interface UserProfile {
  id: string;
  cloudflareId: string;     // Cloudflare Access sub
  email: string;
  fullName: string;
  avatar?: string;
  emailVerified: boolean;
  provider: 'google' | 'github' | 'email';
  preferences: UserPreferences;
  subscriptionStatus: 'free' | 'premium' | 'professional';
  stripeCustomerId?: string;
  createdAt: Date;
  updatedAt: Date;
}

export class CloudflareAccessAuth {
  private jwtSecret: string;
  private authDomain: string;
  private appId: string;

  constructor(private env: Env) {
    this.jwtSecret = env.JWT_SECRET;
    this.authDomain = env.CLOUDFLARE_ACCESS_DOMAIN;
    this.appId = env.CLOUDFLARE_ACCESS_APP_ID;
  }

  // Verify Cloudflare Access JWT token
  async verifyAccessToken(token: string): Promise<CloudflareAccessUser | null> {
    try {
      // Fetch Cloudflare Access public keys
      const jwksResponse = await fetch(
        `https://${this.authDomain}/cdn-cgi/access/certs`,
        {
          headers: {
            'User-Agent': 'FortuneT-V2/1.0'
          }
        }
      );

      if (!jwksResponse.ok) {
        throw new Error('Failed to fetch JWKS');
      }

      const jwks = await jwksResponse.json();

      // Verify JWT with Cloudflare Access public keys
      const decoded = await this.verifyJWT(token, jwks);

      if (!decoded || decoded.iss !== `https://${this.authDomain}`) {
        return null;
      }

      // Validate audience
      if (decoded.aud !== this.appId) {
        return null;
      }

      return decoded as CloudflareAccessUser;

    } catch (error) {
      console.error('Access token verification failed:', error);
      return null;
    }
  }

  // Create or update user profile from Cloudflare Access data
  async handleUserLogin(accessUser: CloudflareAccessUser): Promise<UserProfile> {
    // Check if user exists by cloudflare ID
    let user = await this.getUserByCloudflareId(accessUser.sub);

    if (!user) {
      // Check if user exists by email (migration from other providers)
      user = await this.getUserByEmail(accessUser.email);

      if (user) {
        // Update existing user with Cloudflare ID
        user = await this.updateUserCloudflareId(user.id, accessUser.sub);
      } else {
        // Create new user
        user = await this.createUserFromAccess(accessUser);
      }
    } else {
      // Update existing user data
      user = await this.updateUserProfile(user.id, accessUser);
    }

    return user;
  }

  // Create application session token
  async createSessionToken(user: UserProfile): Promise<string> {
    const now = Math.floor(Date.now() / 1000);

    const payload = {
      sub: user.id,
      email: user.email,
      name: user.fullName,
      role: user.subscriptionStatus,
      permissions: this.getUserPermissions(user.subscriptionStatus),
      iat: now,
      exp: now + (7 * 24 * 60 * 60), // 7 days
      aud: this.appId,
      iss: `https://${this.authDomain}`
    };

    return await this.signJWT(payload);
  }

  // Refresh session and update user data
  async refreshSession(userId: string): Promise<UserProfile | null> {
    const user = await this.getUserById(userId);
    if (!user) return null;

    // Update last activity
    await this.env.DB.prepare(`
      UPDATE user_profiles SET updated_at = CURRENT_TIMESTAMP
      WHERE user_id = ?
    `).bind(userId).run();

    return user;
  }

  // Private helper methods
  private async createUserFromAccess(accessUser: CloudflareAccessUser): Promise<UserProfile> {
    const userId = crypto.randomUUID();

    const profile: UserProfile = {
      id: userId,
      cloudflareId: accessUser.sub,
      email: accessUser.email,
      fullName: accessUser.name,
      avatar: accessUser.picture,
      emailVerified: accessUser.email_verified,
      provider: this.determineProvider(accessUser),
      preferences: this.getDefaultPreferences(),
      subscriptionStatus: 'free',
      createdAt: new Date(),
      updatedAt: new Date()
    };

    // Insert into D1 database
    await this.env.DB.prepare(`
      INSERT INTO users (
        id, cloudflare_id, email, full_name, avatar_url,
        email_verified, provider, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).bind(
      profile.id,
      profile.cloudflareId,
      profile.email,
      profile.fullName,
      profile.avatar,
      profile.emailVerified,
      profile.provider,
      profile.createdAt.toISOString(),
      profile.updatedAt.toISOString()
    ).run();

    // Create user profile entry
    await this.env.DB.prepare(`
      INSERT INTO user_profiles (
        id, user_id, preferences, subscription_status, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?)
    `).bind(
      crypto.randomUUID(),
      profile.id,
      JSON.stringify(profile.preferences),
      profile.subscriptionStatus,
      profile.createdAt.toISOString(),
      profile.updatedAt.toISOString()
    ).run();

    return profile;
  }

  private async getUserByCloudflareId(cloudflareId: string): Promise<UserProfile | null> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status, up.stripe_customer_id
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.cloudflare_id = ?
    `).bind(cloudflareId).first();

    return result ? this.mapDbRowToProfile(result) : null;
  }

  private async getUserByEmail(email: string): Promise<UserProfile | null> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status, up.stripe_customer_id
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.email = ?
    `).bind(email).first();

    return result ? this.mapDbRowToProfile(result) : null;
  }

  private async getUserById(userId: string): Promise<UserProfile | null> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status, up.stripe_customer_id
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.id = ?
    `).bind(userId).first();

    return result ? this.mapDbRowToProfile(result) : null;
  }

  private mapDbRowToProfile(row: any): UserProfile {
    return {
      id: row.id,
      cloudflareId: row.cloudflare_id,
      email: row.email,
      fullName: row.full_name,
      avatar: row.avatar_url,
      emailVerified: row.email_verified,
      provider: row.provider,
      preferences: JSON.parse(row.preferences || '{}'),
      subscriptionStatus: row.subscription_status || 'free',
      stripeCustomerId: row.stripe_customer_id,
      createdAt: new Date(row.created_at),
      updatedAt: new Date(row.updated_at)
    };
  }

  private determineProvider(accessUser: CloudflareAccessUser): 'google' | 'github' | 'email' {
    // Determine provider based on custom attributes or email domain
    if (accessUser.email?.includes('@gmail.com')) {
      return 'google';
    } else if (accessUser.email?.includes('@users.noreply.github.com')) {
      return 'github';
    }
    return 'email';
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

  // JWT implementation using Web Crypto API
  private async signJWT(payload: any): Promise<string> {
    const header = { alg: 'HS256', typ: 'JWT' };

    const encoder = new TextEncoder();
    const keyData = encoder.encode(this.jwtSecret);
    const key = await crypto.subtle.importKey(
      'raw',
      keyData,
      { name: 'HMAC', hash: 'SHA-256' },
      false,
      ['sign']
    );

    const encodedHeader = this.base64UrlEncode(
      encoder.encode(JSON.stringify(header))
    );
    const encodedPayload = this.base64UrlEncode(
      encoder.encode(JSON.stringify(payload))
    );

    const message = encoder.encode(`${encodedHeader}.${encodedPayload}`);
    const signature = await crypto.subtle.sign('HMAC', key, message);
    const encodedSignature = this.base64UrlEncode(new Uint8Array(signature));

    return `${encodedHeader}.${encodedPayload}.${encodedSignature}`;
  }

  private async verifyJWT(token: string, jwks: any): Promise<any> {
    const parts = token.split('.');
    if (parts.length !== 3) {
      throw new Error('Invalid JWT format');
    }

    const [encodedHeader, encodedPayload, encodedSignature] = parts;

    // Find matching key in JWKS
    const header = JSON.parse(this.base64UrlDecode(encodedHeader));
    const key = jwks.keys.find((k: any) => k.kid === header.kid);

    if (!key) {
      throw new Error('Key not found in JWKS');
    }

    // Import the public key
    const publicKey = await this.importPublicKey(key);

    // Verify signature
    const message = this.base64UrlDecode(encodedHeader) + '.' + this.base64UrlDecode(encodedPayload);
    const signature = this.base64UrlDecode(encodedSignature);

    const isValid = await crypto.subtle.verify(
      'RS256',
      publicKey,
      this.base64UrlDecodeToUint8Array(signature),
      new TextEncoder().encode(message)
    );

    if (!isValid) {
      throw new Error('Invalid JWT signature');
    }

    // Decode payload
    const payload = JSON.parse(this.base64UrlDecode(encodedPayload));

    return payload;
  }

  private async importPublicKey(key: any): Promise<CryptoKey> {
    const publicKeyJwk = {
      kty: key.kty,
      n: key.n,
      e: key.e,
      alg: key.alg,
      kid: key.kid,
      use: key.use
    };

    return await crypto.subtle.importKey(
      'jwk',
      publicKeyJwk,
      {
        name: 'RSASSA-PKCS1-v1_5',
        hash: 'SHA-256'
      },
      false,
      ['verify']
    );
  }

  private base64UrlEncode(data: ArrayBuffer | Uint8Array): string {
    const base64 = btoa(String.fromCharCode(...new Uint8Array(data)));
    return base64.replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
  }

  private base64UrlDecode(base64: string): string {
    base64 = base64.replace(/-/g, '+').replace(/_/g, '/');

    while (base64.length % 4) {
      base64 += '=';
    }

    return atob(base64);
  }

  private base64UrlDecodeToUint8Array(base64: string): Uint8Array {
    const binaryString = this.base64UrlDecode(base64);
    return Uint8Array.from(binaryString, c => c.charCodeAt(0));
  }
}
```

### 3. Authentication Routes

```typescript
// src/routes/auth.ts
import { Hono } from 'hono';
import { setCookie, deleteCookie } from 'hono/cookie';
import { CloudflareAccessAuth } from '../services/cloudflare-access-auth';

const auth = new Hono<{ Bindings: Env }>();

// Initiate Cloudflare Access login
auth.get('/login', async (c) => {
  const redirectTo = c.req.query('redirect') || '/';
  const appId = c.env.CLOUDFLARE_ACCESS_APP_ID;

  // Redirect to Cloudflare Access login
  const loginUrl = `https://${c.env.CLOUDFLARE_ACCESS_DOMAIN}/cdn-cgi/access/login/${appId}?redirect=${encodeURIComponent(redirectTo)}`;

  return c.redirect(loginUrl);
});

// Handle Cloudflare Access callback
auth.get('/callback', async (c) => {
  try {
    // Extract Cloudflare Access token from headers
    const cfAccessJwt = c.req.header('Cf-Access-Jwt-Assertion');

    if (!cfAccessJwt) {
      return c.redirect('/login?error=no_token');
    }

    const authService = new CloudflareAccessAuth(c.env);

    // Verify Cloudflare Access token
    const accessUser = await authService.verifyAccessToken(cfAccessJwt);
    if (!accessUser) {
      return c.redirect('/login?error=invalid_token');
    }

    // Create or update user profile
    const userProfile = await authService.handleUserLogin(accessUser);

    // Create application session token
    const sessionToken = await authService.createSessionToken(userProfile);

    // Set session cookie
    setCookie(c, 'session_token', sessionToken, {
      httpOnly: true,
      secure: true,
      sameSite: 'Lax',
      maxAge: 7 * 24 * 60 * 60, // 7 days
      path: '/'
    });

    // Store session in Durable Object
    await c.env.SESSION_MANAGER.createSession(userProfile.id, {
      userId: userProfile.id,
      email: userProfile.email,
      subscriptionStatus: userProfile.subscriptionStatus,
      preferences: userProfile.preferences,
      activeCharts: [],
      lastActivity: Date.now()
    });

    // Redirect to intended destination
    const redirectTo = c.req.query('redirect') || '/dashboard';
    return c.redirect(redirectTo);

  } catch (error) {
    console.error('Auth callback error:', error);
    return c.redirect('/login?error=callback_failed');
  }
});

// Logout
auth.post('/logout', async (c) => {
  const sessionToken = getCookie(c, 'session_token');

  if (sessionToken) {
    try {
      const authService = new CloudflareAccessAuth(c.env);
      const payload = await authService.verifySessionToken(sessionToken);

      if (payload) {
        // Invalidate session in Durable Object
        await c.env.SESSION_MANAGER.invalidateSession(payload.sub);
      }
    } catch (error) {
      console.error('Logout error:', error);
    }
  }

  // Clear session cookie
  deleteCookie(c, 'session_token', {
    httpOnly: true,
    secure: true,
    sameSite: 'Lax',
    path: '/'
  });

  // Redirect to Cloudflare Access logout
  const logoutUrl = `https://${c.env.CLOUDFLARE_ACCESS_DOMAIN}/cdn-cgi/access/logout`;
  return c.redirect(logoutUrl);
});

// Get current user profile
auth.get('/me', authMiddleware, async (c) => {
  const user = c.get('user');

  if (!user) {
    return c.json({ error: 'Not authenticated' }, 401);
  }

  return c.json({
    success: true,
    data: {
      id: user.id,
      email: user.email,
      fullName: user.fullName,
      avatar: user.avatar,
      subscriptionStatus: user.subscriptionStatus,
      preferences: user.preferences
    }
  });
});

export { auth };
```

## 🔧 Option B: Custom OAuth Implementation

### 1. Google OAuth Setup

#### Google Cloud Console

```bash
# 1. Go to Google Cloud Console
# 2. Create new project or select existing
# 3. Enable Google+ API and Google OAuth2 API
# 4. Create OAuth 2.0 Credentials
# 5. Configure authorized redirect URIs:
#    - https://v2.ahexagram.com/auth/google/callback
#    - https://localhost:3000/auth/google/callback (development)
```

### 2. Custom OAuth Service

```typescript
// src/services/custom-google-oauth.ts
export interface GoogleUser {
  id: string;
  email: string;
  name: string;
  picture?: string;
  verified_email: boolean;
  locale?: string;
  hd?: string; // Hosted domain
}

export interface TokenResponse {
  access_token: string;
  refresh_token?: string;
  token_type: string;
  expires_in: number;
  scope: string;
  id_token?: string;
}

export class CustomGoogleOAuth {
  private clientId: string;
  private clientSecret: string;
  private redirectUri: string;
  private scopes: string[];

  constructor(private env: Env) {
    this.clientId = env.GOOGLE_OAUTH_CLIENT_ID;
    this.clientSecret = env.GOOGLE_OAUTH_CLIENT_SECRET;
    this.redirectUri = `${env.APP_URL}/auth/google/callback`;
    this.scopes = [
      'openid',
      'email',
      'profile'
    ];
  }

  // Get Google OAuth authorization URL
  getAuthUrl(state?: string, prompt: 'consent' | 'none' = 'consent'): string {
    const params = new URLSearchParams({
      client_id: this.clientId,
      redirect_uri: this.redirectUri,
      response_type: 'code',
      scope: this.scopes.join(' '),
      access_type: 'offline',
      prompt,
      state: state || crypto.randomUUID()
    });

    return `https://accounts.google.com/o/oauth2/v2/auth?${params.toString()}`;
  }

  // Exchange authorization code for tokens
  async exchangeCodeForTokens(code: string): Promise<TokenResponse> {
    const response = await fetch('https://oauth2.googleapis.com/token', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json'
      },
      body: new URLSearchParams({
        client_id: this.clientId,
        client_secret: this.clientSecret,
        code,
        grant_type: 'authorization_code',
        redirect_uri: this.redirectUri
      })
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to exchange code for tokens: ${error}`);
    }

    return await response.json();
  }

  // Get user info from Google API
  async getUserInfo(accessToken: string): Promise<GoogleUser> {
    const response = await fetch(
      'https://www.googleapis.com/oauth2/v2/userinfo',
      {
        headers: {
          'Authorization': `Bearer ${accessToken}`,
          'Accept': 'application/json'
        }
      }
    );

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to fetch user info: ${error}`);
    }

    return await response.json();
  }

  // Verify ID token (if available)
  async verifyIdToken(idToken: string): Promise<any> {
    // For production, you should use Google's token verification libraries
    // or fetch Google's public keys and verify locally

    try {
      const [header, payload, signature] = idToken.split('.');
      const decodedPayload = JSON.parse(atob(payload));

      // Basic validation
      if (decodedPayload.iss !== 'https://accounts.google.com' &&
          decodedPayload.iss !== 'accounts.google.com') {
        throw new Error('Invalid issuer');
      }

      if (decodedPayload.aud !== this.clientId) {
        throw new Error('Invalid audience');
      }

      if (decodedPayload.exp && Date.now() >= decodedPayload.exp * 1000) {
        throw new Error('Token expired');
      }

      return decodedPayload;

    } catch (error) {
      throw new Error(`Invalid ID token: ${error.message}`);
    }
  }

  // Refresh access token
  async refreshAccessToken(refreshToken: string): Promise<TokenResponse> {
    const response = await fetch('https://oauth2.googleapis.com/token', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json'
      },
      body: new URLSearchParams({
        client_id: this.clientId,
        client_secret: this.clientSecret,
        refresh_token: refreshToken,
        grant_type: 'refresh_token'
      })
    });

    if (!response.ok) {
      const error = await response.text();
      throw new Error(`Failed to refresh token: ${error}`);
    }

    return await response.json();
  }

  // Handle complete OAuth flow
  async handleOAuthCallback(code: string): Promise<{
    user: GoogleUser;
    tokens: TokenResponse;
    userProfile: UserProfile;
  }> {
    try {
      // Exchange code for tokens
      const tokens = await this.exchangeCodeForTokens(code);

      // Get user info
      const googleUser = await this.getUserInfo(tokens.access_token);

      // Verify ID token if available
      let idTokenPayload = null;
      if (tokens.id_token) {
        idTokenPayload = await this.verifyIdToken(tokens.id_token);
      }

      // Create or update user profile
      const userProfile = await this.createOrUpdateGoogleUser(googleUser, tokens);

      return {
        user: googleUser,
        tokens,
        userProfile
      };

    } catch (error) {
      console.error('OAuth callback error:', error);
      throw new Error(`OAuth flow failed: ${error.message}`);
    }
  }

  private async createOrUpdateGoogleUser(
    googleUser: GoogleUser,
    tokens: TokenResponse
  ): Promise<UserProfile> {
    // Check if user exists by Google ID
    let user = await this.getUserByGoogleId(googleUser.id);

    if (!user) {
      // Check if user exists by email (migration from other providers)
      user = await this.getUserByEmail(googleUser.email);

      if (user) {
        // Update existing user with Google ID
        user = await this.updateUserGoogleId(user.id, googleUser.id);
      } else {
        // Create new user
        user = await this.createUserFromGoogle(googleUser);
      }
    } else {
      // Update existing user data
      user = await this.updateGoogleUser(user.id, googleUser);
    }

    // Store OAuth tokens securely
    await this.storeOAuthTokens(user.id, tokens, 'google');

    return user;
  }

  private async createUserFromGoogle(googleUser: GoogleUser): Promise<UserProfile> {
    const userId = crypto.randomUUID();

    const profile: UserProfile = {
      id: userId,
      cloudflareId: null, // Not using Cloudflare Access
      googleId: googleUser.id,
      email: googleUser.email,
      fullName: googleUser.name,
      avatar: googleUser.picture,
      emailVerified: googleUser.verified_email,
      provider: 'google',
      preferences: this.getDefaultPreferences(),
      subscriptionStatus: 'free',
      createdAt: new Date(),
      updatedAt: new Date()
    };

    // Insert into D1 database
    await this.env.DB.prepare(`
      INSERT INTO users (
        id, google_id, email, full_name, avatar_url,
        email_verified, provider, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).bind(
      profile.id,
      profile.googleId,
      profile.email,
      profile.fullName,
      profile.avatar,
      profile.emailVerified,
      profile.provider,
      profile.createdAt.toISOString(),
      profile.updatedAt.toISOString()
    ).run();

    // Create user profile entry
    await this.env.DB.prepare(`
      INSERT INTO user_profiles (
        id, user_id, preferences, subscription_status, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?)
    `).bind(
      crypto.randomUUID(),
      profile.id,
      JSON.stringify(profile.preferences),
      profile.subscriptionStatus,
      profile.createdAt.toISOString(),
      profile.updatedAt.toISOString()
    ).run();

    return profile;
  }

  private async storeOAuthTokens(
    userId: string,
    tokens: TokenResponse,
    provider: string
  ): Promise<void> {
    // Encrypt and store tokens in D1 or secure storage
    const expiresAt = new Date(Date.now() + (tokens.expires_in * 1000));

    await this.env.DB.prepare(`
      INSERT OR REPLACE INTO oauth_tokens (
        user_id, provider, access_token, refresh_token, token_type,
        scope, expires_at, created_at, updated_at
      ) VALUES (?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
    `).bind(
      userId,
      provider,
      tokens.access_token,
      tokens.refresh_token || null,
      tokens.token_type,
      tokens.scope,
      expiresAt.toISOString()
    ).run();
  }

  private async getUserByGoogleId(googleId: string): Promise<UserProfile | null> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status, up.stripe_customer_id
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.google_id = ?
    `).bind(googleId).first();

    return result ? this.mapDbRowToProfile(result) : null;
  }

  private async getUserByEmail(email: string): Promise<UserProfile | null> {
    const result = await this.env.DB.prepare(`
      SELECT u.*, up.preferences, up.subscription_status, up.stripe_customer_id
      FROM users u
      LEFT JOIN user_profiles up ON u.id = up.user_id
      WHERE u.email = ?
    `).bind(email).first();

    return result ? this.mapDbRowToProfile(result) : null;
  }

  private mapDbRowToProfile(row: any): UserProfile {
    return {
      id: row.id,
      cloudflareId: row.cloudflare_id,
      googleId: row.google_id,
      email: row.email,
      fullName: row.full_name,
      avatar: row.avatar_url,
      emailVerified: row.email_verified,
      provider: row.provider,
      preferences: JSON.parse(row.preferences || '{}'),
      subscriptionStatus: row.subscription_status || 'free',
      stripeCustomerId: row.stripe_customer_id,
      createdAt: new Date(row.created_at),
      updatedAt: new Date(row.updated_at)
    };
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
}
```

### 3. Custom OAuth Routes

```typescript
// src/routes/oauth.ts
import { Hono } from 'hono';
import { setCookie, deleteCookie } from 'hono/cookie';
import { CustomGoogleOAuth } from '../services/custom-google-oauth';

const oauth = new Hono<{ Bindings: Env }>();

// Initiate Google OAuth login
oauth.get('/google/login', async (c) => {
  const state = crypto.randomUUID();
  const redirectTo = c.req.query('redirect') || '/';

  // Store state in session or KV for verification
  await c.env.KV.put(`oauth_state:${state}`, JSON.stringify({
    redirectTo,
    timestamp: Date.now()
  }), { expirationTtl: 600 }); // 10 minutes

  const oauthService = new CustomGoogleOAuth(c.env);
  const authUrl = oauthService.getAuthUrl(state);

  return c.redirect(authUrl);
});

// Handle Google OAuth callback
oauth.get('/google/callback', async (c) => {
  try {
    const code = c.req.query('code');
    const state = c.req.query('state');
    const error = c.req.query('error');

    if (error) {
      return c.redirect('/login?error=oauth_error&message=' + encodeURIComponent(error));
    }

    if (!code || !state) {
      return c.redirect('/login?error=missing_parameters');
    }

    // Verify state
    const stateData = await c.env.KV.get(`oauth_state:${state}`);
    if (!stateData) {
      return c.redirect('/login?error=invalid_state');
    }

    const { redirectTo } = JSON.parse(stateData);
    await c.env.KV.delete(`oauth_state:${state}`);

    const oauthService = new CustomGoogleOAuth(c.env);
    const result = await oauthService.handleOAuthCallback(code);

    // Create application session token
    const sessionToken = await createSessionToken(result.userProfile);

    // Set session cookie
    setCookie(c, 'session_token', sessionToken, {
      httpOnly: true,
      secure: true,
      sameSite: 'Lax',
      maxAge: 7 * 24 * 60 * 60, // 7 days
      path: '/'
    });

    // Store session in Durable Object
    await c.env.SESSION_MANAGER.createSession(result.userProfile.id, {
      userId: result.userProfile.id,
      email: result.userProfile.email,
      subscriptionStatus: result.userProfile.subscriptionStatus,
      preferences: result.userProfile.preferences,
      activeCharts: [],
      lastActivity: Date.now()
    });

    // Redirect to intended destination
    return c.redirect(redirectTo || '/dashboard');

  } catch (error) {
    console.error('OAuth callback error:', error);
    return c.redirect('/login?error=callback_failed&message=' + encodeURIComponent(error.message));
  }
});

export { oauth };
```

## 🔄 Migration Strategy

### Data Migration from Current Database

```typescript
// src/migration/auth-migration.ts
export class AuthenticationMigrationService {
  constructor(private env: Env) {}

  // Step 1: Export users from Current Database
  async exportCurrent DatabaseUsers(): Promise<Current DatabaseUser[]> {
    const supabase = createClient(
      this.env.SUPABASE_URL,
      this.env.SUPABASE_SERVICE_KEY
    );

    const { data: users, error } = await supabase
      .from('users')
      .select(`
        id,
        email,
        email_confirmed_at,
        full_name,
        avatar_url,
        created_at,
        updated_at,
        user_profiles (
          subscription_status,
          preferences,
          stripe_customer_id
        )
      `);

    if (error) throw error;
    return users;
  }

  // Step 2: Transform data for Cloudflare schema
  async transformUserData(supabaseUsers: Current DatabaseUser[]): Promise<UserProfile[]> {
    return supabaseUsers.map(user => ({
      id: user.id,
      cloudflareId: null, // Will be populated on next login
      googleId: null,     // Will be populated if using custom OAuth
      email: user.email,
      emailVerified: !!user.email_confirmed_at,
      fullName: user.full_name || '',
      avatar: user.avatar_url,
      provider: 'email',  // Default, will be updated on OAuth login
      preferences: user.user_profiles?.preferences ?
        JSON.parse(user.user_profiles.preferences) : {},
      subscriptionStatus: user.user_profiles?.subscription_status || 'free',
      stripeCustomerId: user.user_profiles?.stripe_customer_id,
      createdAt: new Date(user.created_at),
      updatedAt: new Date(user.updated_at)
    }));
  }

  // Step 3: Import users into D1
  async importUsersToD1(users: UserProfile[]): Promise<void> {
    const batch = this.env.DB.batch();

    for (const user of users) {
      // Insert user
      batch.push(
        this.env.DB.prepare(`
          INSERT OR REPLACE INTO users (
            id, email, email_verified, full_name, avatar_url,
            provider, created_at, updated_at
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        `).bind(
          user.id,
          user.email,
          user.emailVerified,
          user.fullName,
          user.avatar,
          user.provider,
          user.createdAt.toISOString(),
          user.updatedAt.toISOString()
        )
      );

      // Insert user profile
      batch.push(
        this.env.DB.prepare(`
          INSERT OR REPLACE INTO user_profiles (
            id, user_id, preferences, subscription_status,
            stripe_customer_id, created_at, updated_at
          ) VALUES (?, ?, ?, ?, ?, ?, ?)
        `).bind(
          crypto.randomUUID(),
          user.id,
          JSON.stringify(user.preferences),
          user.subscriptionStatus,
          user.stripeCustomerId,
          user.createdAt.toISOString(),
          user.updatedAt.toISOString()
        )
      );
    }

    await batch;
  }

  // Step 4: Handle account linking on next login
  async linkOAuthAccounts(userId: string, cloudflareId: string): Promise<void> {
    await this.env.DB.prepare(`
      UPDATE users SET cloudflare_id = ?, provider = 'cloudflare_access', updated_at = CURRENT_TIMESTAMP
      WHERE id = ?
    `).bind(cloudflareId, userId).run();
  }

  async linkGoogleAccount(userId: string, googleId: string): Promise<void> {
    await this.env.DB.prepare(`
      UPDATE users SET google_id = ?, provider = 'google', updated_at = CURRENT_TIMESTAMP
      WHERE id = ?
    `).bind(googleId, userId).run();
  }

  // Execute full migration
  async executeMigration(): Promise<{
    usersMigrated: number;
    success: boolean;
    errors?: string[];
  }> {
    try {
      // Export and transform users
      const supabaseUsers = await this.exportCurrent DatabaseUsers();
      const transformedUsers = await this.transformUserData(supabaseUsers);

      // Import users
      await this.importUsersToD1(transformedUsers);

      return {
        usersMigrated: transformedUsers.length,
        success: true
      };

    } catch (error) {
      console.error('Migration failed:', error);
      return {
        usersMigrated: 0,
        success: false,
        errors: [error.message]
      };
    }
  }
}
```

## 🛡️ Security Considerations

### JWT Security

```typescript
// src/services/jwt-security.ts
export class JWTSecurityService {
  private jwtSecret: string;
  private issuer: string;
  private audience: string;

  constructor(private env: Env) {
    this.jwtSecret = env.JWT_SECRET;
    this.issuer = `https://${env.APP_DOMAIN}`;
    this.audience = 'fortune-teller-app';
  }

  // Create secure session token
  async createSecureToken(user: UserProfile, expiresIn: number = 7 * 24 * 60 * 60): Promise<string> {
    const now = Math.floor(Date.now() / 1000);

    const payload = {
      sub: user.id,
      email: user.email,
      name: user.fullName,
      role: user.subscriptionStatus,
      permissions: this.getUserPermissions(user.subscriptionStatus),
      iat: now,
      exp: now + expiresIn,
      aud: this.audience,
      iss: this.issuer,
      jti: crypto.randomUUID() // JWT ID for revocation
    };

    return await this.signJWT(payload);
  }

  // Verify and validate token
  async verifySecureToken(token: string): Promise<DecodedToken | null> {
    try {
      const payload = await this.verifyJWT(token);

      // Additional security checks
      if (!this.validatePayload(payload)) {
        return null;
      }

      // Check if token is revoked
      if (await this.isTokenRevoked(payload.jti)) {
        return null;
      }

      return payload;

    } catch (error) {
      console.error('Token verification failed:', error);
      return null;
    }
  }

  // Revoke token (logout)
  async revokeToken(jti: string, exp: number): Promise<void> {
    await this.env.KV.put(`revoked_token:${jti}`, 'true', {
      expirationTtl: exp - Math.floor(Date.now() / 1000)
    });
  }

  private validatePayload(payload: any): boolean {
    // Validate required fields
    if (!payload.sub || !payload.email || !payload.exp) {
      return false;
    }

    // Validate expiration
    if (Date.now() / 1000 > payload.exp) {
      return false;
    }

    // Validate audience and issuer
    if (payload.aud !== this.audience || payload.iss !== this.issuer) {
      return false;
    }

    return true;
  }

  private async isTokenRevoked(jti: string): Promise<boolean> {
    const revoked = await this.env.KV.get(`revoked_token:${jti}`);
    return !!revoked;
  }
}

interface DecodedToken {
  sub: string;
  email: string;
  name: string;
  role: string;
  permissions: string[];
  iat: number;
  exp: number;
  aud: string;
  iss: string;
  jti: string;
}
```

### Rate Limiting for Auth

```typescript
// src/middleware/auth-rate-limit.ts
export const authRateLimitMiddleware = async (c: Context, next: Next) => {
  const ip = c.req.header('CF-Connecting-IP') || 'unknown';
  const endpoint = c.req.path;

  // Check rate limit
  const rateLimit = await c.env.RATE_LIMITER.checkLimit(ip, endpoint, 'free');

  if (!rateLimit.allowed) {
    return c.json({
      error: 'Too many authentication attempts',
      retryAfter: rateLimit.retryAfter
    }, 429);
  }

  await next();
};
```

## 🧪 Testing Authentication

### Authentication Test Suite

```typescript
// tests/auth.test.ts
describe('Authentication System', () => {
  let authService: CloudflareAccessAuth;
  let env: Env;

  beforeEach(() => {
    env = getMiniflareBindings();
    authService = new CloudflareAccessAuth(env);
  });

  describe('Cloudflare Access Integration', () => {
    it('should verify valid Access JWT tokens', async () => {
      const mockAccessUser = createMockAccessUser();
      const token = await createMockAccessJWT(mockAccessUser);

      const result = await authService.verifyAccessToken(token);

      expect(result).toBeTruthy();
      expect(result?.email).toBe(mockAccessUser.email);
    });

    it('should reject invalid tokens', async () => {
      const invalidToken = 'invalid.jwt.token';

      const result = await authService.verifyAccessToken(invalidToken);

      expect(result).toBeNull();
    });

    it('should create new user from Access data', async () => {
      const accessUser = createMockAccessUser();

      const userProfile = await authService.handleUserLogin(accessUser);

      expect(userProfile.email).toBe(accessUser.email);
      expect(userProfile.cloudflareId).toBe(accessUser.sub);
    });
  });

  describe('Session Management', () => {
    it('should create and verify session tokens', async () => {
      const user = createMockUserProfile();

      const token = await authService.createSessionToken(user);
      const verified = await authService.verifySessionToken(token);

      expect(verified).toBeTruthy();
      expect(verified?.sub).toBe(user.id);
    });

    it('should handle session expiration', async () => {
      const user = createMockUserProfile();

      // Create token with very short expiration
      const token = await authService.createSessionToken(user, 1); // 1 second
      await new Promise(resolve => setTimeout(resolve, 1100)); // Wait 1.1 seconds

      const verified = await authService.verifySessionToken(token);

      expect(verified).toBeNull();
    });
  });
});
```

## 📊 Migration Monitoring

### Authentication Analytics

```typescript
// src/services/auth-analytics.ts
export class AuthAnalyticsService {
  async trackLogin(user: UserProfile, provider: string, ip: string): Promise<void> {
    await this.env.DB.prepare(`
      INSERT INTO auth_logs (user_id, event_type, provider, ip_address, success, created_at)
      VALUES (?, 'login', ?, ?, true, CURRENT_TIMESTAMP)
    `).bind(user.id, provider, ip).run();
  }

  async trackFailedLogin(email: string, provider: string, ip: string, reason: string): Promise<void> {
    await this.env.DB.prepare(`
      INSERT INTO auth_logs (email, event_type, provider, ip_address, success, error_message, created_at)
      VALUES (?, 'failed_login', ?, ?, false, ?, CURRENT_TIMESTAMP)
    `).bind(email, provider, ip, reason).run();
  }

  async getAuthMetrics(timeRange: string = '24h'): Promise<AuthMetrics> {
    const now = Date.now();
    const startTime = this.getStartTimeRange(timeRange, now);

    const result = await this.env.DB.prepare(`
      SELECT
        event_type,
        provider,
        COUNT(*) as count,
        COUNT(DISTINCT user_id) as unique_users
      FROM auth_logs
      WHERE created_at >= datetime(?, 'unixepoch')
      GROUP BY event_type, provider
    `).bind(startTime / 1000).all();

    return this.formatAuthMetrics(result);
  }
}

interface AuthMetrics {
  timeRange: string;
  totalLogins: number;
  successfulLogins: number;
  failedLogins: number;
  loginsByProvider: Record<string, number>;
  uniqueUsers: number;
  generatedAt: string;
}
```

---

## 📚 Related Documentation

- [Migration Plan](./MIGRATION_PLAN.md) - Complete migration strategy
- [Cloudflare Architecture](./CLOUDFLARE_ARCHITECTURE.md) - Technical architecture overview
- [D1 Database Schema](./D1_DATABASE_SCHEMA.md) - Database design and migrations
- [Durable Objects Design](./DURABLE_OBJECTS_DESIGN.md) - Stateful caching patterns
- [Testing Strategy](./TESTING_STRATEGY.md) - Comprehensive testing approach

This authentication migration provides a secure, performant, and cost-effective solution that eliminates Current Database dependency while maintaining full feature parity and improving the overall user experience.