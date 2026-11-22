<script lang="ts">
  // A set, for anyone with the link (ADR 0017): listed and unlisted sets open signed out, a
  // private one answers like a missing one. The Inspector shape (redesign, 2026/11/21): the name,
  // owner and size head the left column, then search inside the set and the emotes in the browse
  // grid (BrowseGrid: the same tiles, tile menu, Select mode with its bar, + to add to the set
  // you are adding to, drag to a set). On the right a panel: the actions (clone it into your own
  // sets, copy the link, edit it when it is yours), its facts and its editors (for whoever may see
  // them: the owner, its editors, staff). Under 1100px the actions sit in the head and the rest of
  // the panel folds above the grid. The ⋯ next to the name, or a right-click on it, opens the set
  // menu (SetActions): rename in place, copy link, set active, capacity, editor, delete.
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import State from '$lib/components/State.svelte';
  import CopyLink from '$lib/components/CopyLink.svelte';
  import BrowseGrid from '$lib/components/browse/BrowseGrid.svelte';
  import SelectBar from '$lib/components/browse/SelectBar.svelte';
  import SelectSwitch from '$lib/components/browse/SelectSwitch.svelte';
  import { all, CAP, EMPTY, toggle, type Selection, type Step } from '$lib/components/browse/selection';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import { clean, count, date } from '$lib/format';
  import { legacyRedirect } from '$lib/ids';
  import { PUBLIC_SET, SET_EDITORS } from '$lib/queries';
  import Capacity from '$lib/components/Capacity.svelte';
  import FactRows from '$lib/components/FactRows.svelte';
  import { filterMembers, fillPercent } from '$lib/sets/public';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import InspectorLayout from '$lib/shell/InspectorLayout.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import type { PublicSetQuery, SetEditorsQuery } from '$lib/gql/graphql';
  import CloneDialog from './CloneDialog.svelte';
  import InlineRename from '$lib/sets/InlineRename.svelte';
  import SetActions, { type SetChange } from '$lib/sets/SetActions.svelte';
  import { sectionHref } from '$lib/profile/rows';
  import { forgetProfile } from '$lib/profile/data.svelte';

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
      if (gone) return;
      late = { id, set };
      // Found only now (the server could not see it): an old ULID link still ends on the snowflake address.
      const moved = legacyRedirect('sets', id, set?.id, page.url.search);
      if (moved) void goto(moved, { replaceState: true });
    })();
    return () => (gone = true);
  });
  const asked = $derived(data.set ? null : late?.id === data.id ? late : undefined);
  const s = $derived(data.set ?? asked?.set ?? null);

  // What the set menu changed here, until the page is read again.
  let edited = $state<{ id: string; name?: string; capacity?: number } | null>(null);
  const local = $derived(edited && edited.id === s?.id ? edited : null);
  const rawName = $derived(local?.name ?? s?.name ?? '');
  const capacity = $derived(local?.capacity ?? s?.capacity ?? 0);
  const name = $derived(s ? clean(rawName) : '');
  const owner = $derived(s?.owner ?? null);
  const mine = $derived(!!s && !!session.me && session.me.id === s.owner?.id);
  const used = $derived(s ? Math.max(s.slotsUsed, s.emotes.length) : 0);
  const pct = $derived(s ? fillPercent(used, capacity) : 0);
  // Every set is public; staff can still take one off the listings (moderation).
  const unlisted = $derived(s?.visibility === 'UNLISTED');
  const KIND: Record<string, string> = { CHANNEL: 'Channel set', PERSONAL: 'Personal set', GLOBAL: 'Global set' };

  const members = $derived(s?.emotes ?? []);
  let query = $state('');
  const needle = $derived(query.trim());
  const shown = $derived(filterMembers(members, query));
  // Tiles show the name in the set (and the emote's own name stays in its page). A set can hold
  // a thousand emotes: the page draws them a step at a time (the search above still looks
  // through all of them), so a big set doesn't put a thousand tiles in at once.
  const STEP = 120;
  let limit = $state(STEP);
  $effect(() => {
    void needle;
    limit = STEP;
  });
  const drawn = $derived(shown.slice(0, limit));
  const cards = $derived(drawn.map((m) => ({ ...m.emote, shownName: m.name !== m.emote.name ? m.name : undefined })));

  // ---- Its editors: the API answers the owner, the set's editors and staff; anyone else sees no block.
  let editors = $state<{ id: string; list: SetEditorsQuery['setEditors'] } | null>(null);
  $effect(() => {
    const id = s?.id;
    if (!id || !session.me) return;
    if (!mine && !can('sets.edit_any')) return;
    let gone = false;
    gql(SET_EDITORS, { id })
      .then((r) => {
        if (!gone) editors = { id, list: r.setEditors };
      })
      .catch(() => {});
    return () => (gone = true);
  });
  const editorList = $derived(editors && editors.id === s?.id ? editors.list : null);
  $effect(() => {
    if (editorList?.length) wantLooks(editorList.slice(0, 10).map((e) => e.id));
  });

  // ---- Select mode (the browse grid's): picked ids in the order picked, up to CAP.
  let selecting = $state(false);
  let sel = $state.raw<Selection>(EMPTY);
  const order = $derived(cards.map((e) => e.id));
  let note = $state('');
  let noteTimer: ReturnType<typeof setTimeout> | undefined;
  function step(x: Step) {
    if (x.sel !== sel && !x.refused) note = '';
    sel = x.sel;
    if (x.refused) {
      note = `That’s ${CAP}, the most at once. Add these first, then pick more.`;
      clearTimeout(noteTimer);
      noteTimer = setTimeout(() => (note = ''), 4000);
    }
  }
  function setSelecting(on: boolean, id?: string) {
    selecting = on;
    if (!on) sel = EMPTY;
    else if (id && !sel.ids.includes(id)) step(toggle(sel, id));
  }
  function unpick(done: string[]) {
    note = '';
    sel = { ...sel, ids: sel.ids.filter((id) => !done.includes(id)) };
    return sel.ids;
  }
  // Escape clears the picks (then leaves Select mode); Ctrl/⌘+A picks what is shown.
  function onkeydown(e: KeyboardEvent) {
    if (!selecting || e.defaultPrevented) return;
    const t = e.target as HTMLElement;
    if (t.closest('input, textarea, select, [contenteditable="true"], [role="menu"], [role="dialog"]')) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      note = '';
      if (sel.ids.length) sel = EMPTY;
      else setSelecting(false);
    } else if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'a') {
      e.preventDefault();
      step(all(sel, order));
    }
  }

  let dlg = $state<CloneDialog>();
  let acts = $state<SetActions>();
  let renaming = $state(false);
  let menuButton = $state<HTMLButtonElement>();
  const menuSet = $derived(s ? { id: s.id, name: rawName, kind: s.kind, ownerId: s.owner?.id ?? null, capacity } : null);
  function changed(c: SetChange) {
    if (!s) return;
    forgetProfile('library:');
    if (c.kind === 'capacity' && c.capacity != null) edited = { ...(local ?? { id: s.id }), capacity: c.capacity };
    else if (c.kind === 'deleted') void goto(sectionHref(s.owner?.id ?? session.me?.id ?? '', 'sets'));
  }
  function renamed(next: string | null) {
    renaming = false;
    if (next && s) {
      edited = { ...(local ?? { id: s.id }), name: next };
      forgetProfile('library:');
    }
    menuButton?.focus({ preventScroll: true });
  }
  function menuAt(e: MouseEvent) {
    if (!menuSet) return;
    e.preventDefault();
    acts?.open(menuSet, e);
  }
  const back = $derived(`/signin?back=${encodeURIComponent(page.url.pathname)}`);
  const signin = $derived(`/signin?back=${encodeURIComponent(page.url.pathname + page.url.search)}`);
