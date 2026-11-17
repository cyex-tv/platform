// Chat lines for a channel's preview, made only of the channel's own emotes and made-up viewers.
import type { ChatLine, ChatEmote } from '../ChatPreview.svelte';

const VIEWERS = ['lumen_fox', 'quietoak', 'nora_byte', 'pixelmoth', 'sunnyfern', 'mossgate'];
const SAYS = ['', 'that was close ', 'chat is moving ', 'brb ', 'same ', 'gg '];

/** Up to `max` lines, each with one or two of `emotes` (zero-width ones are left out, they need a base). */
export function chatLines(emotes: ChatEmote[], max = 6): ChatLine[] {
  const base = emotes.filter((e) => !e.zeroWidth);
  if (base.length === 0) return [];
  const out: ChatLine[] = [];
  for (let i = 0; i < max; i++) {
    const a = base[i % base.length];
    const parts: ChatLine['parts'] = [{ text: SAYS[i % SAYS.length] }, { emote: a }];
    if (i % 3 === 1 && base.length > 1) parts.push({ text: ' ' }, { emote: base[(i + 1) % base.length] });
    if (i % 3 === 2) parts.push({ text: ' ' }, { emote: a });
    out.push({ user: VIEWERS[i % VIEWERS.length], parts: parts.filter((p) => !('text' in p) || p.text !== '') });
  }
  return out;
}
