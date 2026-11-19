<script lang="ts">
  // The emote page's activity rows: an icon, one plain sentence about this emote (it is the page's
  // subject, so it isn't named again unless a set calls it something else), who did it, and the
  // date and time it happened (2026/11/19 14:05, how long ago on hover).
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, NO_LOOK, wantLooks } from '$lib/cosmetics/store.svelte';
  import { clean, dateTime } from '$lib/format';
  import { relative, type ActivityItem, type ActivityKind } from '$lib/profile/activity';

  /** `name`: the emote's own name, so a row only spells out a different one (an alias in a set). */
  let { items, name }: { items: ActivityItem[]; name: string } = $props();

  $effect(() => wantLooks(items.map((i) => i.actor?.id)));

  const ICON: Record<ActivityKind, IconName> = {
    added: 'plus', removed: 'minus', renamed: 'pencil', active: 'zap', cosmetics: 'brush', uploaded: 'upload', created: 'folder', setRenamed: 'pencil',
  };
  const alias = (i: ActivityItem) => (i.emote && clean(i.emote.name) !== clean(name) ? clean(i.emote.name) : null);
</script>

{#snippet setName(i: ActivityItem)}
  {#if i.set}<a href="/sets/{i.set.id}"><bdi>{clean(i.set.name)}</bdi></a>
  {:else}<strong><bdi>{clean(i.to ?? 'a set')}</bdi></strong>{/if}
{/snippet}

<ol class="feed">
  {#each items as i (i.id)}
    {@const as = alias(i)}
    <li class="item {i.kind}">
      <span class="ic" aria-hidden="true"><Icon name={ICON[i.kind]} size={16} /></span>
      <span class="line">
        {#if i.kind === 'uploaded'}Uploaded
        {:else if i.kind === 'added'}Added to {@render setName(i)}{#if as}{' as '}<strong><bdi>{as}</bdi></strong>{/if}
        {:else if i.kind === 'removed'}Removed from {@render setName(i)}
        {:else if i.kind === 'renamed'}Renamed{#if i.from}{' from '}<strong><bdi>{clean(i.from)}</bdi></strong>{/if}{' to '}<strong><bdi>{as ?? clean(name)}</bdi></strong> in {@render setName(i)}
        {:else}Changed in {@render setName(i)}{/if}
        {#if i.actor}{' by '}<a href="/users/{encodeURIComponent(i.actor.id)}"><PaintedName name={i.actor.displayName} paint={(looks[i.actor.id] ?? NO_LOOK).paint} badge={(looks[i.actor.id] ?? NO_LOOK).badge} badgeSize={16} /></a>{/if}
      </span>
      <time datetime={i.at} title={relative(i.at)}>{dateTime(i.at)}</time>
    </li>
  {/each}
</ol>

<style>
  .feed { list-style: none; margin: 0; padding: 0; }
  .item { display: grid; grid-template-columns: 28px minmax(0, 1fr) auto; align-items: center; column-gap: 12px; min-height: 52px; padding: 10px 4px;
    border-top: 1px solid var(--border); }
  .item:first-child { border-top: 0; }
  .ic { display: grid; place-items: center; width: 28px; height: 28px; border: 1px solid var(--border); border-radius: var(--radius-btn); color: var(--muted); }
  .uploaded .ic, .added .ic { color: var(--accent-text); }
  .line { font-size: var(--t-ui); line-height: 1.45; overflow-wrap: anywhere; color: var(--muted); }
  .line strong { font-weight: 600; color: var(--text); }
  .line a { color: var(--text); font-weight: 500; text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  time { font-size: var(--t-small); color: var(--muted); font-variant-numeric: tabular-nums; white-space: nowrap; }
  @media (hover: hover) and (pointer: fine) { .line a:hover { text-decoration-color: currentColor; } }
  /* Phones: the date and time go under the sentence. */
  @media (max-width: 560px) {
    .item { grid-template-columns: 28px minmax(0, 1fr); align-items: start; row-gap: 2px; }
    .ic { grid-row: span 2; }
    time { font-size: var(--t-fine); }
  }
</style>
