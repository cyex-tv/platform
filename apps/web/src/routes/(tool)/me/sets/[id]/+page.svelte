<script lang="ts">
  import { MediaQuery } from 'svelte/reactivity';
  import Capacity from '$lib/components/Capacity.svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import { problemTitle } from '$lib/components/problem';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { visibilityHint, visibilityOptions } from '$lib/sets/visibility';
  import { clean, count, iso } from '$lib/format';
  import { sectionHref } from '$lib/profile/rows';
  import Name from '$lib/components/Name.svelte';
  import type { EditSetQuery, EditorPermission, SetVisibility } from '$lib/gql/graphql';
  import { EDIT_SET, DELETE_SET, REMOVE_SET_EDITOR, UPDATE_CHANNEL, UPDATE_SET } from '$lib/queries';
  import Icon from '$lib/components/Icon.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import SetEmotes from '$lib/tool/SetEmotes.svelte';
  import AddEmotes from '$lib/tool/AddEmotes.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { covers, grantOn, permissionLabels, scopeOf, setAccess } from '$lib/tool/editors';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  type EditableSet = NonNullable<EditSetQuery['set']>;
  let set = $state<EditableSet | null>(null);
  let editors = $state<EditSetQuery['setEditors']>([]);
  let grants = $state<EditSetQuery['editorOf']>([]);
  let channels = $state<EditSetQuery['myChannels']>([]);
  let switching = $state<string | null>(null);
  let name = $state('');
  let saving = $state(false);
  let deleting = $state(false);
  let changing = $state(false);

  const narrow = new MediaQuery('max-width: 1000px');
  const id = $derived(page.params.id ?? '');
  const owner = $derived(set?.owner?.id === session.me?.id);
  // What you may do here: as the owner, a per-set editor, or the owner's account editor (ADR 0014)
  // whose grant covers this set.
  const grant = $derived(grants.find((g) => g.account.id === set?.owner?.id));
  const covered = $derived(!!set && covers(scopeOf(grant), set.id));
  const held = $derived(owner || !covered ? new Set<EditorPermission>() : grantOn(grants, set?.owner?.id, session.me?.id));
  const setEditor = $derived(editors.some((e) => e.id === session.me?.id));
  // Staff may edit any set, and the global set with its own permission.
  const staff = $derived(can('sets.edit_any') || (set?.kind === 'GLOBAL' && can('global_set.edit')));
  const access = $derived(setAccess({ owner, setEditor, staff, held }));
  const canRename = $derived(access.rename);
  const canDelete = $derived(access.manage);
  const canSee = $derived(access.manage && set?.kind !== 'GLOBAL');
  // Channels (yours or the ones you change) that show this set right now: it can't be deleted then.
  const shownOn = $derived(channels.filter((c) => c.activeSetId === set?.id));
  // The channels this set can be shown on: the owner's, when you may change their settings.
  const usable = $derived(set?.kind === 'CHANNEL' && (owner || held.has('CHANNELS')) ? channels.filter((c) => c.account.id === set?.owner?.id) : []);
  const showing = $derived(usable.filter((c) => c.activeSetId === set?.id));
  const channelKey = (c: { platform: string; platformChannelId: string }) => `${c.platform}:${c.platformChannelId}`;
  const platformName = (p: string) => (p === 'TWITCH' ? 'Twitch' : 'Kick');

  const L = loader(async ({ fetch }) => {
    const want = page.params.id ?? '';
    let r: EditSetQuery;
    try {
      r = await gql(EDIT_SET, { id: want }, fetch);
    } catch (e) {
      // Not yours to edit: the set's own page is the place for you.
      if (e instanceof ApiError && e.code === 'forbidden') {
        await goto(`/sets/${encodeURIComponent(want)}`, { replaceState: true });
        throw new ApiError('Only the set’s owner and editors can change it.', 'forbidden');
      }
      throw e;
    }
    if (!r.set) throw new ApiError('This set doesn’t exist.', 'not_found');
    if (want === page.params.id) {
      set = r.set;
      editors = r.setEditors;
      grants = r.editorOf;
      // Your own channels, then the ones you change for others (the same channel can't be both).
      channels = [...r.myChannels, ...r.editorChannels.filter((c) => !r.myChannels.some((m) => m.platform === c.platform && m.platformChannelId === c.platformChannelId))];
      name = r.set.name;
    }
    return r.set;
  });
  const load = () => L.reload();

  async function rename(e: SubmitEvent) {
    e.preventDefault();
    if (saving || !set || !name.trim() || name.trim() === set.name) return;
    saving = true;
    try {
      const r = await gql(UPDATE_SET, { id: set.id, input: { name: name.trim() } });
      set.name = r.updateSet.name;
      notify('Renamed.');
    } catch (err) {
      fail(err);
    } finally {
      saving = false;
    }
  }

  async function setVisibility(v: SetVisibility) {
    if (!set || changing || v === set.visibility) return;
    changing = true;
    try {
      const r = await gql(UPDATE_SET, { id: set.id, input: { visibility: v } });
      set.visibility = r.updateSet.visibility;
      notify(`${visibilityOptions(set.kind).find((o) => o.value === set!.visibility)?.label ?? 'Visibility'} saved.`);
    } catch (err) {
      fail(err);
    } finally {
      changing = false;
    }
  }

  async function remove() {
    if (!set) return;
    deleting = true;
    try {
      await gql(DELETE_SET, { id: set.id });
      notify(`${iso(set.name)} deleted.`);
      await goto('/me/sets');
    } catch (err) {
      fail(err);
      deleting = false;
    }
  }

  async function useOn(c: EditSetQuery['myChannels'][number]) {
    if (!set || switching) return;
    switching = channelKey(c);
    try {
      const r = await gql(UPDATE_CHANNEL, { platform: c.platform, id: c.platformChannelId, input: { activeSetId: set.id } });
      channels = channels.map((x) => (channelKey(x) === channelKey(c) ? { ...x, activeSetId: r.updateChannelSettings.activeSetId ?? null } : x));
      notify(`${platformName(c.platform)} ${c.login} now shows ${iso(set.name)}.`);
    } catch (err) {
      fail(err);
    } finally {
      switching = null;
    }
  }

  async function leave() {
    if (!set || !session.me) return;
    try {
      await gql(REMOVE_SET_EDITOR, { setId: set.id, accountId: session.me.id });
      notify(`You no longer edit ${iso(set.name)}.`);
      await goto('/me/sets');
    } catch (err) {
      fail(err);
    }
  }

  const taken = $derived(new Set(set?.emotes.map((m) => m.emote.id) ?? []));
  const names = $derived(new Set(set?.emotes.map((m) => m.name) ?? []));
  const full = $derived(set ? set.emotes.length >= set.capacity : false);
