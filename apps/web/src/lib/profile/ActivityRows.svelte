<script lang="ts">
  // The activity rows of the profile's Activity section: an icon, a sentence with the emote's
  // picture and the names (paints drawn in the paint, sets and people as links), and when
  // (ActivityWhen). On an account's own feed (`subject`) a line the account made itself doesn't
  // repeat its name ("by …" only for its editors). The emote page has its own, shorter rows
  // (EmoteActivityRows); the icons and the "by" rule are shared (activity-view.ts).
  import Icon from '$lib/components/Icon.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, NO_LOOK, wantLooks, type Catalog } from '$lib/cosmetics/store.svelte';
  import EmotePicture from '$lib/components/EmotePicture.svelte';
  import { ready } from '$lib/emote';
  import { clean } from '$lib/format';
  import ActivityWhen from './ActivityWhen.svelte';
  import type { ActivityItem } from './activity';
  import { ICON, namesActor } from './activity-view';

  /** `pictures: false` drops the emote's picture from each row (the emote's own page already shows it). */
  type Props = { items: ActivityItem[]; catalog?: Catalog | null; pictures?: boolean; subject?: string | null };
  let { items, catalog = null, pictures = true, subject = null }: Props = $props();

  // Who did each change, in their paint and badge: the shared cosmetics cache, one batch per list.
  $effect(() => wantLooks(items.flatMap((i) => [i.actor?.id, i.editor?.id])));
</script>

{#snippet emote(i: ActivityItem)}
  {#if i.emote}
    <strong><bdi>{clean(i.emote.name)}</bdi></strong>
    {#if pictures && ready(i.emote.picture)}<EmotePicture emote={i.emote.picture} class="pic" />{/if}
  {/if}
{/snippet}
{#snippet setName(i: ActivityItem)}
  {#if i.set}<a href="/sets/{i.set.id}"><strong><bdi>{clean(i.set.name)}</bdi></strong></a>
  {:else}<strong><bdi>{clean(i.to ?? 'a set')}</bdi></strong>{/if}
{/snippet}
{#snippet paint(id: string | null | undefined, name: string | null | undefined)}
  {@const p = id ? catalog?.paints.get(id) : null}
  {#if p}<strong><PaintedName name={p.name} paint={p} /></strong>{:else if name}<strong><bdi>{clean(name)}</bdi></strong>{:else}<strong>none</strong>{/if}
{/snippet}
{#snippet person(p: { id: string; displayName: string })}
  <a href="/users/{encodeURIComponent(p.id)}"><PaintedName name={p.displayName} paint={(looks[p.id] ?? NO_LOOK).paint} badge={(looks[p.id] ?? NO_LOOK).badge} badgeSize={16} /></a>
{/snippet}
{#snippet cosmetics(i: ActivityItem)}
  {#if i.paint && (i.paint.fromId !== i.paint.toId)}
    Changed the paint from {@render paint(i.paint.fromId, i.paint.fromName)} to {@render paint(i.paint.toId, i.paint.toName)}{#if i.badge && i.badge.fromId !== i.badge.toId}, and the badge to <strong><bdi>{clean(i.badge.toName ?? 'none')}</bdi></strong>{/if}
  {:else if i.badge}
    Changed the badge from <strong><bdi>{clean(i.badge.fromName ?? 'none')}</bdi></strong> to <strong><bdi>{clean(i.badge.toName ?? 'none')}</bdi></strong>
  {:else}Changed the cosmetics{/if}
{/snippet}

<ol class="feed">
  {#each items as i (i.id)}
    <li class="item">
      <span class="ic" aria-hidden="true"><Icon name={ICON[i.kind]} size={18} /></span>
      <span class="body">
        <span class="line">
          {#if i.kind === 'added'}Added {@render emote(i)} to {@render setName(i)}
          {:else if i.kind === 'removed'}Removed {@render emote(i)} from {@render setName(i)}
          {:else if i.kind === 'renamed'}Renamed <strong><bdi>{clean(i.from)}</bdi></strong> to {@render emote(i)} in {@render setName(i)}
          {:else if i.kind === 'active'}Made {@render setName(i)} the active set{#if i.from}{' instead of '}<strong><bdi>{clean(i.from)}</bdi></strong>{/if}
          {:else if i.kind === 'cosmetics'}{@render cosmetics(i)}
          {:else if i.kind === 'uploaded'}Published {@render emote(i)}
          {:else if i.kind === 'created'}Made the set {@render setName(i)}
          {:else if i.kind === 'editorAdded'}{#if i.editor}Added {@render person(i.editor)} as an editor{:else}Added an editor{/if}
          {:else if i.kind === 'editorRemoved'}{#if i.editor && i.actor?.id === i.editor.id}{@render person(i.editor)} stopped being an editor{:else if i.editor}Removed {@render person(i.editor)} as an editor{:else}Removed an editor{/if}
          {:else}Renamed the set <strong><bdi>{clean(i.from)}</bdi></strong> to <strong><bdi>{clean(i.to ?? i.set?.name)}</bdi></strong>{/if}
          {#if i.actor && i.actor.id !== subject && namesActor(i, null)} by {@render person(i.actor)}{/if}
        </span>
        <span class="when"><ActivityWhen at={i.at} /></span>
      </span>
    </li>
  {/each}
</ol>

<style>
  .feed { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border); }
  .item { display: flex; gap: 14px; padding: 14px 4px; border-bottom: 1px solid var(--border); }
  .ic { flex: none; display: grid; place-items: center; width: 24px; height: 28px; color: var(--muted); }
  .body { display: grid; gap: 2px; min-width: 0; }
  .line { font-size: var(--t-ui); line-height: 28px; overflow-wrap: anywhere; color: var(--muted); }
  .line strong { font-weight: 600; color: var(--text); }
  .line a { color: var(--text); font-weight: 500; text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  li :global(.pic) { display: inline-block; height: 28px; width: auto; max-width: 64px; margin: 0 2px; vertical-align: middle; }
  .when { font-size: var(--t-fine); color: var(--muted); }
  @media (hover: hover) and (pointer: fine) { .line a:hover { text-decoration-color: currentColor; } }
</style>