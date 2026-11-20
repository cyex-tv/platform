<script lang="ts">
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

<div class="narrow">
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
</div>

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
</style>
