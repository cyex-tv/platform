<script lang="ts">
  // The identity block of a creator or channel page: avatar, name, what it is, a line of facts and
  // the page's actions. It heads the SideRail on wide windows and sits above the content on phones
  // (the rail is a tab strip there), so the page always says whose it is.
  import type { Snippet } from 'svelte';
  import { clean } from '$lib/format';
  import Avatar from '../Avatar.svelte';

  type Props = {
    name: string;
    avatar?: string | null;
    /** The small line under the name, e.g. "Twitch channel". */
    kind: string;
    /** Facts under it (counts, the active set). */
    meta?: Snippet;
    actions?: Snippet;
    /** Which heading it is: the page's h1 (default), or none when another copy is the h1. */
    heading?: boolean;
  };
  let { name: raw, avatar = null, kind, meta, actions, heading = true }: Props = $props();
  const name = $derived(clean(raw));
</script>

<div class="card">
  <div class="top">
    <Avatar src={avatar} {name} size={56} />
    <div class="who">
      {#if heading}<h1><bdi>{name}</bdi></h1>{:else}<p class="h"><bdi>{name}</bdi></p>{/if}
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
  h1, .h { margin: 0; font-size: var(--t-block); font-weight: 600; line-height: 1.25; letter-spacing: -.02em; overflow-wrap: anywhere; }
  .kind { margin: 2px 0 0; font-size: var(--t-small); color: var(--muted); }
  .meta { margin: 0; font-size: var(--t-small); color: var(--muted); font-variant-numeric: tabular-nums; overflow-wrap: anywhere; }
  .pc-actions { display: flex; flex-wrap: wrap; gap: 8px; }
</style>
