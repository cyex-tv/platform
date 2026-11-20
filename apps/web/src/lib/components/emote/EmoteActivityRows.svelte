<script lang="ts">
  // The emote page's activity rows: an icon, one plain sentence about this emote (it is the page's
  // subject, so it isn't named again unless a set calls it something else), who did it (unless that
  // is you), and when (ActivityWhen). The sentence switch is exhaustive: a new ActivityKind is a
  // type error here until it has a sentence. The icons and the "by" rule are shared
  // (activity-view.ts).
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, NO_LOOK, wantLooks } from '$lib/cosmetics/store.svelte';
  import { clean } from '$lib/format';
  import ActivityWhen from '$lib/profile/ActivityWhen.svelte';
  import type { ActivityItem } from '$lib/profile/activity';
  import { ICON, namesActor } from '$lib/profile/activity-view';
  import { session } from '$lib/tool/session.svelte';

  /** `name`: the emote's own name, so a row only spells out a different one (an alias in a set). */
  let { items, name }: { items: ActivityItem[]; name: string } = $props();

  $effect(() => wantLooks(items.map((i) => i.actor?.id)));

  /** Fails to compile when a new `ActivityKind` has no sentence above. */
  const unreachable = (kind: never): string => String(kind);
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
        {:else if i.kind === 'active' || i.kind === 'cosmetics' || i.kind === 'created' || i.kind === 'setRenamed' || i.kind === 'editorAdded' || i.kind === 'editorRemoved'}Changed in {@render setName(i)}
        {:else}{unreachable(i.kind)}{/if}
        {#if i.actor && namesActor(i, session.me?.id)}{' by '}<a href="/users/{encodeURIComponent(i.actor.id)}"><PaintedName name={i.actor.displayName} paint={(looks[i.actor.id] ?? NO_LOOK).paint} badge={(looks[i.actor.id] ?? NO_LOOK).badge} badgeSize={16} /></a>{/if}
      </span>
      <span class="when"><ActivityWhen at={i.at} /></span>
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
  .when { font-size: var(--t-small); color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .line a:hover { text-decoration-color: currentColor; } }
  /* Phones: the date and time go under the sentence. */
  @media (max-width: 560px) {
    .item { grid-template-columns: 28px minmax(0, 1fr); align-items: start; row-gap: 2px; }
    .ic { grid-row: span 2; }
    .when { font-size: var(--t-fine); }
  }
</style>
