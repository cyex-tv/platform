// The developer platform in words and numbers: API key usage (ADR 0033) and webhook events and
// deliveries (ADR 0034). Pure helpers, so the panels stay small and this is tested alone.
import type { EventType } from '$lib/gql/graphql';
import { count, dateTime } from '$lib/format';

/** Keys one account may hold, and webhooks one channel may have (the API's limits). */
export const MAX_KEYS = 10;
export const MAX_WEBHOOKS = 5;
/** Days of usage the API returns. */
export const USAGE_DAYS = 30;

/** The events a webhook can ask for, in the order the form lists them. */
export const WEBHOOK_EVENTS: { id: EventType; label: string }[] = [
  { id: 'SET_EMOTE_ADDED' as EventType, label: 'Emote added' },
  { id: 'SET_EMOTE_REMOVED' as EventType, label: 'Emote removed' },
  { id: 'SET_EMOTE_RENAMED' as EventType, label: 'Emote renamed' },
  { id: 'CHANNEL_ACTIVE_SET_CHANGED' as EventType, label: 'Active set changed' },
  { id: 'EMOTE_UPDATED' as EventType, label: 'Emote approved or changed' },
  { id: 'COSMETIC_CREATED' as EventType, label: 'Cosmetic created' },
  { id: 'COSMETIC_UPDATED' as EventType, label: 'Cosmetic updated' },
  { id: 'COSMETIC_DELETED' as EventType, label: 'Cosmetic deleted' },
];

export const eventLabel = (id: string): string => WEBHOOK_EVENTS.find((e) => e.id === id)?.label ?? id;

/** `2026-11-26` for a date in UTC. */
export function utcDay(d: Date): string {
  return d.toISOString().slice(0, 10);
}

/**
 * A key's last `days` UTC days, oldest first, with 0 for days without requests: the bars of the
 * usage view. Days outside the window are dropped.
 */
export function usageBars(usage: { day: string; requests: number }[], now = new Date(), days = USAGE_DAYS): { day: string; requests: number }[] {
  const by = new Map(usage.map((u) => [u.day, u.requests]));
  const out: { day: string; requests: number }[] = [];
  for (let i = days - 1; i >= 0; i--) {
    const day = utcDay(new Date(now.getTime() - i * 86_400_000));
    out.push({ day, requests: by.get(day) ?? 0 });
  }
  return out;
}

/** The requests of a key over the window, and today's. */
export function usageTotals(usage: { day: string; requests: number }[], now = new Date()): { total: number; today: number } {
  const bars = usageBars(usage, now);
  return { total: bars.reduce((n, b) => n + b.requests, 0), today: bars[bars.length - 1]?.requests ?? 0 };
}

/** The hint under a key's name: when it was made and last used. */
export function keyHint(k: { createdAt: string; lastUsedAt?: string | null }): string {
  const used = k.lastUsedAt ? `last used ${dateTime(k.lastUsedAt)}` : 'never used';
  return `Made ${dateTime(k.createdAt)}, ${used}`;
}

/** `12.345 requests in 30 days, 120 today`. */
export function usageLine(usage: { day: string; requests: number }[], now = new Date()): string {
  const t = usageTotals(usage, now);
  return `${count(t.total)} ${t.total === 1 ? 'request' : 'requests'} in ${USAGE_DAYS} days, ${count(t.today)} today`;
}

/** What a webhook's state says. */
export function webhookState(h: { enabled: boolean; disabledReason?: string | null; failedDeliveries: number }): string {
  if (!h.enabled) return h.disabledReason === 'failures' ? 'Off: too many failed deliveries' : h.disabledReason === 'blocked' ? 'Off: the account was blocked' : 'Off';
  if (h.failedDeliveries > 0) return `On, ${count(h.failedDeliveries)} failed in a row`;
  return 'On';
}

/** One delivery as a line: its state, status and timing. */
export function deliveryLine(d: { state: string; attempts: number; responseStatus?: number | null; error?: string | null; durationMs?: number | null }): string {
  const parts: string[] = [d.state === 'delivered' ? 'Delivered' : d.state === 'failed' ? 'Failed' : 'Waiting'];
  if (d.responseStatus) parts.push(String(d.responseStatus));
  else if (d.error) parts.push(d.error);
  if (d.attempts > 1) parts.push(`${d.attempts} attempts`);
  if (d.durationMs != null && d.attempts > 0) parts.push(`${count(d.durationMs)} ms`);
  return parts.join(' · ');
}

/** A URL checked the way the API checks it, for a message before sending: https, a host, no credentials. */
export function urlProblem(raw: string): string | null {
  const v = raw.trim();
  if (!v) return 'Give the URL to send events to.';
  let u: URL;
  try {
    u = new URL(v);
  } catch {
    return "That isn't a URL.";
  }
  if (u.protocol !== 'https:') return 'Webhook URLs must start with https://.';
  if (u.username || u.password) return 'Leave the user name and password out of the URL.';
  if (u.port && u.port !== '443') return 'Webhook URLs use the https port, 443. Leave the port out.';
  return null;
}
