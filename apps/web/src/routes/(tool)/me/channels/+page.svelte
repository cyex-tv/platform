<script lang="ts">
  // Channels: your channel on each platform you signed in with, and the channels of accounts you
  // edit, each with the set its chat shows and whether chatters' personal emotes show. In your
  // profile's frame (MeFrame); the panel sums them up and links to what they depend on.
  import Icon from '$lib/components/Icon.svelte';
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import { count } from '$lib/format';
  import { sectionHref } from '$lib/profile/rows';
  import Name from '$lib/components/Name.svelte';
  import { clean, iso } from '$lib/format';
  import { ownerSetsHref, permissionLabels } from '$lib/tool/editors';
  import { gql } from '$lib/api';
  import type { MyChannelsQuery } from '$lib/gql/graphql';
  import { MY_CHANNELS, UPDATE_CHANNEL } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import State from '$lib/components/State.svelte';

  type Channel = MyChannelsQuery['myChannels'][number];
  const L = loader(({ fetch }) => gql(MY_CHANNELS, {}, fetch));
  const sets = $derived(L.data?.mySets ?? []);
  /** Channels with a save on its way: one at a time per channel, however fast the controls are used. */
  let saving = $state(new Set<string>());

  const channelSets = $derived(sets.filter((s) => s.kind === 'CHANNEL' && s.owner?.id === session.me?.id));
  // Account editing (ADR 0014): channels of accounts you edit with the channels permission (which
  // the sets permission brings), by owner.
  const managed = $derived(
    (L.data?.editorOf ?? [])
      .filter((g) => g.permissions.includes('CHANNELS') || g.permissions.includes('SETS'))
      .map((g) => ({
        ownerId: g.account.id,
        permissions: g.permissions,
        channels: (L.data?.editorChannels ?? []).filter((c) => c.account.id === g.account.id),
        sets: (L.data?.editorSets ?? []).filter((s) => s.kind === 'CHANNEL' && s.owner?.id === g.account.id),
      }))
      .filter((g) => g.channels.length),
  );

  async function update(ch: Channel, input: { activeSetId?: string; allowPersonalEmotes?: boolean }, revert: () => void) {
    const key = `${ch.platform}:${ch.platformChannelId}`;
    if (saving.has(key)) return revert();
    saving = new Set([...saving, key]);
    try {
      const r = await gql(UPDATE_CHANNEL, { platform: ch.platform, id: ch.platformChannelId, input });
      ch.activeSetId = r.updateChannelSettings.activeSetId;
      ch.allowPersonalEmotes = r.updateChannelSettings.allowPersonalEmotes;
      notify('Saved. Chat picks it up within seconds.');
    } catch (err) {
      // The control goes back to what is saved, not the value that failed.
      revert();
      fail(err);
    } finally {
      const left = new Set(saving);
      left.delete(key);
      saving = left;
    }
  }

  const platformName = (p: string) => (p === 'TWITCH' ? 'Twitch' : 'Kick');
  const own = $derived(L.data?.myChannels ?? []);
  const setName = (id: string | null | undefined) => (id ? [...sets, ...(L.data?.editorSets ?? [])].find((s) => s.id === id)?.name : undefined);
  const showing = $derived([...own, ...managed.flatMap((g) => g.channels)]);
</script>

<svelte:head><title>Channels - cyex.tv</title></svelte:head>

