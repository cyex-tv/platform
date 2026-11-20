// Pure helpers for the emote page (src/routes/emotes/[id]): the back link, facts, chat lines,
// review copy.
import type { ChatEmote, ChatLine } from '$lib/components/ChatPreview.svelte';
import type { IconName } from '$lib/components/Icon.svelte';
import { isWide, type Picture } from '$lib/emote';
import { bytes, count } from '$lib/format';
import { visibilityOf } from '$lib/tool/visibility';
import type { EmoteFieldsFragment, EmoteState } from '$lib/gql/graphql';

export type PageEmote = EmoteFieldsFragment;

/** A channel that has the emote in its active set. */
export type EmoteChannel = {
  platform: 'TWITCH' | 'KICK';
  login: string;
  displayName: string;
  avatar?: string | null;
};

/** How many channels use the emote; null (the fact is hidden) when none do. */
export function channelCount(e: Pick<PageEmote, 'channelCount'>): number | null {
  return e.channelCount > 0 ? e.channelCount : null;
}

/** Where the back link goes: the browse view the reader came from, else all emotes. */
export function backTarget(from: URL | null | undefined, here: string): { href: string; label: string; back: boolean } {
  if (from && from.pathname !== here) {
    const p = from.pathname;
    const href = p + from.search;
    if (p === '/search') return { href, label: from.searchParams.get('q') ? 'Back to results' : 'Back to emotes', back: true };
    if (p.startsWith('/users/')) return { href, label: 'Back to creator', back: true };
    if (p.startsWith('/channels/')) return { href, label: 'Back to channel', back: true };
    if (p === '/') return { href, label: 'Back to the front page', back: true };
  }
  return { href: '/search', label: 'All emotes', back: false };
}

const FORMAT_NAMES: Record<string, string> = { WEBP: 'WebP', AVIF: 'AVIF' };

/** The file formats, in a stable order: `WebP, AVIF`. */
export function formats(files: PageEmote['files']): string[] {
  const seen = [...new Set(files.map((f) => f.format))];
  return seen.sort().reverse().map((f) => FORMAT_NAMES[f] ?? f);
}

/** The file shown at each scale (moving one for animated emotes, WebP first), smallest first. */
export function scaleFiles(files: PageEmote['files']): PageEmote['files'] {
  const out: PageEmote['files'] = [];
  for (const s of [1, 2, 3, 4]) {
    const at = files.filter((f) => f.scale === s && !f.isStatic);
    const pick = at.find((f) => f.format === 'WEBP') ?? at[0];
    if (pick) out.push(pick);
  }
  return out;
}

/** `900 B` for a file whose size the API sent; empty when it is unknown (missing or 0). */
export function fileSize(f: { size?: number | null }): string {
  return typeof f.size === 'number' && f.size > 0 ? bytes(f.size) : '';
}

/** The type in words: `Animated, zero-width, wide`. */
export function kind(e: PageEmote): string {
  const parts = [e.animated ? 'Animated' : 'Still'];
  if (e.flags.zeroWidth) parts.push('zero-width');
  if (isWide(e)) parts.push('wide');
  return parts.join(', ');
}

/** The small labels under the name. */
export function labels(e: PageEmote): string[] {
  return [e.animated && 'animated', e.flags.zeroWidth && 'zero-width', isWide(e) && 'wide', e.flags.nsfw && 'NSFW'].filter(
    Boolean,
  ) as string[];
}

/** One flag on the emote page's card: its icon, word, and what it means (the tooltip). */
export type Flag = { key: 'animated' | 'zero-width' | 'wide' | 'nsfw' | 'unlisted' | 'private'; label: string; icon: IconName; tip: string };

/**
 * The emote's flags, in reading order: how it moves, how it sits in chat, NSFW, then who can
 * find it (ADR 0020: unlisted and private say so). A still, public, ordinary emote has none.
 */
export function flagsOf(e: PageEmote): Flag[] {
  const out: Flag[] = [];
  if (e.animated) out.push({ key: 'animated', label: 'Animated', icon: 'play', tip: 'Moves in chat' });
  if (e.flags.zeroWidth) out.push({ key: 'zero-width', label: 'Zero-width', icon: 'layers', tip: 'Sits on top of the emote before it' });
  if (e.flags.wide || isWide(e)) out.push({ key: 'wide', label: 'Wide', icon: 'wide', tip: 'At least 1,75 times as wide as it is tall' });
  if (e.flags.nsfw) out.push({ key: 'nsfw', label: 'NSFW', icon: 'eye-off', tip: 'Not safe for work: hidden unless a viewer shows NSFW emotes' });
  const vis = visibilityOf(e);
  if (vis === 'UNLISTED') out.push({ key: 'unlisted', label: 'Unlisted', icon: 'link', tip: 'Search leaves it out: only people with the link find it' });
  else if (vis === 'PRIVATE') out.push({ key: 'private', label: 'Private', icon: 'shield', tip: 'Only its creator and their editors can add it to sets' });
  return out;
}

