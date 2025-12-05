import { MiddlewareHandler } from 'hono';

export const securityHeaders: MiddlewareHandler = async (c, next) => {
  await next();
  
  c.header('X-Content-Type-Options', 'nosniff');
  c.header('X-Frame-Options', 'DENY');
  c.header('X-XSS-Protection', '1; mode=block');
  c.header('Referrer-Policy', 'strict-origin-when-cross-origin');
  c.header('Permissions-Policy', 'geolocation=(), microphone=(), camera=()');
  
  // CSP for API (no inline scripts needed)
  c.header('Content-Security-Policy', "default-src 'none'; frame-ancestors 'none'");
};
