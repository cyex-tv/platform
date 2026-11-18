<script lang="ts">
  // A picked file before it is uploaded, as cyex.tv will show it: 1x, 2x and 4x at their real pixel
  // sizes (1x is 32 px tall, as the worker makes it) on a dark or light ground, or in two chat lines
  // at chat size (28 px). A zero-width emote is drawn over a stand-in emote, as chat stacks it.
  import Segmented from '$lib/components/Segmented.svelte';

  type Props = { src: string; width: number; height: number; name: string; zeroWidth?: boolean };
  let { src, width, height, name, zeroWidth = false }: Props = $props();

  type Ground = 'dark' | 'light' | 'chat';
  let ground = $state<Ground>('dark');
  const ratio = $derived(Math.min(4, Math.max(0.25, width / Math.max(1, height))));
  const at = (h: number) => ({ w: Math.round(h * ratio), h });
</script>

<div class="lp">
  <Segmented label="Preview background" size="s" options={[{ value: 'dark', label: 'Dark', icon: 'moon' }, { value: 'light', label: 'Light', icon: 'sun' }, { value: 'chat', label: 'Chat', icon: 'message-square' }]}
    value={ground} onchange={(v) => (ground = v)} />
  {#if ground === 'chat'}
    <div class="chat" aria-label="{name} in chat">
      <p><b class="u1">lumen_fox</b>: perfect {@render inChat()}</p>
      <p><b class="u2">quietoak</b>: {@render inChat()} {@render inChat()}</p>
    </div>
  {:else}
    <div class="stage {ground}">
      {#each [1, 2, 4] as k}
        {@const s = at(32 * k)}
        <figure>
          <img {src} width={s.w} height={s.h} alt={k === 4 ? name : ''} />
          <figcaption>{k}x</figcaption>
        </figure>
      {/each}
    </div>
  {/if}
</div>

{#snippet inChat()}
  {@const s = at(28)}
  {#if zeroWidth}
    <span class="stack"><span class="base" aria-hidden="true"></span><img {src} width={s.w} height={s.h} alt={name} /></span>
  {:else}
    <img class="em" {src} width={s.w} height={s.h} alt={name} />
  {/if}
{/snippet}

<style>
  .lp { display: grid; justify-items: start; gap: 10px; min-width: 0; }
  .stage { display: flex; align-items: flex-end; gap: 24px; width: 100%; min-height: 176px; padding: 20px; overflow-x: auto;
    border: 1px solid var(--border); border-radius: var(--radius-card); --ink: var(--stage-dark-ink); background: var(--stage-dark); }
  .light { --ink: var(--stage-light-ink); background: var(--stage-light); }
  figure { flex: none; display: grid; justify-items: center; gap: 8px; margin: 0; }
  figure img { display: block; max-width: none; image-rendering: auto; }
  figcaption { font-size: var(--t-fine); color: var(--ink); font-variant-numeric: tabular-nums; }
  .chat { width: 100%; padding: 12px 14px; border: 1px solid var(--border); border-radius: var(--radius-card); background: var(--bg);
    font-size: 13px; line-height: 28px; color: var(--text); }
  .chat p { margin: 0; overflow-wrap: anywhere; }
  .u1 { color: var(--accent-text); }
  .u2 { color: var(--green); }
  .em, .stack { vertical-align: middle; }
  .stack { position: relative; display: inline-grid; place-items: center; }
  .stack > * { grid-area: 1 / 1; }
  .stack img { position: relative; }
  .base { width: 28px; height: 28px; border-radius: 50%; background: var(--border-strong); }
</style>
