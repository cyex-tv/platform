<script lang="ts">
  import { gql } from '$lib/api';
  import type { MyChannelsQuery } from '$lib/gql/graphql';
  import { MY_CHANNELS, UPDATE_CHANNEL } from '$lib/queries';
  import { session } from '$lib/tool/session.svelte';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Channel = MyChannelsQuery['myChannels'][number];
  let channels = $state<Channel[] | null>(null);
  let sets = $state<MyChannelsQuery['mySets']>([]);
  let saving = $state<string | null>(null);

  const channelSets = $derived(sets.filter((s) => s.kind === 'CHANNEL' && s.owner?.id === session.me?.id));

  $effect(() => {
    gql(MY_CHANNELS, {})
      .then((r) => {
        channels = r.myChannels;
        sets = r.mySets;
      })
      .catch(fail);
  });

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
  <div class="ui-head">
    <div>
      <h1>Channels</h1>
      <p>Your channel on each platform you signed in with. Pick the set its chat shows.</p>
    </div>
  </div>

  {#if !channels}
    <div class="ui-empty" aria-busy="true"><span class="ui-spinner" aria-hidden="true"></span></div>
  {:else if channels.length === 0}
    <div class="ui-empty"><p>No channels yet. Sign in with Twitch or Kick and yours appears here.</p></div>
  {:else}
    <div class="ui-stack">
      {#each channels as ch (ch.platform + ch.platformChannelId)}
        {@const busy = saving === `${ch.platform}:${ch.platformChannelId}`}
        <section class="ui-panel" data-platform={ch.platform.toLowerCase()}>
          <header>
            <span class="dot" aria-hidden="true"></span>
            <h2>{ch.login}</h2>
            <span class="ui-hint">{platformName(ch.platform)}</span>
            <a class="ui-btn ghost small" href="/channels/{ch.platform.toLowerCase()}/{ch.login}">Public page</a>
          </header>
          <div class="body">
            <label class="ui-row">
              <div><strong>Active set</strong><span class="ui-hint">What everyone in your chat sees.</span></div>
              <select class="ui-input pick" disabled={busy} value={ch.activeSetId ?? ''}
                onchange={(e) => update(ch, { activeSetId: e.currentTarget.value })}>
                <option value="">None</option>
                {#each channelSets as s}<option value={s.id}>{s.name}</option>{/each}
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
</div>

<style>
  .body { padding: 2px 14px 8px; }
  .ui-row .ui-hint { display: block; }
  .pick { width: auto; min-width: 180px; max-width: 50%; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--twitch-logo); flex: none; }
  [data-platform='kick'] .dot { background: var(--kick); }
  header h2 { flex: none; }
  header .ui-hint { flex: 1; }
</style>