/** Channels on one page of the Channels tab. */
export const CHANNEL_PAGE = 24;

/** `1–24 of 26`: which rows a page shows, for the pager. Empty when there is nothing. */
export function rangeLabel(from: number, shown: number, total?: number | null): string {
  if (shown <= 0) return '';
  const span = `${count(from + 1)}–${count(from + shown)}`;
  return total != null ? `${span} of ${count(total)}` : span;
}

type SetEmote = Picture & { name: string; flags?: { zeroWidth: boolean } | null };
/** Emotes the chat preview can put beside (or under) this one: the global set. */
export type ChatPool = SetEmote[];

const chat = (e: SetEmote, name = e.name): ChatEmote => ({ ...e, name, zeroWidth: e.flags?.zeroWidth ?? false });

/** An ordinary emote from the global set to stand under a zero-width one (not wide, not zero-width). */
export function baseEmote<T extends SetEmote>(pool: T[], not: string): T | null {
  const fits = pool.filter((e) => e.name !== not && !e.flags?.zeroWidth);
  return fits.find((e) => !isWide(e)) ?? fits[0] ?? null;
}

/** Three made-up chat lines showing the emote, layered over a global emote when it's zero-width. */
export function chatLines(e: PageEmote, pool: SetEmote[]): ChatLine[] {
  const me = chat(e);
  const base = baseEmote(pool, e.name);
  const other = baseEmote(
    pool.filter((x) => x !== base),
    e.name,
  );
  const lines: ChatLine[] = [];
  if (other) lines.push({ user: 'nora_byte', parts: [{ text: 'that ending ' }, { emote: chat(other) }] });
  if (e.flags.zeroWidth && base) {
    lines.push({ user: 'lumen_fox', parts: [{ text: 'perfect ' }, { emote: chat(base) }, { text: ' ' }, { emote: me }] });
    lines.push({ user: 'quietoak', parts: [{ emote: chat(base) }, { text: ' ' }, { emote: me }] });
  } else {
    lines.push({ user: 'lumen_fox', parts: [{ text: 'perfect ' }, { emote: me }] });
    lines.push({ user: 'quietoak', parts: [{ emote: me }, { text: ' ' }, { emote: me }] });
  }
  return lines;
}

/** What the review status says, in plain words. */
export function trust(state: EmoteState): { status: string; line: string; tone: 'ok' | 'wait' | 'off' } {
  switch (state) {
    case 'APPROVED':
      return {
        status: 'Approved',
        tone: 'ok',
        line: 'A moderator checked it before it went public. If it breaks the rules, report it and a moderator looks again.',
      };
    case 'PENDING':
      return {
        status: 'Waiting for review',
        tone: 'wait',
        line: 'A moderator checks every emote before it shows up in search or chat. Until then only you and moderators can open this page.',
      };
    case 'REJECTED':
      return {
        status: 'Not approved',
        tone: 'off',
        line: 'A moderator didn’t approve it, so it isn’t public and can’t be added to sets. Your notifications say why.',
      };
    case 'TAKEN_DOWN':
      return {
        status: 'Taken down',
        tone: 'off',
        line: 'A moderator took it down. It no longer shows up in search, in sets or in chat.',
      };
  }
}

/** Whether to offer Report: approved emotes, and not your own (you can't report what you made). */
export function canReport(e: Pick<PageEmote, 'state' | 'owner'>, meId: string | null | undefined): boolean {
  return e.state === 'APPROVED' && !(meId && e.owner?.id === meId);
}

/**
 * The line over a transfer request's buttons, "pixelmoth asks to give it to you.": who asked, and
 * who gets it. Someone asking to give it to themselves (an owner moving it into their own other
 * account is a different row) reads "to themselves", never their name twice.
 */
export function transferLine(
  r: { requestedBy: { id: string; displayName: string }; to: { id: string; displayName: string } },
  meId: string | null | undefined,
): { asker: 'you' | { name: string }; target: 'you' | 'yourself' | 'themselves' | { name: string } } {
  const asker = r.requestedBy.id === meId ? 'you' : { name: r.requestedBy.displayName };
  const target = r.to.id === r.requestedBy.id ? (r.to.id === meId ? 'yourself' : 'themselves') : r.to.id === meId ? 'you' : { name: r.to.displayName };
  return { asker, target };
}
