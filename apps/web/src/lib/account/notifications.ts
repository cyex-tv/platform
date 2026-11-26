// Notifications in words. The API sends a kind and a JSON payload (as text) with the IDs and
// names the notification is about. Everything here is defensive: a payload that isn't JSON, isn't
// an object or lacks a field still gives a sentence, and the result is plain text for the page to
// print as text (never as HTML). Names from other people go through `iso` so they can't reorder
// the words around them. Pure, so it is tested.
import type { IconName } from '$lib/components/Icon.svelte';
import { iso } from '$lib/format';

export type Line = {
  icon: IconName;
  /** One sentence. Plain text. */
  text: string;
  /** A page it makes sense to open, or null. */
  href: string | null;
};

/** The payload as an object, or an empty one when it isn't JSON or isn't an object. */
export function parsePayload(payload: string | null | undefined): Record<string, unknown> {
  if (typeof payload !== 'string' || !payload) return {};
  try {
    const v: unknown = JSON.parse(payload);
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
  } catch {
    return {};
  }
}

/** A non-empty string field, trimmed and cut to a readable length; otherwise null. */
function str(data: Record<string, unknown>, key: string, max = 200): string | null {
  const v = data[key];
  if (typeof v !== 'string') return null;
  const t = v.trim();
  if (!t) return null;
  return t.length > max ? `${t.slice(0, max - 1)}…` : t;
}

/** An ID that is safe to put in an address, or null. */
function id(data: Record<string, unknown>, key: string): string | null {
  const v = data[key];
  return typeof v === 'string' && /^[A-Za-z0-9_-]{1,64}$/.test(v) ? v : null;
}

const noun = (kind: string | null) => (kind === 'badge' ? 'badge' : kind === 'paint' ? 'paint' : 'cosmetic');
const withReason = (sentence: string, reason: string | null) => (reason ? `${sentence} Reason: ${iso(reason)}` : sentence);

/** What a notification says, which icon it gets and where it leads. Unknown kinds get a neutral line. */
export function describe(kind: string, payload: string | null | undefined): Line {
  const d = parsePayload(payload);
  const name = str(d, 'name', 100);
  const emote = id(d, 'emote_id');
  const reason = str(d, 'reason');
  const who = (key: string) => {
    const n = str(d, key, 100);
    return n ? iso(n) : 'Someone';
  };
  const owner = who('owner_name');
  const editor = who('editor_name');
  const asEditor = d.role !== 'owner';
  const owned = id(d, 'owner_id');

  switch (kind) {
    case 'emote.approved':
      return { icon: 'check', text: `${name ? iso(name) : 'Your emote'} was approved.`, href: emote ? `/emotes/${emote}` : null };
    case 'emote.rejected':
      return {
        icon: 'x',
        text: withReason(`${name ? iso(name) : 'Your emote'} was rejected.`, reason),
        href: emote ? `/me/emotes/${emote}` : null,
      };
    case 'emote.failed':
      return {
        icon: 'x',
        text: `${name ? iso(name) : 'Your emote'} couldn’t be processed.`,
        href: emote ? `/emotes/${emote}` : null,
      };
    case 'emote.taken_down':
      return {
        icon: 'ban',
        text: withReason(`${name ? iso(name) : 'Your emote'} was taken down.`, reason),
        href: emote ? `/me/emotes/${emote}` : null,
      };
    case 'editor.added':
      return asEditor
        ? { icon: 'users', text: `${owner} made you an editor of their account.`, href: owned ? `/users/${owned}` : null }
        : { icon: 'users', text: `${editor} is now an editor of your account.`, href: '/me/editors' };
    case 'editor.changed':
      return asEditor
        ? { icon: 'users', text: `${owner} changed what you can do on their account.`, href: owned ? `/users/${owned}` : null }
        : { icon: 'users', text: `What ${editor} can do on your account changed.`, href: '/me/editors' };
    case 'editor.removed':
      return asEditor
        ? { icon: 'users', text: `You’re no longer an editor of ${owner}’s account.`, href: null }
        : { icon: 'users', text: `${editor} is no longer an editor of your account.`, href: '/me/editors' };
    // Editors can't ask for a transfer any more (ADR 0014, amendment 2026/11/22): an old request
    // reads as what happened, with nothing to answer; the emote stayed with its owner.
    case 'emote.transfer_requested': {
      const what = name ? iso(name) : 'your emote';
      return {
        icon: 'arrow-right',
        text: `${who('actor_name')} asked to give ${what} to ${id(d, 'to_id') && id(d, 'to_id') === id(d, 'actor_id') ? 'themselves' : who('to_name')}. Transfer requests are gone: only you can give your emotes away.`,
        href: emote ? `/emotes/${emote}` : null,
      };
    }
    case 'emote.transferred': {
      const what = name ? iso(name) : 'An emote';
      const role = str(d, 'role', 10);
      const text =
        role === 'to'
          ? `${what} is yours now, from ${who('from_name')}.`
          : role === 'editor'
            ? `${who('from_name')} accepted: ${what} went to ${who('to_name')}.`
            : `${what} went to ${who('to_name')}.`;
      return { icon: 'arrow-right', text, href: emote && role !== 'from' ? `/emotes/${emote}` : null };
    }
    case 'emote.transfer_declined':
      return {
        icon: 'x',
        text: `${who('from_name')} declined giving ${name ? iso(name) : 'their emote'} to ${who('to_name')}.`,
        href: emote ? `/emotes/${emote}` : null,
      };
    case 'emote.transfer_cancelled':
      if (str(d, 'reason', 20) === 'retired') {
        const what = name ? iso(name) : 'an emote';
        return str(d, 'role', 10) === 'editor'
          ? { icon: 'x', text: `Your request to give ${what} to ${who('to_name')} was withdrawn: only an emote’s owner can give it away now.`, href: null }
          : { icon: 'x', text: `The request to give ${what} to ${who('to_name')} was withdrawn. It stays yours: only you can give your emotes away.`, href: emote ? `/emotes/${emote}` : null };
      }
      return {
        icon: 'x',
        text: `${who('actor_name')} withdrew the request to give ${name ? iso(name) : 'your emote'} away.`,
        href: emote ? `/me/emotes/${emote}` : null,
      };
    case 'cosmetic.granted': {
      const n = noun(str(d, 'kind', 20));
      return { icon: 'brush', text: withReason(`You got ${name ? `the ${iso(name)} ${n}` : `a new ${n}`}.`, reason), href: '/me/cosmetics' };
    }
    case 'cosmetic.revoked': {
      const n = noun(str(d, 'kind', 20));
      return { icon: 'brush', text: withReason(`${name ? `The ${iso(name)} ${n}` : `A ${n}`} was taken away.`, reason), href: '/me/cosmetics' };
    }
    case 'webhook.disabled': {
      const login = str(d, 'login', 64);
      return {
        icon: 'x',
        text: `A webhook of ${login ? iso(login) : 'your channel'} was turned off after 5 failed deliveries in a row. Fix the receiver, then turn it on again.`,
        href: '/me/channels',
      };
    }
    default:
      return { icon: 'bell', text: 'There’s an update on your account.', href: null };
  }
}

/** The badge on the bell: nothing at 0, the number up to 99, then `99+`. */
export function badge(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return '';
  return n > 99 ? '99+' : String(Math.trunc(n));
}