{#snippet panel(ch: Channel, options: { id: string; name: string }[], owner?: string)}
  {@const busy = saving.has(`${ch.platform}:${ch.platformChannelId}`)}
    <section class="ui-panel" data-platform={ch.platform.toLowerCase()}>
      <header>
        <span class="dot" aria-hidden="true"></span>
        <h2><Name value={ch.login} /></h2>
        <span class="ui-hint" title={owner ? `${platformName(ch.platform)} · ${clean(owner)}’s channel` : undefined}>{platformName(ch.platform)}{#if owner}{' '}· <bdi>{clean(owner)}</bdi>’s channel{/if}</span>
        <a class="ui-btn ghost small" href="/channels/{ch.platform.toLowerCase()}/{encodeURIComponent(ch.login)}">Public page</a>
      </header>
      <div class="body">
        <label class="ui-row">
          <div><strong>Active set</strong><span class="ui-hint">What everyone in {owner ? 'its' : 'your'} chat sees.</span></div>
          <select class="ui-input pick" aria-disabled={busy} value={ch.activeSetId ?? ''}
            onchange={(e) => { const el = e.currentTarget; update(ch, { activeSetId: el.value }, () => (el.value = ch.activeSetId ?? '')); }}>
            <option value="">None</option>
            {#each options as s}<option value={s.id}>{iso(s.name)}</option>{/each}
          </select>
        </label>
        {#if options.length === 0}
          <p class="ui-hint">{owner ? `${iso(owner)} has` : 'You have'} no channel sets yet. <a href={owner ? ownerSetsHref(ch.account.id) : '/me/sets'}>Make one</a>.</p>
        {/if}
        <label class="ui-row">
          <div><strong>Personal emotes</strong><span class="ui-hint">Show chatters’ personal sets in {owner ? 'its' : 'your'} chat.</span></div>
          <input class="ui-switch" type="checkbox" aria-disabled={busy} checked={ch.allowPersonalEmotes}
            onchange={(e) => { const el = e.currentTarget; update(ch, { allowPersonalEmotes: el.checked }, () => (el.checked = ch.allowPersonalEmotes)); }} />
        </label>
      </div>
    </section>
{/snippet}

{#snippet side()}
  <PanelBlock title="In numbers">
    <dl class="kv">
      <dt>Your channels</dt><dd>{L.data ? count(own.length) : '–'}</dd>
      <dt>Channels you edit</dt><dd>{L.data ? count(managed.reduce((n, g) => n + g.channels.length, 0)) : '–'}</dd>
      <dt>Your channel sets</dt><dd>{L.data ? count(channelSets.length) : '–'}</dd>
    </dl>
  </PanelBlock>
  {#if showing.length}
    <PanelBlock title="Chat shows">
      <ul class="shows">
        {#each showing as ch (ch.platform + ch.platformChannelId)}
          {@const set = setName(ch.activeSetId)}
          <li>
            <Icon name={ch.platform === 'KICK' ? 'kick' : 'twitch'} size={16} />
            <span class="who"><bdi>{clean(ch.login)}</bdi></span>
            <span class="what">{#if set && ch.activeSetId}<a href="/sets/{encodeURIComponent(ch.activeSetId)}"><bdi>{clean(set)}</bdi></a>{:else}No set{/if}</span>
          </li>
        {/each}
      </ul>
    </PanelBlock>
  {/if}
  <PanelBlock title="Related">
    <p class="pb-hint">A change reaches chat within seconds. Connect another platform in Settings to add its channel.</p>
    <div class="pb-actions">
      <a class="ui-btn secondary" href="/me/settings/account#conn-h"><Icon name="link" size={15} />Connections</a>
      <div class="pair">
        <a class="ui-btn secondary" href={sectionHref(session.me?.id ?? '', 'sets')}><Icon name="folder" size={15} />Your sets</a>
        <a class="ui-btn secondary" href="/me/editors"><Icon name="circle-user" size={15} />Editors</a>
      </div>
    </div>
  </PanelBlock>
{/snippet}

<MeFrame title="Channels" label="About your channels" panel={side}>
  <PageHead title="Channels" line="Your channel on each platform you signed in with. Pick the set its chat shows." />

  <LoadGate source={L} label="Loading your channels…">
  {#snippet children(data)}
  {@const channels = data.myChannels}
  {#if channels.length === 0 && managed.length === 0}
    <State title="No channels yet">Sign in with Twitch or Kick and yours appears here.</State>
  {:else}
    <div class="ui-stack">
      {#each channels as ch (ch.platform + ch.platformChannelId)}
        {@render panel(ch, channelSets)}
      {/each}
      {#each managed as g (g.ownerId)}
        <h2 class="group" id="of-{g.ownerId}">
          <bdi>{clean(g.channels[0].account.displayName)}</bdi>
          <span class="ui-hint">You edit this account: {permissionLabels(g.permissions)}</span>
        </h2>
        {#each g.channels as ch (ch.platform + ch.platformChannelId)}
          {@render panel(ch, g.sets, g.channels[0].account.displayName)}
        {/each}
      {/each}
    </div>
  {/if}
  {/snippet}
  </LoadGate>
</MeFrame>

<style>
  .body { padding: 2px 14px 8px; }
  /* Busy keeps focus (a disabled control would drop it): dim it instead. */
  [aria-disabled='true'] { opacity: .55; cursor: progress; }
  .ui-row .ui-hint { display: block; }
  .pick { width: auto; min-width: 0; max-width: min(100%, 260px); text-overflow: ellipsis; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--twitch-logo); flex: none; }
  [data-platform='kick'] .dot { background: var(--kick); }
  header h2 { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  header .ui-hint { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .group { display: grid; gap: 2px; margin: 12px 0 -6px; font-size: var(--t-ui); font-weight: 600; scroll-margin-top: 80px; }
  .group .ui-hint { font-weight: 400; }
  .shows { list-style: none; margin: 0; padding: 0; display: grid; }
  .shows li { display: flex; align-items: center; gap: 10px; height: 40px; min-width: 0; font-size: var(--t-small); }
  .shows li + li { border-top: 1px solid var(--border); }
  .shows :global(svg) { flex: none; color: var(--dim); }
  .who { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .what { flex: none; max-width: 55%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); }
  .what a { color: var(--muted); text-decoration: underline; text-decoration-color: var(--border-strong); text-underline-offset: 3px; }
  .ui-stack { max-width: 760px; }
</style>
