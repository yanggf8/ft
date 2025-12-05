import { describe, it, expect, beforeEach } from 'vitest';
import { api } from '../lib/api';

describe('API Client', () => {
  beforeEach(() => {
    api.setSession(null);
    localStorage.clear();
  });

  describe('Session Management', () => {
    it('should store session in localStorage', () => {
      api.setSession('test-session-123');
      expect(localStorage.getItem('sessionId')).toBe('test-session-123');
      expect(api.getSession()).toBe('test-session-123');
    });

    it('should clear session', () => {
      api.setSession('test-session-123');
      api.setSession(null);
      expect(localStorage.getItem('sessionId')).toBeNull();
      expect(api.getSession()).toBeNull();
    });

    it('should retrieve session from localStorage', () => {
      localStorage.setItem('sessionId', 'stored-session');
      expect(api.getSession()).toBe('stored-session');
    });
  });
});
