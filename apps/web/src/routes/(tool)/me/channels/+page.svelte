<script lang="ts">
  import Name from '$lib/components/Name.svelte';
  import { iso } from '$lib/format';
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
  let saving = $state<string | null>(null);

  const channelSets = $derived(sets.filter((s) => s.kind === 'CHANNEL' && s.owner?.id === session.me?.id));

  async function update(ch: Channel, input: { activeSetId?: string; allowPersonalEmotes?: boolean }) {
    const key = `${ch.platform}:${ch.platformChannelId}`;
    saving = key;
    try {
      const r = await gql(UPDATE_CHANNEL, { platform: ch.platform, id: ch.platformChannelId, input });
      ch.activeSetId = r.updateChannelSettings.activeSetId;
      ch.allowPersonalEmotes = r.updateChannelSettings.allowPersonalEmotes;
      notify('Saved. Chat picks it up within seconds.');
    } catch (err) {
      fail(err);
    } finally {
      saving = null;
    }
  }

  const platformName = (p: string) => (p === 'TWITCH' ? 'Twitch' : 'Kick');
</script>

<svelte:head><title>Channels - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Channels" line="Your channel on each platform you signed in with. Pick the set its chat shows." />

  <LoadGate source={L} label="Loading your channels…">
  {#snippet children(data)}
  {@const channels = data.myChannels}
  {#if channels.length === 0}
    <State title="No channels yet">Sign in with Twitch or Kick and yours appears here.</State>
  {:else}
    <div class="ui-stack">
      {#each channels as ch (ch.platform + ch.platformChannelId)}
        {@const busy = saving === `${ch.platform}:${ch.platformChannelId}`}
        <section class="ui-panel" data-platform={ch.platform.toLowerCase()}>
          <header>
            <span class="dot" aria-hidden="true"></span>
            <h2><Name value={ch.login} /></h2>
            <span class="ui-hint">{platformName(ch.platform)}</span>
            <a class="ui-btn ghost small" href="/channels/{ch.platform.toLowerCase()}/{encodeURIComponent(ch.login)}">Public page</a>
          </header>
          <div class="body">
            <label class="ui-row">
              <div><strong>Active set</strong><span class="ui-hint">What everyone in your chat sees.</span></div>
              <select class="ui-input pick" disabled={busy} value={ch.activeSetId ?? ''}
                onchange={(e) => update(ch, { activeSetId: e.currentTarget.value })}>
                <option value="">None</option>
                {#each channelSets as s}<option value={s.id}>{iso(s.name)}</option>{/each}
              </select>
            </label>
            {#if channelSets.length === 0}
              <p class="ui-hint">You have no channel sets yet. <a href="/me/sets">Make one</a>.</p>
            {/if}
            <label class="ui-row">
              <div><strong>Personal emotes</strong><span class="ui-hint">Show chatters’ personal sets in your chat.</span></div>
              <input class="ui-switch" type="checkbox" disabled={busy} checked={ch.allowPersonalEmotes}
                onchange={(e) => update(ch, { allowPersonalEmotes: e.currentTarget.checked })} />
            </label>
          </div>
        </section>
      {/each}
    </div>
  {/if}
  {/snippet}
  </LoadGate>
</div>

<style>
  .body { padding: 2px 14px 8px; }
  .ui-row .ui-hint { display: block; }
  .pick { width: auto; min-width: 0; max-width: 100%; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--twitch-logo); flex: none; }
  [data-platform='kick'] .dot { background: var(--kick); }
  header h2 { flex: 0 1 auto; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  header .ui-hint { flex: 1; }
</style>
