<script lang="ts">
  // Shown instead of the channel page when the channel isn't on cyex.tv (or the API can't be reached).
  import { invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { openPalette } from '$lib/components/palette.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import { causeOfStatus, problemTitle } from '$lib/components/problem';
  import State from '$lib/components/State.svelte';
  import { clean } from '$lib/format';
  import ColumnLayout from '$lib/shell/ColumnLayout.svelte';

  const missing = $derived(page.status === 404);
  const known = $derived(page.params.platform === 'kick' || page.params.platform === 'twitch');
  const platform = $derived(page.params.platform === 'kick' ? 'Kick' : 'Twitch');
  // A login is user input: cleaned, and cut so a 500-character address stays a sentence.
  const login = $derived.by(() => {
    const l = [...clean(page.params.login ?? '')];
    return l.length > 40 ? `${l.slice(0, 40).join('')}…` : l.join('');
  });
</script>

<svelte:head>
  <title>{missing && known ? 'Channel not found - cyex.tv' : problemTitle(causeOfStatus(page.status))}</title>
</svelte:head>

<ColumnLayout top>
  {#if missing && known}
    <State level={1} role="status">
      {#snippet heading()}<bdi>{login}</bdi> isn’t on cyex.tv yet.{/snippet}
      Nobody has set up a {platform} channel called <bdi>{login}</bdi> here. A channel shows up once its owner signs in
      with {platform} and picks an emote set. Check the spelling, or look for another channel.
      {#snippet actions()}
        <button class="ui-btn primary" type="button" onclick={() => openPalette(login)}>Search channels</button>
        <a class="ui-btn secondary" href="/search">Browse emotes</a>
      {/snippet}
    </State>
  {:else}
    <Problem cause={causeOfStatus(page.status)} level={1} onretry={() => invalidateAll()} />
  {/if}
</ColumnLayout>
