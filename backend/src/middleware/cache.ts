import { MiddlewareHandler, Context } from 'hono';

export interface CacheOptions {
  /** Max age in seconds (0 = no caching) */
  maxAge: number;
  /** Allow shared caches (CDNs) to cache? Default: false for user-specific data */
  shared?: boolean;
  /** Must revalidate before using stale cache */
  mustRevalidate?: boolean;
}

/**
 * Set HTTP caching headers on response
 * Uses 'private' by default to prevent CDN from caching user-specific data
 */
export function setCacheHeaders(options: CacheOptions): MiddlewareHandler {
  const { maxAge, shared = false, mustRevalidate = false } = options;

  return async (c, next) => {
    await next();

    if (maxAge <= 0) {
      c.res.headers.set('Cache-Control', 'no-store, no-cache, must-revalidate');
      return;
    }

    const directives: string[] = [];
    directives.push(shared ? 'public' : 'private');
    directives.push(`max-age=${maxAge}`);
    if (mustRevalidate) directives.push('must-revalidate');

    c.res.headers.set('Cache-Control', directives.join(', '));
    // Vary on Authorization so caches don't serve user A's data to user B
    c.res.headers.set('Vary', 'Authorization');
  };
}

/**
 * Helper to create ETag from data hash and timestamp
 * Returns a weak ETag since charts can be recalculated
 */
export function createETag(hash: string, timestamp?: string | number): string {
  const tag = timestamp ? `${hash}-${timestamp}` : hash;
  return `"${tag}"`;
}
