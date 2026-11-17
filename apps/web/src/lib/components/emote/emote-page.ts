// Pure helpers for the emote page (src/routes/emotes/[id]): the back link, facts, chat lines,
// review copy, and the slots for data the API doesn't send yet.
import type { ChatEmote, ChatLine } from '$lib/components/ChatPreview.svelte';
import { isWide, type Picture } from '$lib/emote';
import type { EmoteFieldsFragment, EmoteState } from '$lib/gql/graphql';

export type PageEmote = EmoteFieldsFragment;

/** A channel that has the emote in its active set. */
export type EmoteChannel = {
  platform: 'TWITCH' | 'KICK';
  login: string;
  displayName: string;
  avatar?: string | null;
};

/** How many channels use the emote. Not in the schema yet: null hides the fact. */
export function channelCount(_e: PageEmote): number | null {
  // wire: channelCount (Emote.channelCount)
  return null;
}

/** Up to 8 channels using the emote. Not in the schema yet: null hides "Used in". */
export function emoteChannels(_e: PageEmote): EmoteChannel[] | null {
  // wire: emoteChannels (Query.emoteChannels(id, limit: 8))
  return null;
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