</script>

<svelte:head><title>{L.cause ? problemTitle(L.cause, true, 'No set here') : `${set?.name ?? 'Set'} - cyex.tv`}</title></svelte:head>

{#snippet channelBlock()}
  <section class="ui-card">
    <h2>Use on {usable.length === 1 ? 'my channel' : 'a channel'}</h2>
    <ul class="ui-list chans">
      {#each usable as c (channelKey(c))}
        {@const on = c.activeSetId === set?.id}
        <li>
          <Icon name={c.platform === 'TWITCH' ? 'twitch' : 'kick'} size={16} />
          <span class="chan"><span class="ui-hint">{platformName(c.platform)}</span><bdi>{c.login}</bdi></span>
          {#if on}
            <span class="live"><Icon name="check" size={15} />Shows this set</span>
          {:else}
            <button class="ui-btn secondary small" type="button" disabled={switching !== null} onclick={() => useOn(c)}
              aria-label="Use this set on {platformName(c.platform)} {c.login}">{switching === channelKey(c) ? 'Switching…' : 'Use here'}</button>
          {/if}
        </li>
      {/each}
    </ul>
    <p class="ui-hint gap">Chat shows a channel’s active set. Picking this one replaces what the channel shows now.</p>
  </section>
{/snippet}

{#snippet editorsLink()}
  <section class="ui-card">
    <h2>Editors</h2>
    <p class="ui-hint">Your editors work on your sets for you. Choose who, what they can do and which sets on the Editors page.</p>
    <div class="ui-actions gap"><a class="ui-btn secondary small" href="/me/editors">Manage editors<Icon name="arrow-right" size={15} /></a></div>
  </section>
{/snippet}

{#snippet dangerBlock()}
  {#if set}
    <section class="ui-card">
      {#if canDelete}
        <h2>Delete set</h2>
        {#if shownOn.length}
          <p class="ui-hint">{shownOn.map((c) => `${platformName(c.platform)} ${c.login}`).join(' and ')} shows this set. Pick another active set in <a href="/me/channels">Channels</a> first.</p>
          <div class="ui-actions gap"><button class="ui-btn danger" type="button" disabled>Delete set</button></div>
        {:else}
          <p class="ui-hint">The emotes stay; only this set goes.</p>
          <div class="ui-actions gap"><Confirm label="Delete set" confirm="Click again to delete" busy={deleting} onconfirm={remove} /></div>
        {/if}
      {:else if setEditor}
        <h2>Stop editing</h2>
        <p class="ui-hint">You lose access to this set until the owner adds you again.</p>
        <div class="ui-actions gap"><Confirm label="Leave" confirm="Click again to leave" onconfirm={leave} /></div>
      {/if}
    </section>
  {/if}
{/snippet}

<LoadGate
  source={L}
  label="Loading the set…"
  level={1}
  home={{ href: sectionHref(session.me?.id ?? '', 'sets'), label: 'Your sets' }}
  missing={{ title: 'No set here.', body: 'It may have been deleted, or the address is off.' }}
>
{#snippet children()}
{#if set && !access.emotes}
  <State kind="loading" title="Opening the set…" />
{:else if set}
  <PageHead
    name
    title={clean(set.name)}
    back={set.kind === 'GLOBAL' ? { href: '/admin', label: 'Admin' } : { href: '/me/sets', label: 'Sets' }}
  >
    {#snippet line()}
      {set!.kind === 'GLOBAL' ? 'The global set' : set!.kind === 'PERSONAL' ? 'Personal set' : 'Channel set'} ·
      <Capacity used={set!.emotes.length} capacity={set!.capacity} /> emotes
      {#if !owner && set!.owner}· by <Name value={set!.owner.displayName} />{/if}
      {#if held.size}· you edit this account: {permissionLabels([...held])}{/if}
      {#if showing.length}· shown on {showing.map((c) => `${platformName(c.platform)} ${c.login}`).join(' and ')}{/if}
    {/snippet}
  </PageHead>

  <div class="layout">
    <div class="ui-stack main">
      <SetEmotes {set} onchange={load} />
      <AddEmotes setId={set.id} personal={set.kind === 'PERSONAL'} {taken} {names} {full} onadded={load} />
    </div>
    <aside class="ui-stack">
      {#if canRename}
      <section class="ui-card ui-form">
        <form onsubmit={rename}>
          <h2><label for="set-name">Name</label></h2>
          <div class="row">
            <input id="set-name" class="ui-input" bind:value={name} maxlength="100" />
            <button class="ui-btn secondary" type="submit" disabled={saving || !name.trim() || name.trim() === set.name}>Rename</button>
          </div>
        </form>
        {#if canSee}
          <div class="vis">
            <h3 id="vis-label">Who can open it</h3>
            <Segmented label="Who can open it" size="s" options={visibilityOptions(set.kind)} value={set.visibility} onchange={setVisibility} />
            <p class="ui-hint" aria-live="polite">{visibilityHint(set.visibility)}{#if set.kind === 'PERSONAL'} A personal set can’t be private.{/if}</p>
          </div>
        {/if}
      </section>
      {/if}

      {#if usable.length}{@render channelBlock()}{/if}

      {#if owner && set.kind !== 'GLOBAL'}{@render editorsLink()}{/if}

      {#if set.kind !== 'GLOBAL' && (canDelete || setEditor)}
        {#if narrow.current}
          <details class="fold"><summary>{canDelete ? 'Delete set' : 'Stop editing'}</summary>{@render dangerBlock()}</details>
        {:else}
          {@render dangerBlock()}
        {/if}
      {/if}
    </aside>
  </div>
{/if}
{/snippet}
</LoadGate>

<style>
  .layout { display: grid; grid-template-columns: minmax(0, 1fr) 320px; gap: 16px; align-items: start; }
  .row { display: flex; gap: 8px; }
  .row input { min-width: 0; }
  .gap { margin-top: 12px; }
  .vis { display: grid; gap: 8px; margin-top: 18px; padding-top: 14px; border-top: 1px solid var(--border); }
  .vis h3 { margin: 0; font-size: var(--t-ui); font-weight: 600; }
  .vis .ui-hint { margin: 0; }
  /* Three options in a 276px card: tighter than the default 12px padding. */
  .vis :global(.seg button) { padding: 0 8px; gap: 5px; }
  .vis :global(.seg) { margin-left: -8px; }
  .chans { margin: 0 -22px; border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); }
  .chans > li { padding: 8px 22px; min-height: 52px; }
  .chans :global(svg) { flex: none; }
  .chan { display: grid; flex: 1; min-width: 0; line-height: 1.3; }
  .chan bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .live { display: inline-flex; align-items: center; gap: 6px; font-weight: 600; white-space: nowrap; }
  .live :global(svg) { color: var(--accent); }
  .fold > summary { cursor: pointer; padding: 10px 14px; font-weight: 600; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  .fold[open] > summary { margin-bottom: 8px; }
  /* Narrow: the emotes come first, the settings under them. */
  @media (max-width: 1000px) { .layout { grid-template-columns: minmax(0, 1fr); } }
</style>
