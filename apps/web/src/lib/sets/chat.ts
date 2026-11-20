// The made-up chat on a set's page (/sets/<id>): a handful of lines that show how the set's emotes
// sit in a Twitch or Kick chat: plain ones, wide ones, zero-width ones stacked over the emote
// before them, and the set's owner saying something (their name in their paint). The viewers are
// invented. Pure, so the page and the tests share it.
import type { ChatEmote, ChatLine, ChatPart } from '$lib/components/ChatPreview.svelte';
import { isWide, type Picture } from '$lib/emote';
import type { Look } from '$lib/cosmetics/store.svelte';

type Member = { name: string; zeroWidth: boolean; emote: Picture };

const VIEWERS = ['lumen_fox', 'quietoak', 'nora_byte', 'pixelmoth', 'sunnyfern', 'mossgate'];

const asEmote = (m: Member): ChatEmote => ({ ...m.emote, name: m.name, zeroWidth: m.zeroWidth });

/** Up to `max` lines from the first `pool` members. No lines when the set has nothing to stand on (no ordinary emote). */
export function setChatLines(members: readonly Member[], owner: { name: string; look?: Look } | null, max = 6, pool = 24): ChatLine[] {
  const picked = members.slice(0, pool);
  const base = picked.filter((m) => !m.zeroWidth).map(asEmote);
  const over = picked.filter((m) => m.zeroWidth).map(asEmote);
  if (base.length === 0) return [];
  const wide = base.filter((e) => isWide(e));
  const at = (list: ChatEmote[], i: number) => list[i % list.length];
  const t = (text: string): ChatPart => ({ text });
  const e = (emote: ChatEmote): ChatPart => ({ emote });

  const plans: { by: 'owner' | number; parts: ChatPart[] }[] = [
    { by: 'owner', parts: [t('new set is live '), e(at(base, 0))] },
    { by: 0, parts: [e(at(base, 1)), t(' '), e(at(base, 1))] },
    { by: 1, parts: over.length ? [t('wait for it '), e(at(base, 2)), t(' '), e(at(over, 0))] : [t('same '), e(at(base, 2))] },
    { by: 2, parts: wide.length ? [e(at(wide, 0)), t(' '), e(at(base, 3))] : [t('gg '), e(at(base, 3))] },
    { by: 'owner', parts: over.length > 1 ? [e(at(base, 4)), t(' '), e(at(over, 0)), t(' '), e(at(over, 1))] : [t('thanks for coming '), e(at(base, 4))] },
    { by: 3, parts: [t('chat is moving '), e(at(base, 5)), t(' '), e(at(base, 6))] },
  ];
  const viewers = VIEWERS.filter((v) => v !== owner?.name.toLowerCase());
  return plans.slice(0, max).map((p, i) => {
    if (p.by === 'owner' && owner) return { user: owner.name, look: owner.look, parts: p.parts };
    return { user: viewers[(typeof p.by === 'number' ? p.by : i) % viewers.length], parts: p.parts };
  });
}
