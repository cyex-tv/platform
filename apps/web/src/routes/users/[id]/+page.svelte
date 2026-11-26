<script lang="ts">
  import { media, PANEL } from '$lib/media.svelte';
  // The one profile page, for everyone, in the Inspector layout (redesign, 2026/11/21): the person
  // (ProfileHeader, flat: name, roles as flags, member since, the owner's or an editor's buttons;
  // the roles replaced the link-copy button, owner rule) heads the left column, then the
  // section tabs (the same public rows for every viewer, rows.ts) and the section. On the right a
  // panel (lib/shell/InspectorLayout, the frame every Inspector page shares; tabs are LinkTabs) shows
  // About (ProfileAbout: connections and editors, collapsed, and numbers; public, the
  // same for everyone) until an emote tile is picked: then that emote (EmoteInspector, ?e=<id>),
  // and Escape, the close button or back return to About; ← → step through the grid. Under 1100px
  // About sits under the person and tiles open the emote page as anywhere else. The server renders the public
  // page (Uploaded emotes; Emote sets, Cosmetics and Activity are a tab away). In the browser the
  // owner also opens Overview (?s=overview, the dashboard: the account menu and the band's
  // Dashboard button), the owner and the account's editors Active emotes (from the dashboard, or
  // the band's button for editors), and the Cosmetics tab becomes the picker for the owner and
  // editors with PROFILE. The owner alone also has Account and Editors (?s=account, ?s=editors;
  // they moved here from Settings, owner rule 2026/11/22) and Notifications (?s=notifications, the
  // old /me/notifications: the list on the left, the picked one or the numbers in the panel), tabs
  // nobody else is shown. Sections are ?s= tabs; the old ?s=collection opens Cosmetics.
  import { setContext } from 'svelte';
  import { afterNavigate, goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import Problem from '$lib/components/Problem.svelte';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { clean } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';
  import InspectorLayout from '$lib/shell/InspectorLayout.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import LinkTabs from '$lib/components/LinkTabs.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { ACTIVITY_READY } from '$lib/profile/activity';
  import { relationState } from '$lib/profile/data.svelte';
  import ProfileHeader from '$lib/profile/ProfileHeader.svelte';
  import ProfileAbout from '$lib/profile/ProfileAbout.svelte';
  import EmoteInspector from '$lib/profile/EmoteInspector.svelte';
  import { gql } from '$lib/api';
  import { EMOTE } from '$lib/queries';
  import { count } from '$lib/format';
  import type { PageEmote } from '$lib/components/emote/emote-page';
  import Icon from '$lib/components/Icon.svelte';
  import AdminLink from '$lib/profile/AdminLink.svelte';
  import { canProfile, canSets, manages, profileTabs, sectionFrom, sectionHref, sectionsFor, type SectionKey } from '$lib/profile/rows';
  import { lazyFailed, lazyKeyFor, lazySection, loadSection } from '$lib/profile/sections/lazy.svelte';
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
  // The section's chunk: the page's own Uploaded emotes is null; the rest load when first asked for
  // (the page's load already fetched the one in the address) and ahead of time on tab hover or focus.
  const lazyView = $derived({ mine: relation.mine, canProfile: canProfile(relation), manage: relation.perms.has('EMOTE_MANAGE'), signedIn: !!session.me });
  const notes = $derived(section === 'notifications' && relation.mine);
  const lazyKey = $derived(lazyKeyFor(section, lazyView));
  const Lazy = $derived(lazyKey ? lazySection(lazyKey) : undefined);
  $effect(() => {
    if (lazyKey) void loadSection(lazyKey);
  });
  const Panel = $derived(notes ? lazySection('notificationsPanel') : undefined);
  $effect(() => {
    if (notes) void loadSection('notificationsPanel');
  });
  function ahead(ev: Event) {
    const a = (ev.target as Element | null)?.closest?.('nav[aria-label$="profile"] a');
    if (!(a instanceof HTMLAnchorElement)) return;
    const want = sectionFrom(new URL(a.href, page.url).searchParams, allowed);
    const key = lazyKeyFor(want, lazyView);
    if (key) void loadSection(key);
  }
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

  const TITLE: Record<SectionKey, string> = { overview: 'Overview', uploads: 'Uploaded emotes', active: 'Active emotes', sets: 'Emote sets', favourites: 'Favourites', cosmetics: 'Cosmetics', activity: 'Activity', notifications: 'Notifications', account: 'Account', editors: 'Editors' };

  // The section tabs: the profile's public rows, the off-row section you are on, and the owner's own tabs.
  const tabList = $derived(
    profileTabs(data.id, relation, section, page.url.pathname, ACTIVITY_READY).map((t) =>
      // The public count; whoever manages the account sees pending uploads too, so theirs has none.
      t.href === sectionHref(data.id, 'uploads') && !data.error && !manages(relation) ? { ...t, count: count(data.total) } : t,
    ),
  );

  // ---- The panel's emote: a tile picked in the grid shows on the right (wide screens only).
  const wide = $derived(media(PANEL).matches);
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
  // Focus goes back to the tile the emote came from once About is back (after the navigation,
  // which would otherwise leave focus on the page).
  let refocus: string | null = null;
  afterNavigate(() => {
    const id = refocus;
    refocus = null;
    if (id) requestAnimationFrame(() => tileLinks().find((a) => idOf(a) === id)?.focus());
  });
  function unpick() {
    refocus = pickedId;
    if (pushed) {
      pushed = false;
      history.back();
    } else show(null);
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

<svelte:window {onkeydown} onpointerover={ahead} onfocusin={ahead} />

{#snippet actions()}
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
  {@const showing = !!pickedId && wide && section !== 'notifications'}
  <InspectorLayout
    title={notes ? 'Notification' : showing ? 'Selected emote' : 'About'}
    label={notes ? 'About your notifications' : showing ? 'Selected emote' : `About ${name}`}
    fold={!showing}
    headPad={false}
    testid="profile-panel"
  >
    {#snippet head()}
      <ProfileHeader {account} {relation} profile={data.profile} seed={data.look} {actions} facts={false} flat />
    {/snippet}
    {#snippet tabs()}
      <LinkTabs label="{name}'s profile" items={tabList} />
    {/snippet}

    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div class="body" bind:this={body} onclickcapture={pick}>
      {#if waiting}
        <State kind="loading" title="Loading…" />
      {:else if lazyKey && lazyFailed(lazyKey)}
        <Problem cause="server" level={2} onretry={() => loadSection(lazyKey)} />
      {:else if lazyKey && !Lazy}
        <State kind="loading" title="Loading…" />
      {:else if Lazy && lazyKey === 'overview'}
        {#key data.id}<Lazy accountId={data.id} />{/key}
      {:else if Lazy && lazyKey === 'active'}
        {#key data.id}<Lazy accountId={data.id} mine={relation.mine} />{/key}
      {:else if Lazy && lazyKey === 'sets'}
        {#key data.id}<Lazy accountId={data.id} mine={relation.mine} canCreate={canSets(relation)} {name} />{/key}
      {:else if Lazy && lazyKey === 'favourites'}
        <Lazy />
      {:else if Lazy && lazyKey === 'picker'}
        {#key data.id}<Lazy accountId={data.id} mine={relation.mine} {name} />{/key}
      {:else if Lazy && lazyKey === 'shown'}
        {#key data.id}<Lazy accountId={data.id} {name} seed={data.look} />{/key}
      {:else if Lazy && (lazyKey === 'account' || lazyKey === 'editors' || lazyKey === 'notifications')}
        <Lazy />
      {:else if Lazy && lazyKey === 'activity'}
        {#key data.id}<Lazy accountId={data.id} name={data.creator.displayName} full={relation.mine || !!session.me?.permissions.includes('users.view')} />{/key}
      {:else if Lazy && relation.mine}
        <Lazy />
      {:else if Lazy}
        {#key data.id}<Lazy accountId={data.id} name={data.creator.displayName} canUpload={relation.perms.has('EMOTE_UPLOAD')} />{/key}
      {:else}
        {#key data.id}
          <Uploads id={data.id} {name} first={data.items} total={data.total} />
        {/key}
      {/if}
      <AdminLink accountId={data.id} />
    </div>

    {#snippet panelActions()}
      {#if showing && pickedId}
        {#if picked?.id === pickedId && picked.emote}<IconButton icon="open" label="Open full page" size="s" variant="ghost" href="/emotes/{encodeURIComponent(pickedId)}" />{/if}
        <IconButton icon="x" label="Close" size="s" variant="ghost" onclick={unpick} />
      {/if}
    {/snippet}
    {#snippet panel()}
      {#if notes}
        {#if Panel}<Panel />{/if}
      {:else if showing}
        <EmoteInspector head={false} emote={picked?.id === pickedId ? picked.emote : null} failed={picked?.id === pickedId && picked.failed} onclose={unpick} />
      {:else}
        <ProfileAbout accountId={data.id} profile={data.profile} uploads={data.total} />
      {/if}
    {/snippet}
  </InspectorLayout>
{/if}

<style>
  .pad { padding-top: var(--rail-gap); }

  /* The picked emote's tile is ringed in the accent while it shows in the panel. */
  .body :global(a.tile[aria-current='true']) { border-color: var(--accent-border); }
</style>
