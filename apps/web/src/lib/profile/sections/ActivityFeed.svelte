<script lang="ts">
  // What happened on an account (7TV's activity feed): one row per change with an icon (added,
  // removed, renamed, paint or badge changed, …), a sentence with the emote's picture and the
  // names (paints drawn in the paint), who did it, and how long ago. Public. Pages back 30 at a time.
  import { onMount } from 'svelte';
  import Icon, { type IconName } from '$lib/components/Icon.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import State from '$lib/components/State.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { ready, src, srcset } from '$lib/emote';
  import { clean } from '$lib/format';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loadActivity, PAGE, relative, type ActivityItem, type ActivityKind } from '../activity';

  let { accountId }: { accountId: string } = $props();

  let items = $state<ActivityItem[]>([]);
  let first = $state(true);
  let busy = $state(false);
  let more = $state(false);
  let problem = $state('');
  let catalog = $state<Catalog | null>(null);

  async function next() {
    if (busy) return;
    busy = true;
    problem = '';
    try {
      const got = await loadActivity(accountId, items.at(-1)?.id ?? null);
      items = [...items, ...got];
      more = got.length === PAGE;
    } catch (e) {
      problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      busy = false;
      first = false;
    }
  }
  onMount(() => {
    next();
    loadCatalog().then((c) => (catalog = c)).catch(() => {});
  });

  const ICON: Record<ActivityKind, IconName> = {
    added: 'plus', removed: 'minus', renamed: 'pencil', active: 'zap', cosmetics: 'brush', uploaded: 'upload', created: 'folder', setRenamed: 'pencil',
  };
</script>

{#snippet emote(i: ActivityItem)}
  {#if i.emote}
    <strong><bdi>{clean(i.emote.name)}</bdi></strong>
    {#if ready(i.emote.picture)}<img class="pic" src={src(i.emote.picture, 1)} srcset={srcset(i.emote.picture)} alt="" loading="lazy" />{/if}
  {/if}
{/snippet}
{#snippet setName(i: ActivityItem)}<strong><bdi>{clean(i.set?.name ?? i.to ?? 'a set')}</bdi></strong>{/snippet}
{#snippet paint(id: string | null | undefined, name: string | null | undefined)}
  {@const p = id ? catalog?.paints.get(id) : null}
  {#if p}<strong><PaintedName name={p.name} paint={p} /></strong>{:else if name}<strong><bdi>{clean(name)}</bdi></strong>{:else}<strong>none</strong>{/if}
{/snippet}
{#snippet cosmetics(i: ActivityItem)}
  {#if i.paint && (i.paint.fromId !== i.paint.toId)}
    Changed the paint from {@render paint(i.paint.fromId, i.paint.fromName)} to {@render paint(i.paint.toId, i.paint.toName)}{#if i.badge && i.badge.fromId !== i.badge.toId}, and the badge to <strong><bdi>{clean(i.badge.toName ?? 'none')}</bdi></strong>{/if}
  {:else if i.badge}
    Changed the badge from <strong><bdi>{clean(i.badge.fromName ?? 'none')}</bdi></strong> to <strong><bdi>{clean(i.badge.toName ?? 'none')}</bdi></strong>
  {:else}Changed the cosmetics{/if}
{/snippet}

<PageHead title="Activity" line="Changes to the sets, emotes and cosmetics, newest first." />

{#if first}
  <State kind="loading" title="Loading activity…" />
{:else if problem && !items.length}
  <State kind="error" title="Activity didn’t load">
    {problem}
    {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={next}>Try again</button>{/snippet}
  </State>
{:else if !items.length}
  <State title="Nothing yet.">Changes to the sets, emotes and cosmetics show up here.</State>
{:else}
  <ol class="feed">
    {#each items as i (i.id)}
      <li class="item">
        <span class="ic" aria-hidden="true"><Icon name={ICON[i.kind]} size={18} /></span>
        <span class="body">
          <span class="line">
            {#if i.kind === 'added'}Added {@render emote(i)} to {@render setName(i)}
            {:else if i.kind === 'removed'}Removed {@render emote(i)} from {@render setName(i)}
            {:else if i.kind === 'renamed'}Renamed <strong><bdi>{clean(i.from)}</bdi></strong> to {@render emote(i)} in {@render setName(i)}
            {:else if i.kind === 'active'}Made {@render setName(i)} the active set{#if i.from}{' '}instead of <strong><bdi>{clean(i.from)}</bdi></strong>{/if}
            {:else if i.kind === 'cosmetics'}{@render cosmetics(i)}
            {:else if i.kind === 'uploaded'}Published {@render emote(i)}
            {:else if i.kind === 'created'}Made the set {@render setName(i)}
            {:else}Renamed the set <strong><bdi>{clean(i.from)}</bdi></strong> to <strong><bdi>{clean(i.to ?? i.set?.name)}</bdi></strong>{/if}
            {#if i.actor} by <a href="/users/{encodeURIComponent(i.actor.id)}"><bdi>{clean(i.actor.displayName)}</bdi></a>{/if}
          </span>
          <time datetime={i.at} title={new Date(i.at).toLocaleString()}>{relative(i.at)}</time>
        </span>
      </li>
    {/each}
  </ol>
  <div class="foot"><LoadMore hasMore={more} {busy} error={problem} total={items.length} onmore={next} /></div>
{/if}

<style>
  .feed { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border); }
  .item { display: flex; gap: 14px; padding: 14px 4px; border-bottom: 1px solid var(--border); }
  .ic { flex: none; display: grid; place-items: center; width: 24px; height: 28px; color: var(--muted); }
  .body { display: grid; gap: 2px; min-width: 0; }
  .line { font-size: var(--t-ui); line-height: 28px; overflow-wrap: anywhere; color: var(--muted); }
  .line strong { font-weight: 600; color: var(--text); }
  .line a { color: var(--text); font-weight: 500; text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .pic { display: inline-block; height: 28px; width: auto; max-width: 64px; margin: 0 2px; vertical-align: middle; }
  time { font-size: var(--t-fine); color: var(--dim); font-variant-numeric: tabular-nums; }
  .foot { padding: 20px 0 0; }
  @media (hover: hover) and (pointer: fine) { .line a:hover { text-decoration-color: currentColor; } }
</style>
