// What the command palette offers for a piece of text, apart from the live emote search.
import { count } from './format';

export type Jump = { id: string; label: string; hint?: string; href: string };

/** The site's pages, in the order the palette lists them. */
export const PAGES: Jump[] = [
  { id: 'page-browse', label: 'Browse emotes', hint: 'Search, sort and filter', href: '/search' },
  { id: 'page-help', label: 'Help', hint: 'Questions and answers', href: '/help' },
  { id: 'page-download', label: 'Download', hint: 'The extension for Chrome and Firefox', href: '/download' },
  { id: 'page-cosmetics', label: 'Cosmetics', hint: 'Every badge and paint', href: '/cosmetics' },
  { id: 'page-developers', label: 'Developers', hint: 'The public API', href: '/developers' },
  { id: 'page-reference', label: 'API reference', hint: 'Every query and endpoint', href: '/developers/reference' },
  { id: 'page-me', label: 'Your emotes', hint: 'Uploads, sets and channels', href: '/me' },
];

/**
 * Whether the text could be a channel login. Twitch logins are 4 to 25 letters, digits and
 * underscores; Kick slugs also allow hyphens. Spaces mean it's an emote search, not a login.
 */
export function asLogin(text: string): { twitch: boolean; kick: boolean; login: string } | null {
  const login = text.trim().replace(/^@/, '').toLowerCase();
  if (!/^[a-z0-9_-]{3,25}$/.test(login)) return null;
  const twitch = /^[a-z0-9_]{4,25}$/.test(login) && !login.startsWith('_');
  return { twitch, kick: true, login };
}

/** "Twitch channel x" and "Kick channel x", when the text looks like a login. */
export function channelJumps(text: string): Jump[] {
  const l = asLogin(text);
  if (!l) return [];
  const out: Jump[] = [];
  if (l.twitch) out.push({ id: 'ch-twitch', label: `Twitch channel ${l.login}`, href: `/channels/twitch/${l.login}` });
  if (l.kick) out.push({ id: 'ch-kick', label: `Kick channel ${l.login}`, href: `/channels/kick/${l.login}` });
  return out;
}

/** Pages whose name or hint contains every word of the text; all of them for no text. */
export function pageJumps(text: string): Jump[] {
  const words = text.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return PAGES;
  return PAGES.filter((p) => {
    const hay = `${p.label} ${p.hint ?? ''}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}

/** Moves the active row by `step`, wrapping at both ends. -1 means nothing is active. */
export function move(active: number, step: number, length: number): number {
  if (length === 0) return -1;
  if (active < 0) return step > 0 ? 0 : length - 1;
  return (active + step + length) % length;
}

/** Whether a key press was typed into a field (so "/" there types a slash). */
export function typing(target: EventTarget | null): boolean {
  if (!target || typeof Element === 'undefined' || !(target instanceof Element)) return false;
  const el = target as HTMLElement;
  return el.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(el.tagName);
}

/**
 * What the palette's status line reads out once results settle: "6 emotes, 2 users, 2 channels, 1 page.",
 * or a sentence saying nothing matched. Empty when there's nothing to say.
 */
export function announce(n: { emotes: number; users?: number; channels: number; pages: number }, text: string): string {
  const part = (v: number, one: string, many: string) => (v ? `${count(v)} ${v === 1 ? one : many}` : '');
  const parts = [part(n.emotes, 'emote', 'emotes'), part(n.users ?? 0, 'user', 'users'), part(n.channels, 'channel', 'channels'), part(n.pages, 'page', 'pages')].filter(Boolean);
  if (parts.length) return `${parts.join(', ')}.`;
  return text ? emptyText(text) : '';
}

/** The line shown (and read) when nothing matches the text. */
export function emptyText(text: string): string {
  return `No emotes, users or pages match “${text}”.`;
}
