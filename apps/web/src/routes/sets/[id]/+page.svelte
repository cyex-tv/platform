<script lang="ts">
  // A set, for anyone with the link (ADR 0017): listed and unlisted sets open signed out, a
  // private one answers like a missing one. Name, owner, how full, search inside the set, grid or
  // list, and the actions: clone it into your own sets, copy the link, edit it when it is yours.
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import EmoteTable from '$lib/components/browse/EmoteTable.svelte';
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { useCopied } from '$lib/clipboard.svelte';
  import { clean, count } from '$lib/format';
  import { PUBLIC_SET } from '$lib/queries';
  import { capacityText, filterMembers, fillPercent } from '$lib/sets/public';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import type { PublicSetQuery } from '$lib/gql/graphql';
  import CloneDialog from './CloneDialog.svelte';

  type PageSet = NonNullable<PublicSetQuery['set']>;
  let { data } = $props();

  // The server can't see a private set (no session cookie), so when it found nothing the browser
  // asks once more: its owner and editors get the page.
  let late = $state<{ id: string; set: PageSet | null } | null>(null);
  $effect(() => {
    if (data.set) return;
    const id = data.id;
    let gone = false;
    (async () => {
      let set: PageSet | null = null;
      try {
        set = (await gql(PUBLIC_SET, { id })).set ?? null;
      } catch {
        set = null;
      }
      if (!gone) late = { id, set };
    })();
    return () => (gone = true);
  });
  const asked = $derived(data.set ? null : late?.id === data.id ? late : undefined);
  const s = $derived(data.set ?? asked?.set ?? null);

  const name = $derived(s ? clean(s.name) : '');
  const owner = $derived(s?.owner ?? null);
  const mine = $derived(!!s && !!session.me && session.me.id === s.owner?.id);
  const used = $derived(s ? Math.max(s.slotsUsed, s.emotes.length) : 0);
  const pct = $derived(s ? fillPercent(used, s.capacity) : 0);
  const unlisted = $derived(s?.visibility === 'UNLISTED');
  const description = $derived(
    s
      ? `${name}, an emote set with ${count(s.emotes.length)} ${s.emotes.length === 1 ? 'emote' : 'emotes'} on cyex.tv${owner ? `, by ${clean(owner.displayName)}` : ''}.`
      : '',
  );

  const members = $derived(s?.emotes ?? []);
  let query = $state('');
  const needle = $derived(query.trim());
  const shown = $derived(filterMembers(members, query));
  // Tiles show the name in the set (and the emote's own name stays in its page); rows say when it
  // was added to the set.
  const cards = $derived(shown.map((m) => ({ ...m.emote, shownName: m.name !== m.emote.name ? m.name : undefined })));
  const rows = $derived(shown.map((m) => ({ ...m.emote, name: m.name, createdAt: m.addedAt })));

  type View = 'grid' | 'list';
  let view = $state<View>('grid');
  $effect(() => {
    try {
      const v = localStorage.getItem('cyex.set.view');
      if (v === 'grid' || v === 'list') view = v;
    } catch {
      // Storage blocked: the grid it is.
    }
  });
  function pick(v: View) {
    view = v;
    try {
      localStorage.setItem('cyex.set.view', v);
    } catch {
      // Not remembered, still shown.
    }
  }

  const link = useCopied();
  let dlg = $state<CloneDialog>();
  const back = $derived(`/signin?back=${encodeURIComponent(page.url.pathname)}`);
</script>

