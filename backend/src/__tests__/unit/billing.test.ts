import { describe, it, expect } from 'vitest';
import { checkUserAccess, getTrialEndDate, isPremiumFeature } from '../../services/billing';

describe('Billing Service', () => {
  describe('checkUserAccess', () => {
    it('should grant access to trialing users', () => {
      const futureDate = new Date(Date.now() + 10 * 24 * 60 * 60 * 1000).toISOString();
      const result = checkUserAccess({
        subscription_tier: 'free',
        trial_ends_at: futureDate
      });

      expect(result.hasAccess).toBe(true);
      expect(result.isTrialing).toBe(true);
      expect(result.tier).toBe('free');
    });

    it('should deny access to expired trial users', () => {
      const pastDate = new Date(Date.now() - 10 * 24 * 60 * 60 * 1000).toISOString();
      const result = checkUserAccess({
        subscription_tier: 'free',
        trial_ends_at: pastDate
      });

      expect(result.hasAccess).toBe(false);
      expect(result.isTrialing).toBe(false);
    });

    it('should grant access to premium users', () => {
      const result = checkUserAccess({
        subscription_tier: 'premium',
        trial_ends_at: null
      });

      expect(result.hasAccess).toBe(true);
      expect(result.tier).toBe('premium');
    });

    it('should grant access to professional users', () => {
      const result = checkUserAccess({
        subscription_tier: 'professional',
        trial_ends_at: null
      });

      expect(result.hasAccess).toBe(true);
      expect(result.tier).toBe('professional');
    });
  });

  describe('getTrialEndDate', () => {
    it('should return date 30 days in future', () => {
      const trialEnd = new Date(getTrialEndDate());
      const now = new Date();
      const diffDays = Math.floor((trialEnd.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

      expect(diffDays).toBeGreaterThanOrEqual(29);
      expect(diffDays).toBeLessThanOrEqual(30);
    });
  });

  describe('isPremiumFeature', () => {
    it('should identify premium features', () => {
      expect(isPremiumFeature('ai_interpret')).toBe(true);
      expect(isPremiumFeature('detailed_analysis')).toBe(true);
      expect(isPremiumFeature('export_pdf')).toBe(true);
    });

    it('should return false for non-premium features', () => {
      expect(isPremiumFeature('basic_chart')).toBe(false);
      expect(isPremiumFeature('unknown')).toBe(false);
    });
  });
});
