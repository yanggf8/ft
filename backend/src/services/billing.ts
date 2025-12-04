/**
 * Billing Service
 * - 免費試用期管理
 * - 訂閱狀態檢查
 * - Stripe 整合預留
 */

const TRIAL_DAYS = 30; // 免費試用天數

export interface UserBilling {
  tier: 'free' | 'premium' | 'professional';
  isTrialing: boolean;
  trialEndsAt: string | null;
  hasAccess: boolean; // 是否有 premium 功能權限
}

export function getTrialEndDate(): string {
  const date = new Date();
  date.setDate(date.getDate() + TRIAL_DAYS);
  return date.toISOString();
}

export function checkUserAccess(user: { subscription_tier: string; trial_ends_at: string | null }): UserBilling {
  const now = new Date();
  const trialEndsAt = user.trial_ends_at;
  const isTrialing = trialEndsAt ? new Date(trialEndsAt) > now : false;
  const tier = user.subscription_tier as UserBilling['tier'];

  // 有權限條件：premium/professional 訂閱，或在試用期內
  const hasAccess = tier !== 'free' || isTrialing;

  return { tier, isTrialing, trialEndsAt, hasAccess };
}

export function isPremiumFeature(feature: string): boolean {
  const premiumFeatures = ['ai_interpret', 'detailed_analysis', 'export_pdf'];
  return premiumFeatures.includes(feature);
}