</script>

<svelte:window {onkeydown} />

<svelte:head>
  {#if s}
    <title>{name} - cyex.tv</title>
  {:else}
    <title>No set here - cyex.tv</title>
  {/if}
</svelte:head>

{#snippet clone()}
  {#if session.me}
    <button class="btn btn-s btn-primary" type="button" onclick={() => dlg?.open()}><Icon name="layers" size={16} />Clone to my sets</button>
  {:else if session.known}
    <a class="btn btn-s btn-primary" href={back}><Icon name="layers" size={16} />Sign in to clone</a>
  {:else}
    <button class="btn btn-s btn-primary" type="button" disabled><Icon name="layers" size={16} />Clone to my sets</button>
  {/if}
{/snippet}

{#snippet edit()}
  {#if mine && s}<a class="btn btn-s" href="/me/sets/{s.id}"><Icon name="pencil" size={16} />Edit set</a>{/if}
{/snippet}

{#snippet side()}
  {#if s}
    <div class="wide-only">
      <PanelBlock title="Actions">
        <div class="pb-actions">
          {@render clone()}
          <div class:pair={mine}><CopyLink url={() => page.url.origin + page.url.pathname} />{@render edit()}</div>
        </div>
      </PanelBlock>
    </div>
    <PanelBlock title="Facts">
      <FactRows label="Facts" items={[
        ...(owner ? [{ label: 'Owner', key: 'owner' }] : []),
        { label: 'Kind', value: KIND[s.kind] ?? 'Set' },
        { label: 'Emotes', key: 'cap' },
        ...(s.createdAt ? [{ label: 'Made', key: 'made' }] : []),
      ]}>
        {#snippet value(k)}
          {#if k === 'owner' && owner}<a class="owner" href="/users/{owner.id}"><Avatar src={owner.avatar} name={owner.displayName} size={20} /><bdi>{clean(owner.displayName)}</bdi></a>
          {:else if k === 'cap'}
            <span class="cap">
              <span class="ui-mono"><Capacity {used} {capacity} /></span>
              <span class="bar" role="meter" aria-label="How full" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct}>
                <span class="fill" class:full={pct >= 90} style:width="{pct}%"></span>
              </span>
            </span>
          {:else if k === 'made' && s.createdAt}<time class="ui-mono" datetime={s.createdAt}>{date(s.createdAt)}</time>{/if}
        {/snippet}
      </FactRows>
    </PanelBlock>
    <!-- Editors come last: the block arrives after the session (owner and staff only), so nothing under it can move. -->
    {#if editorList && (editorList.length || mine)}
      <PanelBlock title="Editors" count={editorList.length ? count(editorList.length) : undefined} more={mine ? { href: '/me/editors', label: 'Manage' } : undefined}>
        {#if editorList.length}
          <ul class="eds">
            {#each editorList.slice(0, 10) as e (e.id)}
              <li><Avatar src={e.avatar} name={clean(e.displayName)} size={24} /><a href="/users/{encodeURIComponent(e.id)}"><PaintedName name={e.displayName} paint={looks[e.id]?.paint} /></a></li>
            {/each}
          </ul>
          {#if editorList.length > 10}<p class="pb-hint">And {count(editorList.length - 10)} more.</p>{/if}
        {:else}
          <p class="pb-hint">Nobody else edits this set. Only you see this.</p>
        {/if}
      </PanelBlock>
    {/if}
  {/if}
{/snippet}

{#if s}
  {#key s.id}
    <InspectorLayout title="Set" label="About {name}" panel={side} fold testid="set-panel">
      {#snippet head()}
        <header class="head" aria-labelledby="set-h">
          <div class="title-row">
            {#if renaming && menuSet}
              <h1 class="page-title editing" id="set-h"><InlineRename id={menuSet.id} name={menuSet.name} size="title" ondone={renamed} /></h1>
            {:else}
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <h1 class="page-title" id="set-h" oncontextmenu={menuAt}><bdi>{name}</bdi></h1>
            {/if}
            <button class="ui-icon more" type="button" bind:this={menuButton} aria-label="Set actions for {name}" aria-haspopup="menu"
              onclick={(e) => menuSet && acts?.open(menuSet, e.currentTarget)} oncontextmenu={menuAt}><Icon name="more" size={18} /></button>
          </div>
          <p class="meta">
            {#if owner}
              <a class="owner" href="/users/{owner.id}">
                <Avatar src={owner.avatar} name={owner.displayName} size={24} />
                <span><span class="sr-only">By </span><bdi>{clean(owner.displayName)}</bdi></span>
              </a>
            {/if}
            {#if unlisted}
              <span class="mark" title="Only people with the link can find it"><Icon name="link" size={15} />Unlisted</span>
            {/if}
          </p>
          <div class="actions narrow-only">
            {@render clone()}
            <CopyLink url={() => page.url.origin + page.url.pathname} />
            {@render edit()}
          </div>
        </header>
      {/snippet}

      {#if members.length}
        <div class="tools">
          <label class="find">
            <span class="sr-only">Search this set</span>
            <Icon name="search" size={16} />
            <input class="input" type="search" placeholder="Search this set" bind:value={query} autocomplete="off" data-page-search />
          </label>
          <p class="total" aria-live="polite">
            {needle ? `${count(shown.length)} of ${count(members.length)}` : count(members.length)}
            {members.length === 1 ? 'emote' : 'emotes'}
          </p>
          <span class="sw"><SelectSwitch on={selecting} onchange={(v) => setSelecting(v)} /></span>
        </div>
      {/if}

      {#if members.length === 0}
        <State title="No emotes in this set yet.">
          {#if mine}Add some from the set’s editor.{:else}Nothing has been added to it yet.{/if}
          {#snippet actions()}
            {#if mine}<a class="btn btn-s btn-primary" href="/me/sets/{s.id}">Add emotes</a>{:else}<a class="btn btn-s" href="/search">Browse emotes</a>{/if}
          {/snippet}
        </State>
      {:else if shown.length === 0}
        <State title="Nothing matches “{needle}”.">
          No emote in this set has that in its name.
          {#snippet actions()}
            <button class="btn btn-s" type="button" onclick={() => (query = '')}>Clear search</button>
          {/snippet}
        </State>
      {:else}
        <BrowseGrid sheet items={cards} eager={12} label="Emotes in {name}" {selecting} {sel} onstep={step} onstart={(id) => setSelecting(true, id)} onstop={() => setSelecting(false)} />
      {/if}
      {#if shown.length > STEP}
        <div class="more"><LoadMore hasMore={shown.length > limit} total={shown.length} onmore={() => (limit += STEP)} /></div>
      {/if}
      {#if selecting && shown.length}
        <SelectBar
          ids={sel.ids}
          shown={cards.length}
          {signin}
          {note}
          onall={() => step(all(sel, order))}
          onclear={() => ((sel = EMPTY), (note = ''))}
          onexit={() => setSelecting(false)}
          onadded={unpick}
        />
      {/if}
    </InspectorLayout>
    <CloneDialog bind:this={dlg} id={s.id} name={s.name} />
    <SetActions bind:this={acts} here="page" onrename={() => (renaming = true)} onchanged={changed} />
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
          <a class="btn btn-s btn-primary" href="/search">Browse emotes</a>
          <a class="btn btn-s" href="/">Front page</a>
        {/snippet}
      </State>
    </div>
  </ColumnLayout>
{/if}

<style>
  .head { display: grid; gap: 10px; margin-bottom: 20px; }
  .head h1 { margin: 0; min-width: 0; }
  .title-row { display: flex; align-items: center; gap: 6px; min-width: 0; }
  .title-row h1 { flex: 0 1 auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .title-row h1.editing { flex: 1 1 auto; overflow: visible; }
  .more { flex: none; color: var(--muted); }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 20px; margin: 0; font-size: var(--t-small); color: var(--muted); }
  .owner { display: inline-flex; align-items: center; gap: 8px; min-width: 0; max-width: 100%; min-height: 32px; color: var(--text); font-weight: 500; }
  .owner span, .owner bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .owner :global(.av) { flex: none; }
  @media (pointer: coarse) { .owner { min-height: 44px; } }
  @media (hover: hover) and (pointer: fine) { .owner:hover bdi { text-decoration: underline; text-underline-offset: 3px; } }
  .mark { display: inline-flex; align-items: center; gap: 6px; }
  .mark :global(svg) { color: var(--accent-text); }
  .actions { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 4px; }

  /* The actions: in the panel on wide screens, in the head under 1100px (one place at a time). */
  .narrow-only { display: none; }
  @media (max-width: 1099.98px) {
    .narrow-only { display: flex; }
    .wide-only { display: none; }
  }

  .wide-only :global(.pb-actions .btn) { width: 100%; }
  .cap .owner, :global(.facts) .owner { justify-content: flex-end; min-height: 0; font-weight: 600; }
  .cap { display: grid; justify-items: end; gap: 6px; }
  .bar { display: block; width: 120px; height: 3px; border-radius: 2px; background: var(--border); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
  .fill.full { background: var(--yellow); }
  @media (forced-colors: active) { .fill { background: Highlight; } }
  .eds { list-style: none; margin: 0; padding: 0; display: grid; }
  .eds li { display: flex; align-items: center; gap: 10px; height: 40px; min-width: 0; font-size: var(--t-small); font-weight: 600; }
  .eds li + li { border-top: 1px solid var(--border); }
  .eds a { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text); }

  /* The slim bar over the sheet (redesign round 2): search, mono count, Select; a hairline under it. */
  .tools { display: flex; flex-wrap: wrap; align-items: center; gap: 8px 16px; min-height: 52px; padding: 6px 0; margin-bottom: 16px;
    border-bottom: 1px solid var(--border); }
  .total { margin: 0; font-family: var(--font-mono); font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; }
  .sw { margin-left: auto; }
  .find { position: relative; display: block; flex: 0 1 280px; min-width: 0; }
  .find :global(svg) { position: absolute; left: 12px; top: 50%; translate: 0 -50%; color: var(--muted); pointer-events: none; }
  .find .input { width: 100%; height: 36px; padding-left: 36px; }
  @media (pointer: coarse) { .find .input { height: 44px; } }
  @media (max-width: 520px) { .find { flex-basis: 100%; } }
  .more { margin-top: 24px; }
  .missing { display: grid; gap: 16px; min-height: 50svh; align-content: start; padding-bottom: 40px; }
  .back { justify-self: start; display: inline-flex; align-items: center; gap: 8px; min-height: 40px; color: var(--muted); font-size: var(--t-small); }
  @media (hover: hover) and (pointer: fine) { .back:hover { color: var(--text); } }
</style>
