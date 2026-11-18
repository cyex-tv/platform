<script lang="ts">
  // The emotes chat shows right now: each channel's active set (Twitch and Kick can differ), as
  // the owner or an editor sees them. A switch picks the channel when there are two.
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import State from '$lib/components/State.svelte';
  import { count } from '$lib/format';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { libraryOf } from '../data.svelte';

  let { accountId, mine }: { accountId: string; mine: boolean } = $props();

  const L = loader(() => libraryOf(accountId, mine));
  let pick = $state<string>('');
</script>

<LoadGate source={L} label="Loading the active emotes…">
  {#snippet children(lib)}
    {@const chans = lib.channels}
    {@const chan = chans.find((c) => c.platform === pick) ?? chans[0]}
    {@const set = chan ? lib.sets.find((s) => s.id === chan.activeSetId) : undefined}
    <PageHead title="Active emotes">
      {#snippet line()}
        {#if set}
          <bdi>{set.name}</bdi> · {count(set.emotes.length)} of {count(set.capacity)} · shown in {chan?.platform === 'KICK' ? 'Kick' : 'Twitch'} chat
        {:else}
          What chat shows from cyex.tv.
        {/if}
      {/snippet}
      {#snippet actions()}
        {#if chans.length > 1}
          <Segmented label="Channel" size="s" options={chans.map((c) => ({ value: c.platform, label: c.platform === 'KICK' ? 'Kick' : 'Twitch', icon: c.platform === 'KICK' ? ('kick' as const) : ('twitch' as const) }))}
            value={chan?.platform ?? 'TWITCH'} onchange={(v) => (pick = v)} />
        {/if}
        {#if set}<a class="ui-btn secondary" href="/me/sets/{set.id}"><Icon name="pencil" size={15} />Edit set</a>{/if}
      {/snippet}
    </PageHead>
    {#if !chans.length}
      <State title="No channel yet.">
        A channel appears once {mine ? 'you sign' : 'they sign'} in with Twitch or Kick.
      </State>
    {:else if !set}
      <State title="No active set.">
        Pick the set this channel shows, and its emotes appear here.
        {#snippet actions()}<a class="ui-btn primary" href="/me/channels">Pick a set</a>{/snippet}
      </State>
    {:else if !set.emotes.length}
      <State title="This set is empty.">
        Add emotes to it and they show in chat right away.
        {#snippet actions()}<a class="ui-btn primary" href="/me/sets/{set.id}">Add emotes</a>{/snippet}
      </State>
    {:else}
      <EmoteGrid items={set.emotes.map((m) => ({ ...m.emote, shownName: m.name !== m.emote.name ? m.name : undefined }))} eager={12} label="Active emotes" />
    {/if}
  {/snippet}
</LoadGate>
