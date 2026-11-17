<script lang="ts">
  // Badges and name paints you own and wear in chat. Not built yet: the page is here so the menu
  // and the rail already have their place, and it says plainly what is coming.
  import State from '$lib/components/State.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
</script>

<svelte:head><title>Cosmetics - cyex.tv</title></svelte:head>

<PageHead title="Cosmetics" line="Badges and name paints that show next to your name in chat." />

<div class="grid">
  <section class="ui-card" aria-labelledby="badges-h">
    <h2 id="badges-h">Badges</h2>
    <p class="hint">A small image before your name in Twitch and Kick chat, for everyone with the extension or cyex deck.</p>
  </section>
  <section class="ui-card" aria-labelledby="paints-h">
    <h2 id="paints-h">Name paints</h2>
    <p class="hint">A colour or pattern on your name in chat, in place of your usual name colour.</p>
  </section>
</div>

<State title="Coming soon">
  You'll pick the badge and paint you wear here, and see the ones you own.
</State>

<style>
  .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; margin-bottom: 16px; }
  .grid h2 { font-size: 15px; margin: 0 0 6px; }
  .hint { margin: 0; color: var(--muted); font-size: var(--t-ui); line-height: 1.5; }
  @media (max-width: 720px) { .grid { grid-template-columns: minmax(0, 1fr); } }
</style>
