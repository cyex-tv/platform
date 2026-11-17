<script lang="ts">
  // The emote at 1x to 4x, each at its real pixel size, on a dark, light or checkered ground,
  // or in a Twitch-style and a Kick-style chat message (zero-width ones layered over a global
  // emote). Animated emotes can be paused (the still first frame takes over); under reduced
  // motion they start paused, and before the script runs a <picture> source keeps them still.
  import ChatPreview from '$lib/components/ChatPreview.svelte';
  import IconButton from '$lib/components/IconButton.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import { hasStill, ready } from '$lib/emote';
  import { chatLines, scaleFiles, type ChatPool, type PageEmote } from './emote-page';

  let { emote: e, pool = [] }: { emote: PageEmote; pool?: ChatPool } = $props();

  type Ground = 'dark' | 'light' | 'checker' | 'chat';
  const grounds: { value: Ground; label: string }[] = [
    { value: 'dark', label: 'Dark' },
    { value: 'light', label: 'Light' },
    { value: 'checker', label: 'Checker' },
    { value: 'chat', label: 'Chat' },
  ];
  let ground = $state<Ground>('dark');

  // null: follow the reader's motion setting (decided by CSS before the script runs).
  let paused = $state<boolean | null>(null);
  let reduced = $state(false);
  $effect(() => {
    const mq = matchMedia('(prefers-reduced-motion: reduce)');
    reduced = mq.matches;
    const on = () => (reduced = mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });

  const canPause = $derived(hasStill(e));
  const still = $derived(paused ?? reduced);
  const files = $derived(scaleFiles(e.files));
  const lines = $derived(chatLines(e, pool));
  const stillName = (scale: number) => `${e.host}/${scale}x_static.webp`;
  const moving = (name: string) => `${e.host}/${name}`;
</script>

<section class="preview" aria-label="Preview">
  <div class="tools">
    <Segmented label="Background" size="s" options={grounds} value={ground} onchange={(v) => (ground = v)} />
    {#if canPause}
      <IconButton
        icon={still ? 'play' : 'pause'}
        label={still ? 'Play' : 'Pause'}
        variant="outline"
        size="s"
        tip="top"
        onclick={() => (paused = !still)}
      />
    {/if}
  </div>

  <div class="stage {ground}" class:chatting={ground === 'chat'}>
    {#if !ready(e)}
      <p class="wait">The files are still being made. This takes a minute or so.</p>
    {:else if ground === 'chat'}
      <div class="chats">
        <ChatPreview platform="twitch" {lines} play={!still} label="{e.name} in a Twitch-style chat" />
        <ChatPreview platform="kick" {lines} play={!still} label="{e.name} in a Kick-style chat" />
      </div>
    {:else}
      <div class="sizes">
        {#each files as f (f.name)}
          <figure>
            <span class="cell" style:width="{f.width}px" style:height="{f.height}px">
              {#if canPause && paused === null}
                <picture>
                  <source media="(prefers-reduced-motion: reduce)" srcset={stillName(f.scale)} />
                  <img src={moving(f.name)} width={f.width} height={f.height} alt={f.scale === 4 ? e.name : ''} />
                </picture>
              {:else}
                <img
                  src={canPause && still ? stillName(f.scale) : moving(f.name)}
                  width={f.width}
                  height={f.height}
                  alt={f.scale === 4 ? e.name : ''}
                />
              {/if}
            </span>
            <figcaption><span class="x">{f.scale}x</span> {f.width} × {f.height}</figcaption>
          </figure>
        {/each}
      </div>
    {/if}
  </div>
</section>

<style>
  p { margin: 0; }
  .preview { display: grid; gap: 12px; min-width: 0; }
  .tools { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  .stage { position: relative; display: grid; place-items: center; min-height: clamp(240px, 26vw, 340px);
    padding: clamp(24px, 4vw, 48px); border: 1px solid var(--border); border-radius: var(--radius-l);
    overflow: hidden; --ink: var(--stage-dark-ink); background: var(--stage-dark); }
  .light { --ink: var(--stage-light-ink); background: var(--stage-light); }
  .checker { --ink: var(--stage-light-ink); background-color: var(--checker-a);
    background-image: repeating-conic-gradient(var(--checker-b) 0 25%, transparent 0 50%);
    background-size: var(--checker-size) var(--checker-size); }
  .chatting { padding: clamp(12px, 2vw, 24px); background: var(--surface); place-items: stretch; align-content: center; }

  .sizes { display: flex; flex-wrap: wrap; align-items: flex-end; justify-content: center; gap: clamp(20px, 3vw, 40px); }
  figure { margin: 0; display: grid; justify-items: center; gap: 12px; }
  .cell { display: grid; place-items: center; }
  .cell img { display: block; max-width: none; }
  figcaption { font-size: var(--t-fine); color: var(--ink); font-variant-numeric: tabular-nums; white-space: nowrap; }
  .x { font-weight: 600; }
  .wait { color: var(--ink); font-size: var(--t-small); text-align: center; max-width: 32ch; }

  .chats { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  @media (max-width: 720px) { .chats { grid-template-columns: minmax(0, 1fr); } }
</style>
