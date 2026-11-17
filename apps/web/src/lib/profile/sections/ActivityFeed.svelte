<script lang="ts">
  // What happened on an account (7TV's activity feed): one row per change with an icon (added,
  // removed, renamed, paint changed), a sentence with the emote's picture and the names, and the
  // time it happened. Reads `ActivityItem`s from the adapter in ../activity.ts.
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { ready, src, srcset } from '$lib/emote';
  import { clean } from '$lib/format';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { loadActivity, relative, type ActivityItem, type ActivityKind } from '../activity';

  let { accountId }: { accountId: string } = $props();

  const L = loader(({ fetch }) => loadActivity(accountId, fetch));
  let catalog = $state<Catalog | null>(null);
  $effect(() => {
    loadCatalog().then((c) => (catalog = c)).catch(() => {});
  });

  const ICON: Record<ActivityKind, IconName> = { added: 'plus', removed: 'minus', renamed: 'pencil', paint: 'brush' };
</script>

{#snippet emote(i: ActivityItem)}
  {#if i.emote}
    <strong><bdi>{clean(i.emote.name)}</bdi></strong>
    {#if i.emote.picture && ready(i.emote.picture)}<img class="pic" src={src(i.emote.picture, 1)} srcset={srcset(i.emote.picture)} alt="" loading="lazy" />{/if}
  {/if}
{/snippet}
{#snippet paint(id: string | null | undefined)}
  {@const p = id ? catalog?.paints.get(id) : null}
  {#if p}<strong><PaintedName name={p.name} paint={p} /></strong>{:else}<strong>{id ? 'a paint' : 'none'}</strong>{/if}
{/snippet}

<PageHead title="Activity" line="Changes to the sets and cosmetics, newest first." />

<LoadGate source={L} label="Loading activity…">
  {#snippet children(items)}
    {#if !items.length}
      <State title="Nothing yet.">Changes to the sets and the paint show up here.</State>
    {:else}
      <ol class="feed">
        {#each items as i (i.id)}
          <li class="item">
            <span class="ic" aria-hidden="true"><Icon name={ICON[i.kind]} size={18} /></span>
            <span class="body">
              <span class="line">
                {#if i.kind === 'added'}Added {@render emote(i)} to <strong><bdi>{clean(i.set?.name)}</bdi></strong>
                {:else if i.kind === 'removed'}Removed {@render emote(i)} from <strong><bdi>{clean(i.set?.name)}</bdi></strong>
                {:else if i.kind === 'renamed'}Renamed <strong><bdi>{clean(i.from)}</bdi></strong> to {@render emote(i)} in <strong><bdi>{clean(i.set?.name)}</bdi></strong>
                {:else}Changed the paint from {@render paint(i.paintFrom)} to {@render paint(i.paintTo)}{/if}
                {#if i.actor} by <a href="/users/{encodeURIComponent(i.actor.id)}"><bdi>{clean(i.actor.displayName)}</bdi></a>{/if}
              </span>
              <time datetime={i.at}>{relative(i.at)}</time>
            </span>
          </li>
        {/each}
      </ol>
    {/if}
  {/snippet}
</LoadGate>

<style>
  .feed { list-style: none; margin: 0; padding: 0; }
  .item { display: flex; gap: 14px; padding: 14px 4px; border-bottom: 1px solid var(--border); }
  .ic { flex: none; display: grid; place-items: center; width: 24px; height: 24px; color: var(--muted); }
  .body { display: grid; gap: 4px; min-width: 0; }
  .line { font-size: var(--t-ui); line-height: 1.6; overflow-wrap: anywhere; }
  .line strong { font-weight: 600; }
  .line a { color: var(--accent-text); }
  .pic { display: inline-block; height: 28px; width: auto; max-width: 64px; margin: 0 2px; vertical-align: middle; }
  time { font-size: var(--t-fine); color: var(--dim); }
</style>
