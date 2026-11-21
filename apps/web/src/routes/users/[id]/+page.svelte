<script lang="ts">
  // The one profile page, for everyone, in the Inspector layout (redesign, 2026/11/21): the person
  // (ProfileHeader, flat: name, roles, member since, the buttons) heads the left column, then the
  // section tabs (the same public rows for every viewer, rows.ts) and the section. On the right a
  // panel shows About (ProfileAbout: connections and editors, collapsed, and numbers; public, the
  // same for everyone) until an emote tile is picked: then that emote (EmoteInspector, ?e=<id>),
  // and Escape, the close button or back return to About; ← → step through the grid. Under 1100px
  // About sits under the person and tiles open the emote page as anywhere else. The server renders the public
  // page (Uploaded emotes; Emote sets, Cosmetics and Activity are a tab away). In the browser the
  // owner also opens Overview (?s=overview, the dashboard: the account menu and the band's
  // Dashboard button), the owner and the account's editors Active emotes (from the dashboard, or
  // the band's button for editors), and the Cosmetics tab becomes the picker for the owner and
  // editors with PROFILE. Sections are ?s= tabs; the old ?s=collection opens Cosmetics.
  import { setContext } from 'svelte';
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import Problem from '$lib/components/Problem.svelte';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { clean } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { ACTIVITY_READY } from '$lib/profile/activity';
  import { relationState } from '$lib/profile/data.svelte';
  import ProfileHeader from '$lib/profile/ProfileHeader.svelte';
  import ProfileAbout from '$lib/profile/ProfileAbout.svelte';
  import EmoteInspector from '$lib/profile/EmoteInspector.svelte';
  import CopyLink from '$lib/components/CopyLink.svelte';
  import { arrowNav } from '$lib/shell/rail';
  import { gql } from '$lib/api';
  import { EMOTE } from '$lib/queries';
  import { count } from '$lib/format';
  import type { PageEmote } from '$lib/components/emote/emote-page';
  import Icon from '$lib/components/Icon.svelte';
  import AdminLink from '$lib/profile/AdminLink.svelte';
  import Overview from '$lib/profile/sections/Overview.svelte';
  import { canProfile, canSets, manages, profileRail, sectionFrom, sectionHref, sectionsFor, stripExtra, stripTabs, type SectionKey } from '$lib/profile/rows';
  import ActiveEmotes from '$lib/profile/sections/ActiveEmotes.svelte';
  import ActivityFeed from '$lib/profile/sections/ActivityFeed.svelte';
  import CosmeticsPicker from '$lib/profile/sections/CosmeticsPicker.svelte';
  import ShownCosmetics from '$lib/profile/sections/ShownCosmetics.svelte';
  import Favourites from '$lib/profile/sections/Favourites.svelte';
  import MyUploads from '$lib/profile/sections/MyUploads.svelte';
  import SetCards from '$lib/profile/sections/SetCards.svelte';
  import Uploads from './Uploads.svelte';

  let { data } = $props();
  // The page's h1 is the person's name (below); the sections are h2s.
  setContext('heading-level', 2);
  const name = $derived(clean(data.creator?.displayName ?? ''));
  const rel = relationState(() => data.id);
  const relation = $derived(rel.value);
  const allowed = $derived(sectionsFor(relation, ACTIVITY_READY));
  const asked = $derived(page.url.searchParams.get('s'));
  const section = $derived<SectionKey>(sectionFrom(page.url.searchParams, allowed));
  // Grants load after the session; give them a moment before falling back to Uploaded emotes.
  let granted = $state(false);
  $effect(() => {
    void data.id;
    granted = false;
    const t = setTimeout(() => (granted = true), 1500);
    return () => clearTimeout(t);
  });
  // A section only the owner or an editor has, asked for before the session is known: wait for it.
  const waiting = $derived(!!asked && asked !== section && (!session.known || (!!session.me && !granted)));

  const TITLE: Record<SectionKey, string> = { overview: 'Overview', uploads: 'Uploaded emotes', active: 'Active emotes', sets: 'Emote sets', favourites: 'Favourites', cosmetics: 'Cosmetics', activity: 'Activity' };

  // The section tabs: the profile's public rows, plus the off-row section you are on.
  const tabs = $derived(
    stripTabs(profileRail(data.id, section, ACTIVITY_READY), stripExtra(page.url.pathname, section)).map((t) =>
      // The public count; whoever manages the account sees pending uploads too, so theirs has none.
      t.href === sectionHref(data.id, 'uploads') && !data.error && !manages(relation) ? { ...t, count: count(data.total) } : t,
    ),
  );

  // ---- The panel's emote: a tile picked in the grid shows on the right (wide screens only).
  let wide = $state(false);
  $effect(() => {
    const mq = matchMedia('(min-width: 1100px)');
    wide = mq.matches;
    const on = () => (wide = mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });
  const pickedId = $derived(page.url.searchParams.get('e'));
  let picked = $state<{ id: string; emote: PageEmote | null; failed: boolean } | null>(null);
  const cache = new Map<string, PageEmote>();
  $effect(() => {
    const id = pickedId;
    if (!id || !wide) return;
    const known = cache.get(id);
    if (known) {
      picked = { id, emote: known, failed: false };
      return;
    }
    picked = { id, emote: null, failed: false };
    let gone = false;
    gql(EMOTE, { id })
      .then((r) => {
        if (gone) return;
        if (r.emote) cache.set(id, r.emote);
        picked = { id, emote: r.emote ?? null, failed: !r.emote };
      })
      .catch(() => {
        if (!gone) picked = { id, emote: null, failed: true };
      });
    return () => (gone = true);
  });
  // Under 1100px About sits above the tabs, folded until opened (the grid stays near the top).
  let aboutOpen = $state(false);
  let body = $state<HTMLElement>();
  const tileLinks = () => [...(body?.querySelectorAll<HTMLAnchorElement>('a.tile[href^="/emotes/"]') ?? [])];
  const idOf = (a: Element) => decodeURIComponent((a.getAttribute('href') ?? '').match(/^\/emotes\/([^/?#]+)/)?.[1] ?? '');
  // Mark the picked tile (aria-current) while it shows in the panel.
  $effect(() => {
    const id = wide ? pickedId : null;
    void section;
    void picked;
    for (const a of tileLinks()) {
      if (id && idOf(a) === id) a.setAttribute('aria-current', 'true');
      else a.removeAttribute('aria-current');
    }
  });
  let pushed = false;
  function show(id: string | null) {
    const url = new URL(page.url);
    if (id) url.searchParams.set('e', id);
    else url.searchParams.delete('e');
    const replace = !!pickedId;
    if (!pickedId && id) pushed = true;
    void goto(`${url.pathname}${url.search}`, { noScroll: true, keepFocus: true, replaceState: replace });
  }
  function pick(ev: MouseEvent) {
    if (!wide || ev.defaultPrevented || ev.button !== 0 || ev.metaKey || ev.ctrlKey || ev.shiftKey || ev.altKey) return;
    const a = (ev.target as Element | null)?.closest?.('a.tile');
    if (!(a instanceof HTMLAnchorElement) || a.getAttribute('role') === 'checkbox') return;
    const id = idOf(a);
    if (!id) return;
    ev.preventDefault();
    show(id);
  }
  function unpick() {
    const id = pickedId;
    if (pushed) {
      pushed = false;
      history.back();
    } else show(null);
    // Focus goes back to the tile it came from.
    requestAnimationFrame(() => tileLinks().find((a) => idOf(a) === id)?.focus());
  }
  function onkeydown(ev: KeyboardEvent) {
    if (!pickedId || !wide || ev.defaultPrevented || ev.ctrlKey || ev.metaKey || ev.altKey) return;
    const t = ev.target as HTMLElement;
    if (t.closest('input, textarea, select, [contenteditable="true"], [role="menu"], [role="dialog"], [role="tablist"], nav')) return;
    if (ev.key === 'Escape') {
      ev.preventDefault();
      unpick();
    } else if (ev.key === 'ArrowLeft' || ev.key === 'ArrowRight') {
      const list = tileLinks();
      const i = list.findIndex((a) => idOf(a) === pickedId);
      const next = list[i + (ev.key === 'ArrowRight' ? 1 : -1)];
      if (i < 0 || !next) return;
      ev.preventDefault();
      show(idOf(next));
      next.focus({ preventScroll: false });
    }
  }
</script>

<svelte:window {onkeydown} />

{#snippet actions()}
  <CopyLink url={() => `${page.url.origin}/users/${encodeURIComponent(data.id)}`} />
  {#if relation.mine}
    <a class="btn btn-s" href={sectionHref(data.id, 'overview')} aria-current={section === 'overview' ? 'page' : undefined}><Icon name="gauge" size={15} />Dashboard</a>
    <a class="btn btn-s" href="/me/settings"><Icon name="settings" size={15} />Settings</a>
  {:else if manages(relation)}
    <a class="btn btn-s" href={sectionHref(data.id, 'active')} aria-current={section === 'active' ? 'page' : undefined}><Icon name="zap" size={15} />Active emotes</a>
  {/if}
{/snippet}

<svelte:head>
  <title>{data.creator ? `${section === 'uploads' ? name : `${TITLE[section]} · ${name}`} - cyex.tv` : data.error ? problemTitle('server') : 'Nothing here - cyex.tv'}</title>
</svelte:head>

{#if data.error}
  <ColumnLayout>
    <div class="pad">
      <Problem cause="server" level={1} onretry={() => invalidateAll()} />
    </div>
  </ColumnLayout>
{:else if !data.creator}
  <ColumnLayout>
    <div class="pad">
      <State level={1} role="status" title="No profile here.">
        There is no account at this address, or its owner has deleted it. Check the link, or look through what
        everyone has made.
        {#snippet actions()}
          <a class="btn btn-s btn-primary" href="/search">Browse emotes</a>
        {/snippet}
      </State>
    </div>
  </ColumnLayout>
{:else}
  {@const account = { id: data.id, displayName: data.creator.displayName, avatar: data.creator.avatar }}
  <div class="wrap pf" class:picking={!!pickedId}>
    <div class="head">
      <ProfileHeader {account} {relation} profile={data.profile} seed={data.look} {actions} facts={false} flat />
    </div>

    <nav class="ptabs" aria-label="{name}'s profile" use:arrowNav={'horizontal'}>
      {#each tabs as t (t.href)}
        <a href={t.href} aria-current={t.current ? 'page' : undefined}>
          <span class="tl" data-t={t.label}>{t.label}</span>{#if t.count !== undefined}<span class="n">{t.count}</span>{/if}
        </a>
      {/each}
    </nav>

    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="body" bind:this={body} onclickcapture={pick}>
      {#if waiting}
        <State kind="loading" title="Loading…" />
      {:else if section === 'overview'}
        {#key data.id}<Overview accountId={data.id} />{/key}
      {:else if section === 'active'}
        {#key data.id}<ActiveEmotes accountId={data.id} mine={relation.mine} />{/key}
      {:else if section === 'sets'}
        {#key data.id}<SetCards accountId={data.id} mine={relation.mine} canCreate={canSets(relation)} {name} />{/key}
      {:else if section === 'favourites'}
        <Favourites />
      {:else if section === 'cosmetics' && canProfile(relation)}
        {#key data.id}<CosmeticsPicker accountId={data.id} mine={relation.mine} {name} />{/key}
      {:else if section === 'cosmetics'}
        {#key data.id}<ShownCosmetics accountId={data.id} {name} seed={data.look} />{/key}
      {:else if section === 'activity'}
        {#key data.id}<ActivityFeed accountId={data.id} name={data.creator.displayName} />{/key}
      {:else if relation.mine}
        <MyUploads />
      {:else if relation.perms.has('EMOTE_MANAGE')}
        {#key data.id}<MyUploads accountId={data.id} name={data.creator.displayName} canUpload={relation.perms.has('EMOTE_UPLOAD')} />{/key}
      {:else}
        {#key data.id}
          <Uploads id={data.id} {name} first={data.items} total={data.total} />
        {/key}
      {/if}
      <AdminLink accountId={data.id} />
    </div>

    <aside class="panel" aria-label={pickedId ? 'Selected emote' : `About ${name}`} data-testid="profile-panel">
      {#if pickedId && wide}
        <EmoteInspector emote={picked?.id === pickedId ? picked.emote : null} failed={picked?.id === pickedId && picked.failed} onclose={unpick} />
      {:else}
        <div class="ip-head">
          <h2 class="label" id="about-h">About</h2>
          <button class="about-toggle" type="button" aria-expanded={aboutOpen} aria-controls="about-body" onclick={() => (aboutOpen = !aboutOpen)}>
            <span class="sr-only">About {name}</span><span aria-hidden="true">{aboutOpen ? 'Hide' : 'Show'}</span><Icon name="chevron-down" size={14} />
          </button>
        </div>
        <div class="ip-body" id="about-body" class:shut={!aboutOpen}><ProfileAbout accountId={data.id} profile={data.profile} uploads={data.total} /></div>
      {/if}
    </aside>
  </div>
{/if}

<style>
  .pad { padding-top: var(--rail-gap); }

  /* The Inspector profile: the person, the tabs and the grid on the left; on the right a panel
     with About (connections, editors, numbers) until an emote is picked, then that emote. */
  .pf { display: grid; grid-template-columns: minmax(0, 1fr) 360px; grid-template-areas: 'head panel' 'tabs panel' 'body panel';
    grid-template-rows: auto auto 1fr; column-gap: 24px; align-items: start; }
  .head { grid-area: head; min-width: 0; }
  .ptabs { grid-area: tabs; min-width: 0; }
  .body { grid-area: body; min-width: 0; padding: 20px 0 48px; }
  .panel { grid-area: panel; align-self: start; position: sticky; top: var(--nav-h); height: calc(100svh - var(--nav-h)); overflow-y: auto;
    overscroll-behavior: contain; margin-right: calc(-1 * var(--page-pad)); border-left: 1px solid var(--border); background: var(--surface); }
  .ip-head { position: sticky; top: 0; z-index: 1; display: flex; align-items: center; height: 48px; padding: 0 16px;
    border-bottom: 1px solid var(--border); background: var(--surface); }
  .label { margin: 0; font-size: var(--t-label, 11px); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .ip-body { padding: 8px 16px 16px; }
  .about-toggle { display: none; }

  /* The tabs: words on a hairline; the current one bold (no bar or fill). */
  .ptabs { display: flex; gap: 4px; overflow-x: auto; scrollbar-width: none; border-bottom: 1px solid var(--border); margin-inline: -10px; padding-inline: 0; }
  .ptabs a { flex: none; display: inline-flex; align-items: center; gap: 6px; height: 44px; padding: 0 10px; color: var(--muted);
    font-size: var(--t-ui); font-weight: 500; white-space: nowrap; transition: color var(--fast); }
  .ptabs a[aria-current='page'] { color: var(--text); font-weight: 700; }
  .ptabs a:focus-visible { outline-offset: -2px; }
  .tl { display: inline-grid; }
  .tl::after { content: attr(data-t); font-weight: 700; height: 0; overflow: hidden; visibility: hidden; user-select: none; }
  .n { font-size: var(--t-small); color: var(--dim); font-weight: 500; font-variant-numeric: tabular-nums; }
  @media (hover: hover) and (pointer: fine) { .ptabs a:hover { color: var(--text); } }

  /* The picked emote's tile is ringed in the accent while it shows in the panel. */
  .body :global(a.tile[aria-current='true']) { border-color: var(--accent-border); }

  /* Narrower: one column; About follows the person, and tiles open the emote page. */
  @media (max-width: 1099.98px) {
    .pf { grid-template-columns: minmax(0, 1fr); grid-template-areas: 'head' 'panel' 'tabs' 'body'; grid-template-rows: none; }
    .panel { position: static; height: auto; overflow: visible; margin: 0 0 16px; border: 1px solid var(--border); border-radius: var(--radius-card); }
    .ip-head { position: static; height: auto; padding: 0; border-radius: var(--radius-card); border-bottom: 0; }
    .ip-head .label { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
    .about-toggle { display: flex; align-items: center; justify-content: space-between; gap: 8px; width: 100%; min-height: 44px; padding: 0 16px;
      border: 0; border-radius: var(--radius-card); background: none; color: var(--text); font: inherit; font-size: var(--t-small); font-weight: 600; cursor: pointer; }
    .about-toggle::before { content: 'About'; margin-right: auto; font-size: var(--t-label, 11px); letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
    .about-toggle :global(svg) { color: var(--dim); transition: transform var(--fast) var(--ease); }
    .about-toggle[aria-expanded='true'] :global(svg) { transform: rotate(180deg); }
    .about-toggle:focus-visible { outline-offset: -2px; }
    .ip-body { border-top: 1px solid var(--border); }
    .ip-body.shut { display: none; }
  }
</style>
