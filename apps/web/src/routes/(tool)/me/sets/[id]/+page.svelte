<script lang="ts">
  // A set's editor, in the Workspace (redesign round 2): your area's rail on the left with this set
  // current (its meter as on the browse rail); in the middle the set's emotes as a contact sheet
  // under a slim bar (SetEmotes) and Add emotes; on the right the inspector: its name, its
  // capacity (the allowed range from setCapacityRange), the channels it can be shown on, its facts
  // (capacity meter, owner, editors) and deleting it (or leaving it, for a set editor). Every set
  // is public, so there is no visibility choice (ADR 0017, amendment 2026/11/22). Under 1100px the
  // panel follows the emotes. Owners, set editors, account editors whose grant covers the set
  // (ADR 0014) and staff.
  import Avatar from '$lib/components/Avatar.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, wantLooks } from '$lib/cosmetics/store.svelte';
  import FactRows from '$lib/components/FactRows.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import Capacity from '$lib/components/Capacity.svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import { problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { clean, count, iso } from '$lib/format';
  import { sectionHref } from '$lib/profile/rows';
  import Name from '$lib/components/Name.svelte';
  import type { EditSetQuery, EditorPermission } from '$lib/gql/graphql';
  import { EDIT_SET, DELETE_SET, REMOVE_SET_EDITOR, SET_CAPACITY_RANGE, SET_MEMBERS, UPDATE_CHANNEL, UPDATE_SET } from '$lib/queries';
  import CapacityForm from '$lib/sets/CapacityForm.svelte';
  import { slotsLine, slotsNote, type CapacityRange } from '$lib/sets/capacity';
  import Icon from '$lib/components/Icon.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import Confirm from '$lib/tool/Confirm.svelte';
  import SetEmotes from '$lib/tool/SetEmotes.svelte';
  import AddEmotes from '$lib/tool/AddEmotes.svelte';
  import { can, session } from '$lib/tool/session.svelte';
  import { covers, grantOn, permissionLabels, scopeOf, setAccess } from '$lib/tool/editors';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import { applyChange, morePages, replayChanges, type MemberChange } from '$lib/tool/set-paging';
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
  // Channels (yours or the ones you change) that show this set right now: it can't be deleted then.
  const shownOn = $derived(channels.filter((c) => c.activeSetId === set?.id));
  // The channels this set can be shown on: the owner's, when you may pick their active set
  // (managing sets, or channel settings as before).
  const usable = $derived(set?.kind === 'CHANNEL' && (owner || held.has('SETS') || held.has('CHANNELS')) ? channels.filter((c) => c.account.id === set?.owner?.id) : []);
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
      if (set?.id !== r.set.id) members = { ids: new Set(), names: new Set() };
      set = r.set;
      editors = r.setEditors;
      grants = r.editorOf;
      // Your own channels, then the ones you change for others (the same channel can't be both).
      channels = [...r.myChannels, ...r.editorChannels.filter((c) => !r.myChannels.some((m) => m.platform === c.platform && m.platformChannelId === c.platformChannelId))];
      name = r.set.name;
    }
    // The member lists are read once per set; what is done here after that updates them locally.
    if (membersFor !== r.set.id) {
      membersFor = r.set.id;
      void readMembers(r.set.id);
    }
    return r.set;
  });
  const load = (change?: MemberChange) => {
    if (change) track(change);
    return L.reload();
  };
  // Emotes were added: the pages in the grid are read again too.
  let rev = $state(0);
  async function added(a: { id: string; name: string }) {
    track({ kind: 'add', ...a });
    await L.reload();
    rev++;
  }

  // Who is in the set (ids and names only, 200 to a request) for the Add panel's "Added" and
  // name clashes; the grid itself reads pages of 120 with pictures, so nothing waits for this.
  let members = $state({ ids: new Set<string>(), names: new Set<string>() });
  let membersToken = 0;
  /** What was done here while the lists were being read: put on top of them when they arrive. */
  let sinceRead: MemberChange[] = [];
  function track(change: MemberChange) {
    sinceRead.push(change);
    members = applyChange(members, change);
  }
  /** The set the lists were asked for. */
  let membersFor = '';
  async function readMembers(want: string) {
    const mine = ++membersToken;
    sinceRead = [];
    try {
      const first = await gql(SET_MEMBERS, { id: want, page: 1, perPage: 200 });
      const rest = await Promise.all(morePages(first.setEmotes.total, 200).map((n) => gql(SET_MEMBERS, { id: want, page: n, perPage: 200 })));
      if (mine !== membersToken) return;
      const all = [first, ...rest].flatMap((r) => r.setEmotes.items);
      members = replayChanges({ ids: new Set(all.map((m) => m.emote.id)), names: new Set(all.map((m) => m.name)) }, sinceRead);
      sinceRead = [];
    } catch {
      if (mine === membersToken) membersFor = ''; // the next reload asks again
      /* the Add panel then lets the API refuse a repeat */
    }
  }

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

  // The capacity's allowed range: null while it loads, or when you can't change it.
  let range = $state<{ id: string; value: CapacityRange | null } | null>(null);
  $effect(() => {
    const sid = set?.id;
    void set?.slotsUsed;
    if (!sid || !canRename) return;
    let gone = false;
    gql(SET_CAPACITY_RANGE, { id: sid })
      .then((r) => {
        if (!gone) range = { id: sid, value: r.setCapacityRange ?? null };
      })
      .catch(() => {});
    return () => (gone = true);
  });
  const capRange = $derived(range && range.id === set?.id ? range.value : null);
  function capacitySaved(n: number) {
    if (!set) return;
    if (set.kind === 'PERSONAL') set.capacityOverride = n;
    if (n === set.capacity) return;
    set.capacity = n;
    notify(`Capacity saved: ${count(n)} slots.`);
  }
  // Staff: take a personal set's override away, so its owner's roles decide the slots again.
  async function resetSlots() {
    if (!set || saving) return;
    saving = true;
    try {
      const r = await gql(UPDATE_SET, { id: set.id, input: { resetCapacity: true } });
      set.capacity = r.updateSet.capacity;
      set.capacityOverride = r.updateSet.capacityOverride ?? null;
      notify(`Back to the plan’s slots: ${count(set.capacity)}.`);
    } catch (err) {
      fail(err);
    } finally {
      saving = false;
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

  const taken = $derived(members.ids);
  const names = $derived(members.names);
  const full = $derived(set ? set.slotsUsed >= set.capacity : false);
  const kindName = $derived(set?.kind === 'GLOBAL' ? 'Global' : set?.kind === 'PERSONAL' ? 'Personal' : 'Channel');
  // Back to the sets of whoever owns it (yours, or the account you edit); staff on the global set go to Admin.
  const setsHome = $derived(
    set?.kind === 'GLOBAL'
      ? { href: '/admin', label: 'Admin' }
      : { href: sectionHref(set?.owner?.id ?? session.me?.id ?? '', 'sets'), label: owner || !set?.owner ? 'Your sets' : `${clean(set.owner.displayName)}’s sets` },
  );
  $effect(() => {
    if (editors.length) wantLooks(editors.slice(0, 10).map((e) => e.id));
  });

  const facts = $derived(
    set
      ? [
          { label: 'Kind', value: `${kindName} set` },
          { label: 'Emotes', key: 'cap', meter: { used: set.slotsUsed, capacity: set.capacity } },
          ...(set.owner ? [{ label: 'Owner', key: 'owner' }] : []),
          ...(showing.length ? [{ label: 'Shown on', value: showing.map((c) => `${platformName(c.platform)} ${c.login}`).join(', ') }] : []),
          ...(set.kind !== 'GLOBAL' && owner ? [{ label: 'Editors', value: editors.length, mono: true }] : []),
          ...(held.size ? [{ label: 'You may', value: permissionLabels([...held]), tone: 'muted' as const }] : []),
          { label: 'Id', value: set.id, mono: true },
        ]
      : [],
  );
</script>

<svelte:head><title>{L.cause ? problemTitle(L.cause, true, 'No set here') : `${set?.name ?? 'Set'} - cyex.tv`}</title></svelte:head>

{#snippet side()}
  {#if set}
    {#if canRename}
      <PanelBlock title="Name">
        <form class="row" onsubmit={rename}>
          <label class="sr-only" for="set-name">Name</label>
          <input id="set-name" class="ui-input" bind:value={name} maxlength="100" />
          <button class="ui-btn secondary" type="submit" disabled={saving || !name.trim() || name.trim() === set.name}>Rename</button>
        </form>
      </PanelBlock>
    {/if}
    {#if set.kind === 'PERSONAL'}
      <PanelBlock title="Slots">
        <p class="slots" data-testid="personal-slots">{slotsLine(set.slotsUsed, set.capacity)}</p>
        {@const note = slotsNote({ used: set.slotsUsed, capacity: set.capacity, override: set.capacityOverride ?? null })}
        {#if note}<p class="pb-hint">{note}</p>{/if}
        {#if canRename && capRange}
          <p class="pb-hint staff">Staff override</p>
          <CapacityForm setId={set.id} capacity={set.capacity} range={capRange} onsaved={capacitySaved} />
          {#if set.capacityOverride != null}
            <div class="pb-actions"><button class="ui-btn secondary" type="button" disabled={saving} onclick={resetSlots}>Use the plan’s slots</button></div>
          {/if}
        {/if}
      </PanelBlock>
    {:else if canRename && capRange}
      <PanelBlock title="Capacity">
        <CapacityForm setId={set.id} capacity={set.capacity} range={capRange} onsaved={capacitySaved} />
      </PanelBlock>
    {/if}
    {#if usable.length}
      <PanelBlock title={usable.length === 1 ? 'Use on my channel' : 'Use on a channel'}>
        <ul class="chans">
          {#each usable as c (channelKey(c))}
            {@const on = c.activeSetId === set?.id}
            <li>
              <Icon name={c.platform === 'TWITCH' ? 'twitch' : 'kick'} size={16} />
              <span class="chan"><bdi>{c.login}</bdi><span class="ui-hint">{platformName(c.platform)}</span></span>
              {#if on}
                <span class="live"><Icon name="check" size={15} />Shows this set</span>
              {:else}
                <button class="ui-btn secondary small" type="button" disabled={switching !== null} onclick={() => useOn(c)}
                  aria-label="Use this set on {platformName(c.platform)} {c.login}">{switching === channelKey(c) ? 'Switching…' : 'Use here'}</button>
              {/if}
            </li>
          {/each}
        </ul>
        <p class="pb-hint">Chat shows a channel’s active set. Picking this one replaces what the channel shows now.</p>
      </PanelBlock>
    {/if}
    <PanelBlock title="Facts">
      <FactRows label="Facts" items={facts}>
        {#snippet value(k)}
          {#if k === 'cap'}<span class="ui-mono"><Capacity used={set!.slotsUsed} capacity={set!.capacity} /></span>
          {:else if k === 'owner' && set!.owner}<a class="who" href="/users/{encodeURIComponent(set!.owner.id)}"><Avatar src={set!.owner.avatar} name={clean(set!.owner.displayName)} size={20} /><Name value={set!.owner.displayName} /></a>{/if}
        {/snippet}
      </FactRows>
    </PanelBlock>
    {#if owner && set.kind !== 'GLOBAL'}
      <PanelBlock title="Editors" count={editors.length || undefined} more={{ href: '/me/editors', label: 'Manage' }}>
        {#if editors.length}
          <ul class="eds">
            {#each editors.slice(0, 10) as e (e.id)}
              <li><Avatar src={e.avatar} name={clean(e.displayName)} size={24} /><a href="/users/{encodeURIComponent(e.id)}"><PaintedName name={e.displayName} paint={looks[e.id]?.paint} /></a></li>
            {/each}
          </ul>
          {#if editors.length > 10}<p class="pb-hint">And {count(editors.length - 10)} more on the Editors page.</p>{/if}
        {:else}
          <p class="pb-hint">Your editors work on your sets for you. Choose who, what they can do and which sets on the Editors page.</p>
        {/if}
      </PanelBlock>
    {/if}
    {#if set.kind !== 'GLOBAL' && (canDelete || setEditor)}
      <PanelBlock title={canDelete ? 'Delete set' : 'Stop editing'}>
        {#if canDelete}
          {#if shownOn.length}
            <p class="pb-hint">{shownOn.map((c) => `${platformName(c.platform)} ${c.login}`).join(' and ')} shows this set. Pick another active set in <a href="/me/channels">Channels</a> first.</p>
            <div class="pb-actions"><button class="ui-btn danger" type="button" disabled>Delete set</button></div>
          {:else}
            <p class="pb-hint">The emotes stay; only this set goes.</p>
            <div class="pb-actions"><Confirm label="Delete set" confirm="Click again to delete" busy={deleting} onconfirm={remove} /></div>
          {/if}
        {:else}
          <p class="pb-hint">You lose access to this set until the owner adds you again.</p>
          <div class="pb-actions"><Confirm label="Leave" confirm="Click again to leave" onconfirm={leave} /></div>
        {/if}
      </PanelBlock>
    {/if}
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
  <div class="wrap wait"><State kind="loading" title="Opening the set…" /></div>
{:else if set}
  <MeFrame title="Set" label="About this set" panel={side} narrow="after" fold={false} currentSet={set.id}>
    {#snippet head()}
      <PageHead name title={clean(set!.name)} back={setsHome}>
        {#snippet line()}
          {kindName} set · <Capacity used={set!.slotsUsed} capacity={set!.capacity} /> emotes
          {#if !owner && set!.owner}· by <Name value={set!.owner.displayName} />{/if}
        {/snippet}
        {#snippet actions()}
          <a class="ui-btn secondary" href="/sets/{encodeURIComponent(set!.id)}"><Icon name="arrow-right" size={15} />Set page</a>
        {/snippet}
      </PageHead>
    {/snippet}
    <SetEmotes setId={set.id} capacity={set.capacity} used={set.slotsUsed} {rev} onchange={load} personal={set.kind === 'PERSONAL'} />
    <AddEmotes setId={set.id} personal={set.kind === 'PERSONAL'} {taken} {names} {full} onadded={added} />
  </MeFrame>
{/if}
{/snippet}
</LoadGate>

<style>
  .wait { padding-top: var(--rail-gap); }
  .row { display: flex; gap: 8px; }
  .slots { margin: 0; font-weight: 600; font-variant-numeric: tabular-nums; }
  .staff { margin-top: 10px; font-weight: 600; }
  .row input { min-width: 0; }
  .chans { list-style: none; margin: 0; padding: 0; display: grid; }
  .chans > li { display: flex; align-items: center; gap: 10px; min-height: 48px; }
  .chans > li + li { border-top: 1px solid var(--border); }
  .chans :global(svg) { flex: none; color: var(--dim); }
  .chan { display: grid; flex: 1; min-width: 0; line-height: 1.3; }
  .chan bdi { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .live { display: inline-flex; align-items: center; gap: 6px; font-size: var(--t-small); font-weight: 600; white-space: nowrap; }
  .live :global(svg) { color: var(--accent-text); }
  .who { display: inline-flex; align-items: center; gap: 8px; min-width: 0; color: var(--text); }
  .eds { list-style: none; margin: 0; padding: 0; display: grid; }
  .eds li { display: flex; align-items: center; gap: 10px; height: 40px; min-width: 0; font-size: var(--t-small); font-weight: 600; }
  .eds li + li { border-top: 1px solid var(--border); }
  .eds a { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--text); }
</style>
