// What the home page shows from one list of emotes: the hero cluster, the Top / New strip and
// the chat preview lines. Pure, so it's tested without a browser.
import type { ChatEmote, ChatLine } from '$lib/components/ChatPreview.svelte';
import type { CardEmote } from '$lib/components/EmoteTile.svelte';
import { isWide, ready } from '$lib/emote';

export type HomeEmote = CardEmote & { createdAt: string; flags: { zeroWidth: boolean } };

const zw = (e: HomeEmote) => e.flags.zeroWidth;

/** The hero cluster: a big animated one first, a wide one, one zero-width overlay, the rest plain. */
export type Cluster = { tiles: HomeEmote[]; wide: HomeEmote | null; base: HomeEmote | null; overlay: HomeEmote | null };

export function cluster(all: HomeEmote[], n = 7): Cluster {
  const ok = all.filter(ready);
  const used = new Set<string>();
  const take = (f: (e: HomeEmote) => boolean) => {
    const e = ok.find((x) => !used.has(x.id) && f(x)) ?? null;
    if (e) used.add(e.id);
    return e;
  };
  const overlay = take(zw);
  const base = overlay ? take((e) => !zw(e) && !e.animated && !isWide(e)) : null;
  const wide = take((e) => isWide(e) && !zw(e));
  // Two animated ones lead (they play), then still ones fill the rest.
  const tiles: HomeEmote[] = [];
  for (const f of [(e: HomeEmote) => e.animated, (e: HomeEmote) => e.animated]) {
    const e = take((x) => f(x) && !zw(x) && !isWide(x));
    if (e) tiles.push(e);
  }
  while (tiles.length < n) {
    const e = take((x) => !zw(x) && !isWide(x));
    if (!e) break;
    tiles.push(e);
  }
  return { tiles, wide, base: overlay ? base : null, overlay: base ? overlay : null };
}

export type Sort = 'top' | 'new';

/**
 * The strip: square emotes only, so two rows on desktop stay two rows. The list arrives
 * already in order (the API's TOP or NEW), so this only filters and cuts.
 */
export function strip(all: HomeEmote[], n = 20): HomeEmote[] {
  return all.filter((e) => !isWide(e)).slice(0, n);
}

/** Made-up viewers (cyex-design), and lines that read like a real, slightly silly chat. */
export function chat(all: HomeEmote[], platform: 'twitch' | 'kick'): ChatLine[] {
  const ok = all.filter(ready);
  const plain = ok.filter((e) => !zw(e) && !isWide(e));
  const over = ok.find(zw);
  const wide = ok.find((e) => isWide(e) && !zw(e));
  if (plain.length < 8) return [];
  const off = platform === 'twitch' ? 0 : 4;
  const p = (i: number): ChatEmote => plain[(i + off) % plain.length];
  const lines: ChatLine[] = [
    { user: 'lumen_fox', parts: [{ text: 'new set just dropped ' }, { emote: p(0) }] },
    { user: 'quietoak', parts: [{ emote: p(1) }, { text: ' ' }, { emote: p(1) }, { text: ' ' }, { emote: p(1) }] },
    { user: 'nora_byte', parts: [{ text: 'who made this one ' }, { emote: p(2) }] },
    { user: 'pixelmoth', parts: [{ text: 'stacked ' }, { emote: p(3) }, ...(over ? [{ text: ' ' }, { emote: { ...over, zeroWidth: true } }] : [])] },
    { user: 'sunnyfern', parts: [{ text: 'gg ' }, { emote: p(4) }] },
    { user: 'mossgate', parts: wide ? [{ emote: wide }, { text: ' this is the energy' }] : [{ emote: p(5) }, { text: ' this is the energy' }] },
    { user: 'tidalwren', parts: [{ text: 'brb, tea ' }, { emote: p(6) }] },
    { user: 'lumen_fox', parts: [{ emote: p(7) }] },
  ];
  return lines;
}

/** Which install card leads: the browser the page is open in, when it can take the extension. */
export type Browser = 'chrome' | 'firefox' | 'other';

export function browserOf(ua: string): Browser {
  if (/iPhone|iPad|iPod/.test(ua)) return 'other'; // no extensions on iOS browsers
  if (/Firefox\//.test(ua)) return 'firefox';
  if (/Android/.test(ua)) return 'other'; // Chrome on Android takes no extensions
  if (/Chrome\/|Chromium\//.test(ua)) return 'chrome'; // Edge, Brave, Opera and Vivaldi too
  return 'other';
}
