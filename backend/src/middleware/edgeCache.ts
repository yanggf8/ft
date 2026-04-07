import { MiddlewareHandler } from 'hono';

/**
 * Cloudflare Cache API middleware for edge caching
 * Caches responses at Cloudflare's edge for faster subsequent requests
 *
 * Note: Cache API respects Cache-Control headers and won't cache responses with:
 * - Cache-Control: no-store, no-cache, private
 * - Set-Cookie headers (for security)
 * - Vary: * or complex Vary headers
 *
 * This middleware adds explicit cache control for responses that should be cached at the edge
 */
export function edgeCache(options: {
  /** Cache TTL in seconds (default: 60) */
  ttl?: number;
  /** Only cache GET requests by default */
  methods?: string[];
}): MiddlewareHandler {
  const { ttl = 60, methods = ['GET'] } = options;

  return async (c, next) => {
    // Only cache GET requests by default
    if (!methods.includes(c.req.method)) {
      await next();
      return;
    }

    // Access global caches API (Cloudflare Workers runtime)
    const cache = (globalThis as any).caches?.default;
    if (!cache) {
      await next();
      return;
    }

    // Build cache key from URL
    const cacheUrl = new URL(c.req.url);
    const cacheKey = new Request(cacheUrl.toString(), {
      method: c.req.method,
      headers: c.req.header(),
    });

    // Try to get from cache
    const cached = await cache.match(cacheKey);
    if (cached) {
      // Return cached response
      return c.newResponse(cached.body, cached);
    }

    // Not in cache, proceed with request
    await next();

    // Only cache successful responses
    if (c.res.status < 200 || c.res.status >= 300) {
      return;
    }

    // Don't cache if response has Set-Cookie
    if (c.res.headers.get('Set-Cookie')) {
      return;
    }

    // Clone the response before caching
    const responseToCache = new Response(c.res.body, c.res);

    // Add Cache-Control for edge caching
    const existingCacheControl = c.res.headers.get('Cache-Control');
    if (!existingCacheControl || existingCacheControl.includes('no-cache')) {
      c.res.headers.set('Cache-Control', `public, max-age=${ttl}`);
    }

    // Store in cache
    await cache.put(cacheKey, responseToCache.clone());
  };
}

/**
 * Invalidate cache by URL pattern
 * Useful for cache invalidation after data updates
 */
export async function invalidateCachePattern(url: string): Promise<void> {
  const cache = (globalThis as any).caches?.default;
  if (!cache) return;
  // Cache API doesn't support pattern matching natively
  // This would need to be implemented with a key-tracking system
  // For now, this is a placeholder for future implementation
}

/**
 * Cache-aware request helper that bypasses cache when needed
 */
export async function fetchWithCache(
  url: string,
  options: RequestInit = {},
  bypassCache = false
): Promise<Response> {
  if (bypassCache) {
    return fetch(url, {
      ...options,
      headers: {
        ...options.headers,
        'Cache-Control': 'no-cache',
      },
    });
  }

  const cache = (globalThis as any).caches?.default;
  if (!cache) {
    return fetch(url, options);
  }

  const cacheKey = new Request(url, options);

  const cached = await cache.match(cacheKey);
  if (cached) {
    return cached;
  }

  const response = await fetch(url, options);

  if (response.ok && !response.headers.get('Set-Cookie')) {
    await cache.put(cacheKey, response.clone());
  }

  return response;
}
