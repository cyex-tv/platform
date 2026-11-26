<script lang="ts">
  // The Cosmetics tab as everyone sees it: the paint and badge the account shows next to its name
  // in Twitch and Kick chat. Only what is already public (it is drawn on the name everywhere):
  // never what else the account owns, since when, or how it came. The owner and editors with
  // PROFILE get the picker (CosmeticsPicker) in this tab instead.
  import BadgeIcon from '$lib/cosmetics/BadgeIcon.svelte';
  import BadgeOriginal from '$lib/cosmetics/BadgeOriginal.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { looks, NO_LOOK, seedLook, wantLooks, type Look } from '$lib/cosmetics/store.svelte';
  import { clean, iso } from '$lib/format';
  import PageHead from '$lib/tool/PageHead.svelte';

  type Props = { accountId: string; name: string; seed?: Look | null };
  let { accountId, name, seed = null }: Props = $props();

  $effect(() => {
    if (seed) seedLook(accountId, seed);
    else wantLooks([accountId]);
  });
  const look = $derived(looks[accountId] ?? seed ?? NO_LOOK);
  const who = $derived(iso(clean(name)));
</script>

<PageHead title="Cosmetics" line={`What ${who} shows next to their name in Twitch and Kick chat, for everyone with cyex.tv.`} />

<div class="shown">
  <section class="panel" aria-labelledby="sc-paint">
    <h2 id="sc-paint">Paint</h2>
    {#if look.paint}
      <p class="value paint"><PaintedName name={clean(name)} paint={look.paint} /></p>
      <p class="sub"><bdi>{clean(look.paint.name)}</bdi></p>
    {:else}
      <p class="value none">No paint</p>
    {/if}
  </section>
  <section class="panel" aria-labelledby="sc-badge">
    <h2 id="sc-badge">Badge</h2>
    {#if look.badge}
      <p class="value"><BadgeIcon badge={look.badge} size={32} /><bdi>{clean(look.badge.name)}</bdi></p>
      <BadgeOriginal badge={look.badge} caption />
    {:else}
      <p class="value none">No badge</p>
    {/if}
  </section>
</div>

<style>
  .shown { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(280px, 100%), 1fr)); gap: 12px; }
  .panel { display: grid; align-content: start; gap: 8px; min-height: 112px; padding: 14px 16px 16px; background: var(--surface);
    border: 1px solid var(--border); border-radius: var(--radius-card); }
  h2 { margin: 0; font-size: 15px; font-weight: 600; }
  .value { display: flex; align-items: center; gap: 10px; min-height: 36px; margin: 0; min-width: 0; font-size: var(--t-ui); font-weight: 600; }
  .value > bdi { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .value.paint { font-size: 20px; overflow-wrap: anywhere; }
  .none { color: var(--muted); font-weight: 500; }
  .sub { margin: 0; font-size: var(--t-fine); color: var(--muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
