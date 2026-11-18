<script lang="ts">
  // The identity block of a creator or channel page: avatar, name, what it is, a line of facts and
  // the page's actions. It heads the SideRail on wide windows and sits above the content on phones
  // (the rail is a tab strip there), so the page always says whose it is. The name is not a
  // heading: the content's title is the page's one h1.
  import type { Snippet } from 'svelte';
  import { clean } from '$lib/format';
  import Avatar from '../Avatar.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { NO_LOOK, type Look } from '$lib/cosmetics/store.svelte';

  type Props = {
    name: string;
    avatar?: string | null;
    /** The small line under the name, e.g. "Twitch channel". */
    kind: string;
    /** Facts under it (counts, the active set). */
    meta?: Snippet;
    actions?: Snippet;
    /** The account's badge and paint, when known: the name is drawn in it. */
    look?: Look;
  };
  let { name: raw, avatar = null, kind, meta, actions, look = NO_LOOK }: Props = $props();
  const name = $derived(clean(raw));
</script>

<div class="card">
  <div class="top">
    <Avatar src={avatar} {name} size={56} />
    <div class="who">
      <p class="h"><PaintedName {name} paint={look.paint} badge={look.badge} badgeSize={20} /></p>
      <p class="kind">{kind}</p>
    </div>
  </div>
  {#if meta}<p class="meta">{@render meta()}</p>{/if}
  {#if actions}<div class="pc-actions">{@render actions()}</div>{/if}
</div>

<style>
  .card { display: flex; flex-direction: column; align-items: stretch; gap: 10px; }
  .top { display: flex; align-items: center; gap: 12px; }
  .who { min-width: 0; }
  .h { display: flex; min-width: 0; margin: 0; font-size: var(--t-block); font-weight: 600; line-height: 1.25; letter-spacing: -.02em; overflow-wrap: anywhere; }
  .kind { margin: 2px 0 0; font-size: var(--t-small); color: var(--muted); }
  .meta { margin: 0; font-size: var(--t-small); color: var(--muted); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
  .pc-actions { display: flex; flex-wrap: wrap; gap: 8px; }
</style>
