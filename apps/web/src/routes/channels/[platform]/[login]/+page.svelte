<script lang="ts">
  import EmoteGrid from '$lib/components/EmoteGrid.svelte';
  import { count } from '$lib/format';

  let { data } = $props();
  const c = $derived(data.channel);
  const platform = $derived(c.platform === 'TWITCH' ? 'Twitch' : 'Kick');
  const items = $derived(data.set?.emotes.map((m) => ({ ...m.emote, shownName: m.name })) ?? []);
</script>

<svelte:head>
  <title>{c.account.displayName} - cyex.tv</title>
  <meta name="description" content="The cyex.tv emotes in {c.account.displayName}'s {platform} chat." />
</svelte:head>

<div class="page">
  <header class="page-head">
    <h1>{c.account.displayName}</h1>
    <p class="lead">
      {#if data.set}
        {count(items.length)} cyex.tv {items.length === 1 ? 'emote' : 'emotes'} in this {platform} chat.
      {:else}
        This {platform} channel hasn't picked a set yet.
      {/if}
      {#if !c.allowPersonalEmotes}Personal emotes are off here.{/if}
    </p>
  </header>
  {#if data.set}
    <EmoteGrid {items} empty="The set is empty." />
  {/if}
</div>
