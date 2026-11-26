// Mock API: signed in/developers. See ../mock-api.mjs. Your API keys (ADR 0033) and your
// channels' webhooks (ADR 0034), in memory, with the API's limits and checks.
import { CHANNELS } from '../account.mjs';
import { err } from '../sets.mjs';

const KEYS = [];
const HOOKS = [];
const DELIVERIES = new Map();
const EVENTS = ['SET_EMOTE_ADDED', 'SET_EMOTE_REMOVED', 'SET_EMOTE_RENAMED', 'CHANNEL_ACTIVE_SET_CHANGED', 'EMOTE_UPDATED', 'COSMETIC_CREATED', 'COSMETIC_UPDATED', 'COSMETIC_DELETED'];
let next = 1;
const nid = () => `01JDEV${String(next++).padStart(20, '0')}`;
const now = () => new Date().toISOString();
const rand = (n) => Array.from({ length: n }, () => 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'[Math.floor(Math.random() * 62)]).join('');
const day = (offset) => new Date(Date.now() - offset * 86_400_000).toISOString().slice(0, 10);

// One key with a few days of use, so the usage bars show something.
KEYS.push({ id: nid(), name: 'chat-bot', prefix: 'cyex_live_Xq3f9LkA', createdAt: '2026-11-02T10:12:00.000Z', lastUsedAt: now(), usage: [6, 4, 3, 1, 0].map((d, i) => ({ day: day(d), requests: [120, 3400, 2210, 980, 455][i] })) });

// Two webhooks on the mock channel: one with a delivery in its log, one to delete.
export const MOCK_HOOK_ID = '01JDEVH00K00000000000000A1';
export const MOCK_HOOK_DELETABLE_ID = '01JDEVH00K00000000000000A2';
for (const [id, url, events] of [
  [MOCK_HOOK_ID, 'https://bot.example.com/cyex', ['SET_EMOTE_ADDED', 'SET_EMOTE_REMOVED']],
  [MOCK_HOOK_DELETABLE_ID, 'https://old.example.com/hook', ['EMOTE_UPDATED']],
]) {
  HOOKS.push({ id, url, events, enabled: true, disabledReason: null, failedDeliveries: 0, createdAt: '2026-11-20T09:00:00.000Z', channel: CHANNELS[0], secret: `cyexhook_${rand(43)}` });
  DELIVERIES.set(id, []);
}
DELIVERIES.get(MOCK_HOOK_ID).push({ id: '41', eventType: 'set.emote_added', eventId: '9021', state: 'delivered', attempts: 1, responseStatus: 200, error: null, durationMs: 88, createdAt: '2026-11-26T10:00:00.000Z', finishedAt: '2026-11-26T10:00:00.100Z' });

const channelOf = (platform, id) => CHANNELS.find((c) => c.platform === platform && c.platformChannelId === id);
const hookView = ({ channel, secret, ...h }) => h;

function urlProblem(raw) {
  let u;
  try {
    u = new URL(String(raw).trim());
  } catch {
    return "That isn't a URL.";
  }
  if (u.protocol !== 'https:') return 'Webhook URLs must start with https://.';
  if (u.username || u.password) return 'Leave the user name and password out of the URL.';
  const host = u.hostname.replace(/^\[|\]$/g, '');
  if (host === 'localhost' || !host.includes('.') && !host.includes(':') || /^(127\.|10\.|192\.168\.|169\.254\.|0\.)/.test(host) || host === '::1' || /\.(local|internal|localhost)$/.test(host))
    return 'That URL points at a private or local address.';
  return null;
}

function eventsProblem(events) {
  if (!events?.length) return 'Pick at least one event type.';
  const bad = events.find((e) => !EVENTS.includes(e));
  return bad ? `Webhooks can't subscribe to ${bad}.` : null;
}

export function signedInDevelopers(op, body, v) {
  switch (op) {
    case 'MyApiKeys':
      return { data: { myApiKeys: KEYS } };
    case 'CreateApiKey': {
      const name = String(v.name ?? '').trim();
      if (!name || name.length > 64) return err('Give the key a name of 1 to 64 characters.', 'bad_name');
      if (KEYS.length >= 10) return err('You have 10 API keys, the most you may. Revoke one first.', 'too_many_api_keys');
      const key = `cyex_live_${rand(43)}`;
      const apiKey = { id: nid(), name, prefix: key.slice(0, 18), createdAt: now(), lastUsedAt: null, usage: [] };
      KEYS.push(apiKey);
      return { data: { createApiKey: { key, apiKey } } };
    }
    case 'RevokeApiKey': {
      const at = KEYS.findIndex((k) => k.id === v.id);
      if (at >= 0) KEYS.splice(at, 1);
      return { data: { revokeApiKey: at >= 0 } };
    }
    case 'ChannelWebhooks': {
      const ch = channelOf(v.platform, v.id);
      if (!ch) return err('Not found.', 'not_found');
      return { data: { channelWebhooks: HOOKS.filter((h) => h.channel === ch).map(hookView) } };
    }
    case 'CreateWebhook': {
      const ch = channelOf(v.platform, v.id);
      if (!ch) return err('Not found.', 'not_found');
      const p = urlProblem(v.input?.url);
      if (p) return err(p, 'bad_webhook_url');
      const e = eventsProblem(v.input?.events);
      if (e) return err(e, 'bad_webhook_events');
      if (HOOKS.filter((h) => h.channel === ch).length >= 5) return err('This channel has 5 webhooks, the most it may. Delete one first.', 'too_many_webhooks');
      const secret = `cyexhook_${rand(43)}`;
      const hook = { id: nid(), url: String(v.input.url).trim(), events: [...new Set(v.input.events)], enabled: true, disabledReason: null, failedDeliveries: 0, createdAt: now(), channel: ch, secret };
      HOOKS.push(hook);
      DELIVERIES.set(hook.id, []);
      return { data: { createWebhook: { secret, webhook: hookView(hook) } } };
    }
    case 'UpdateWebhook': {
      const h = HOOKS.find((x) => x.id === v.id);
      if (!h) return err('Not found.', 'not_found');
      if (v.input.url != null) {
        const p = urlProblem(v.input.url);
        if (p) return err(p, 'bad_webhook_url');
        h.url = String(v.input.url).trim();
      }
      if (v.input.events != null) {
        const e = eventsProblem(v.input.events);
        if (e) return err(e, 'bad_webhook_events');
        h.events = [...new Set(v.input.events)];
      }
      if (v.input.enabled === true) Object.assign(h, { enabled: true, disabledReason: null, failedDeliveries: 0 });
      if (v.input.enabled === false) Object.assign(h, { enabled: false, disabledReason: 'manual' });
      return { data: { updateWebhook: hookView(h) } };
    }
    case 'DeleteWebhook': {
      const at = HOOKS.findIndex((x) => x.id === v.id);
      if (at < 0) return err('Not found.', 'not_found');
      HOOKS.splice(at, 1);
      return { data: { deleteWebhook: true } };
    }
    case 'RotateWebhookSecret': {
      const h = HOOKS.find((x) => x.id === v.id);
      if (!h) return err('Not found.', 'not_found');
      h.secret = `cyexhook_${rand(43)}`;
      return { data: { rotateWebhookSecret: { secret: h.secret, webhook: hookView(h) } } };
    }
    case 'TestWebhook': {
      const h = HOOKS.find((x) => x.id === v.id);
      if (!h) return err('Not found.', 'not_found');
      // The mock has no worker: a test is delivered at once.
      const d = { id: String(next++), eventType: 'webhook.test', eventId: null, state: 'delivered', attempts: 1, responseStatus: 204, error: null, durationMs: 142, createdAt: now(), finishedAt: now() };
      DELIVERIES.get(h.id).unshift(d);
      return { data: { testWebhook: { id: d.id, state: 'pending' } } };
    }
    case 'WebhookDeliveries': {
      if (!HOOKS.some((x) => x.id === v.id)) return err('Not found.', 'not_found');
      return { data: { webhookDeliveries: (DELIVERIES.get(v.id) ?? []).slice(0, 50) } };
    }
    default:
      return null;
  }
}
