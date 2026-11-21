<script lang="ts">
  // The announcement bar as it will show under the header (the same look as the real one, nothing
  // clickable): the kind's icon and word, a small emote or badge, the line and its link.
  import EmotePicture from '$lib/components/EmotePicture.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import type { Badge } from '$lib/cosmetics/store.svelte';
  import { ready, type Picture } from '$lib/emote';
  import type { AnnouncementKind } from '$lib/gql/graphql';
  import { KIND } from '$lib/shell/announcement';

  type Props = {
    kind: AnnouncementKind;
    text: string;
    /** The link's address when it is a safe one, else null (no link shows). */
    href?: string | null;
    linkLabel?: string;
    /** An emote's picture when it is known; `emoteSet` says one is chosen even when it is not. */
    emote?: Picture | null;
    emoteSet?: boolean;
    badge?: Badge | null;
  };
  let { kind, text, href = null, linkLabel = '', emote = null, emoteSet = false, badge = null }: Props = $props();
  const k = $derived(KIND[kind]);
  const line = $derived(text.trim());
</script>

<div class="bar" role="img" aria-label="Preview of the announcement">
  <span class="kind"><Icon name={k.icon} size={15} /><span class="word">{k.label}</span></span>
  {#if emote && ready(emote)}
    <span class="art"><EmotePicture {emote} alt="" class="pic" /></span>
  {:else if emoteSet}
    <span class="art"><span class="ph"></span></span>
  {:else if badge}
    <span class="art"><BadgeIcon {badge} size={18} /></span>
  {/if}
  <p class="text" class:none={!line}>
    <bdi>{line || 'Your line shows here.'}</bdi>
    {#if href}<span class="go">{linkLabel.trim() || 'Learn more'}<Icon name="chevron-right" size={14} /></span>{/if}
  </p>
  <span class="x"><Icon name="x" size={14} /></span>
</div>

<style>
  /* A flat strip, like the real bar: hairlines above and below, no box. */
  .bar { display: flex; align-items: flex-start; gap: 10px; min-height: 44px; padding: 10px 0; border-block: 1px solid var(--border); background: transparent; }
  .kind { flex: none; display: inline-flex; align-items: center; gap: 6px; min-height: 24px; color: var(--muted); font-size: var(--t-fine); font-weight: 600; }
  .kind :global(svg) { color: var(--dim); }
  .art { flex: none; display: inline-flex; align-items: center; min-height: 24px; }
  .art :global(.pic) { height: 22px; width: auto; max-width: 66px; object-fit: contain; }
  .ph { width: 22px; height: 22px; border: 1px dashed var(--border-strong); border-radius: var(--radius); }
  .text { flex: 1; min-width: 0; margin: 0; font-size: var(--t-nav); font-weight: 500; color: var(--text); line-height: 24px; overflow-wrap: anywhere; }
  .text.none { color: var(--dim); }
  .go { display: inline-flex; align-items: center; gap: 2px; margin-left: 8px; color: var(--accent-text); font-weight: 600; white-space: nowrap; }
  .x { flex: none; display: grid; place-items: center; width: 24px; height: 24px; color: var(--dim); }
</style>
