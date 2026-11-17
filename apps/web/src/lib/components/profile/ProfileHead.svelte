<script lang="ts">
  // The top of a channel or creator page: avatar, name, a quiet line under it, actions on the right.
  import type { Snippet } from 'svelte';
  import { clean } from '$lib/format';
  import Avatar from '../Avatar.svelte';

  type Props = {
    name: string;
    avatar?: string | null;
    /** The small line above the name, e.g. "Twitch channel". */
    kind: string;
    /** Facts under the name (counts, handles). */
    meta?: Snippet;
    actions?: Snippet;
  };
  let { name: raw, avatar = null, kind, meta, actions }: Props = $props();
  const name = $derived(clean(raw));
</script>

<header class="head">
  <Avatar src={avatar} {name} size={80} />
  <div class="who">
    <p class="kind">{kind}</p>
    <h1>{name}</h1>
    {#if meta}<p class="meta">{@render meta()}</p>{/if}
  </div>
  {#if actions}<div class="head-actions">{@render actions()}</div>{/if}
</header>

<style>
  .head { display: flex; flex-wrap: wrap; align-items: center; gap: 20px 24px; padding: clamp(32px, 5vw, 64px) 0 32px; }
  .who { flex: 1 1 240px; min-width: 0; }
  .kind { margin: 0 0 6px; font-size: var(--t-small); font-weight: 500; color: var(--muted); }
  h1 { font-size: var(--t-h3-sub); line-height: 1.1; letter-spacing: -.03em; overflow-wrap: anywhere; text-wrap: balance; }
  .meta { margin: 8px 0 0; font-size: var(--t-ui); color: var(--muted); }
  .head-actions { margin-left: auto; }
  @media (max-width: 520px) { .head { gap: 16px; } .head-actions { margin-left: 0; width: 100%; } }
</style>