<svelte:head>
  {#if s}
    <title>{name} - cyex.tv</title>
    <meta name="description" content={description} />
    <meta property="og:type" content="website" />
    <meta property="og:site_name" content="cyex.tv" />
    <meta property="og:title" content="{name} - cyex.tv" />
    <meta property="og:description" content={description} />
    <meta property="og:url" content={page.url.origin + page.url.pathname} />
    {#if s.visibility !== 'LISTED'}<meta name="robots" content="noindex" />{/if}
  {:else}
    <title>No set here - cyex.tv</title>
    <meta name="robots" content="noindex" />
  {/if}
</svelte:head>

{#if s}
  {#key s.id}
    <ColumnLayout top>
      <section class="set" aria-labelledby="set-h">
        <header class="head">
          <div class="text">
            <h1 class="page-title" id="set-h"><bdi>{name}</bdi></h1>
            <div class="meta">
              {#if owner}
                <a class="owner" href="/users/{owner.id}">
                  <Avatar src={owner.avatar} name={owner.displayName} size={24} />
                  <span><bdi>{clean(owner.displayName)}</bdi></span>
                </a>
              {/if}
              {#if unlisted}
                <span class="mark" title="Only people with the link can find it"><Icon name="link" size={15} />Unlisted</span>
              {/if}
              <span class="cap">
                <span class="n" aria-hidden="true">{capacityText(used, s.capacity)}</span>
                <span class="sr-only">{count(used)} of {count(s.capacity)} places used</span>
                <span class="bar" role="meter" aria-label="How full" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct}>
                  <span class="fill" class:full={pct >= 90} style:width="{pct}%"></span>
                </span>
              </span>
            </div>
          </div>
          <div class="actions">
            {#if session.me}
              <button class="ui-btn primary" type="button" onclick={() => dlg?.open()}><Icon name="layers" size={16} />Clone to my sets</button>
            {:else if session.known}
              <a class="ui-btn primary" href={back}><Icon name="layers" size={16} />Sign in to clone</a>
            {:else}
              <button class="ui-btn primary" type="button" disabled><Icon name="layers" size={16} />Clone to my sets</button>
            {/if}
            <button class="ui-btn secondary" type="button" onclick={() => link.copy(page.url.origin + page.url.pathname)}>
              <Icon name={link.copied ? 'check' : 'copy'} size={16} />
              <span class="labels">
                <span class:off={link.state !== 'idle'}>Copy link</span>
                <span class:off={!link.copied} aria-hidden="true">Link copied</span>
                <span class:off={!link.failed} aria-hidden="true">Couldn’t copy</span>
              </span>
            </button>
            {#if mine}<a class="ui-btn secondary" href="/me/sets/{s.id}"><Icon name="pencil" size={16} />Edit set</a>{/if}
            <span class="sr-only" role="status">{link.copied ? 'Link copied' : link.failed ? 'Couldn’t copy' : ''}</span>
          </div>
        </header>

        {#if members.length}
        <div class="tools">
          <label class="find">
            <span class="sr-only">Search this set</span>
            <Icon name="search" size={16} />
            <input class="input" type="search" placeholder="Search this set" bind:value={query} autocomplete="off" />
          </label>
          <p class="total" aria-live="polite">
            {needle ? `${count(shown.length)} of ${count(members.length)}` : count(members.length)}
            {members.length === 1 ? 'emote' : 'emotes'}
          </p>
          <Segmented label="View" size="s" options={[{ value: 'grid', label: 'Grid', icon: 'grid' }, { value: 'list', label: 'List', icon: 'list' }]} value={view} onchange={pick} />
        </div>
        {/if}

        {#if members.length === 0}
          <State title="No emotes in this set yet.">
            {#if mine}Add some from the set’s editor.{:else}Nothing has been added to it yet.{/if}
            {#snippet actions()}
              {#if mine}<a class="ui-btn primary" href="/me/sets/{s.id}">Add emotes</a>{:else}<a class="ui-btn secondary" href="/search">Browse emotes</a>{/if}
            {/snippet}
          </State>
        {:else if shown.length === 0}
          <State title="Nothing matches “{needle}”.">
            No emote in this set has that in its name.
            {#snippet actions()}
              <button class="ui-btn secondary" type="button" onclick={() => (query = '')}>Clear search</button>
            {/snippet}
          </State>
        {:else if view === 'list'}
          <EmoteTable items={rows} label="Emotes in {name}" eager={12} />
        {:else}
          <EmoteGrid items={cards} eager={12} label="Emotes in {name}" />
        {/if}
      </section>
    </ColumnLayout>
    <CloneDialog bind:this={dlg} id={s.id} name={s.name} />
  {/key}
{:else if asked === undefined}
  <ColumnLayout top>
    <div class="missing"><State kind="loading" title="Looking for this set…" /></div>
  </ColumnLayout>
{:else}
  <ColumnLayout top>
    <div class="missing">
      <a class="back" href="/search"><Icon name="arrow-left" />All emotes</a>
      <State level={1} role="status" title="No set here.">
        It may be private, its owner may have deleted it, or the link is missing a character.
        {#snippet actions()}
          <a class="ui-btn primary" href="/search">Browse emotes</a>
          <a class="ui-btn secondary" href="/">Front page</a>
        {/snippet}
      </State>
    </div>
  </ColumnLayout>
{/if}

<style>
  .set { padding-bottom: 40px; }
  .head { display: flex; flex-wrap: wrap; align-items: flex-start; justify-content: space-between; gap: 12px 24px; margin-bottom: 20px; }
  .text { flex: 1 1 320px; min-width: 0; }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 20px; margin-top: 10px; font-size: var(--t-small); color: var(--muted); }
  .owner { display: inline-flex; align-items: center; gap: 8px; min-width: 0; max-width: 100%; min-height: 32px; color: var(--text); font-weight: 500; }
  .owner span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .owner :global(.av) { flex: none; }
  @media (pointer: coarse) { .owner { min-height: 44px; } }
  @media (hover: hover) and (pointer: fine) { .owner:hover span { text-decoration: underline; text-underline-offset: 3px; } }
  .mark { display: inline-flex; align-items: center; gap: 6px; }
  .mark :global(svg) { color: var(--accent-text); }
  .cap { display: inline-flex; align-items: center; gap: 10px; }
  .n { font-variant-numeric: tabular-nums; }
  .bar { display: block; width: 96px; height: 4px; border-radius: 2px; background: var(--elevated-2); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
  .fill.full { background: var(--yellow); }
  @media (forced-colors: active) { .fill { background: Highlight; } }
  .actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .labels { display: inline-grid; }
  .labels > span { grid-area: 1 / 1; text-align: left; }
  .off { visibility: hidden; }
  .tools { display: flex; flex-wrap: wrap; align-items: center; gap: 12px 16px; margin-bottom: 20px; }
  .tools :global([role='radiogroup']) { margin-left: auto; }
  .total { margin: 0; font-size: var(--t-small); color: var(--muted); font-variant-numeric: tabular-nums; }
  .find { position: relative; display: block; flex: 0 1 280px; min-width: 0; }
  .find :global(svg) { position: absolute; left: 12px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 40px; padding-left: 36px; }
  @media (max-width: 520px) { .find { flex-basis: 100%; } .tools :global([role='radiogroup']) { margin-left: 0; } }
  .missing { display: grid; gap: 16px; min-height: 50svh; align-content: start; padding-bottom: 40px; }
  .back { justify-self: start; display: inline-flex; align-items: center; gap: 8px; min-height: 40px; color: var(--muted); font-size: var(--t-small); }
  @media (hover: hover) and (pointer: fine) { .back:hover { color: var(--text); } }
</style>
