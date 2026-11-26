<script lang="ts" module>
  import type { Picture } from '$lib/emote';
  import type { Look } from '$lib/cosmetics/store.svelte';

  export type ChatEmote = Picture & { name: string; zeroWidth?: boolean };
  export type ChatPart = { text: string } | { emote: ChatEmote };
  /** `look`: a badge and paint to draw the name with (a real account's, never a made-up viewer's). */
  export type ChatLine = { user: string; parts: ChatPart[]; look?: Look };
</script>

<script lang="ts">
  import { reducedMotion } from '$lib/media.svelte';
  // A neutral chat frame, labelled "Twitch" or "Kick" (no logos, no platform colours), with
  // made-up viewers and emotes inline at chat size (28px tall; wide ones wider; zero-width ones
  // stacked on the emote before them). With `live`, a new line arrives every ~2.5s, cycling
  // through `lines`; it pauses off screen and in a hidden tab, and never runs under reduced motion.
  import { flip } from 'svelte/animate';
  import { fly } from 'svelte/transition';
  import { hasStill, pictureFor, ready, size } from '$lib/emote';
  import { display } from '$lib/dpr.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { clean } from '$lib/format';
  import { dur } from '$lib/motion';

  type Props = {
    platform: 'twitch' | 'kick';
    lines: ChatLine[];
    live?: boolean;
    /** Lines in view at once in live mode (the frame keeps a fixed height). */
    visible?: number;
    interval?: number;
    /** Animated emotes play (as in a real chat). Off, or under reduced motion, they stay still. */
    play?: boolean;
    label?: string;
  };
  let { platform, lines, live = false, visible = 6, interval = 2500, play = true, label }: Props = $props();

  type Piece = { kind: 'text'; text: string } | { kind: 'stack'; emotes: ChatEmote[] };
  type Shown = { key: number; user: string; look?: Look; pieces: Piece[] };

  // Zero-width emotes join the stack of the emote before them; text keeps its spaces.
  function pieces(parts: ChatPart[]): Piece[] {
    const out: Piece[] = [];
    for (const p of parts) {
      if ('text' in p) {
        out.push({ kind: 'text', text: p.text });
        continue;
      }
      const last = out[out.length - 1];
      const prev = last?.kind === 'stack' ? last : out[out.length - 2]?.kind === 'stack' && last?.kind === 'text' && !last.text.trim() ? out[out.length - 2] : null;
      if (p.emote.zeroWidth && prev && prev.kind === 'stack') {
        if (last !== prev) out.pop(); // drop the blank between an emote and its overlay
        prev.emotes.push(p.emote);
      } else out.push({ kind: 'stack', emotes: [p.emote] });
    }
    return out;
  }

  let root: HTMLElement;
  const still = $derived(reducedMotion.matches);
  let held = $state(false); // hovered or focused: lines stop arriving so they can be read
  let paused = $state(false); // the reader's own pause
  let next = 0;
  let key = 0;
  let shown = $state<Shown[]>([]);

  function at(i: number): Shown {
    const l = lines[i % lines.length];
    return { key: key++, user: l.user, look: l.look, pieces: pieces(l.parts) };
  }

  $effect(() => {
    // Start (or restart when the lines change) with the first lines in view.
    const n = live ? Math.min(visible, lines.length) : lines.length;
    key = 0;
    shown = Array.from({ length: n }, (_, i) => at(i));
    next = n;
  });

  $effect(() => {
    if (!live || still || lines.length === 0) return;
    let seen = false;
    let timer: ReturnType<typeof setInterval> | undefined;
    const run = () => {
      clearInterval(timer);
      timer = undefined;
      if (!seen || document.hidden) return;
      timer = setInterval(() => {
        if (held || paused) return;
        shown = [...shown.slice(-(visible - 1)), at(next++)];
      }, interval);
    };
    const io = new IntersectionObserver(([entry]) => {
      seen = entry.isIntersecting;
      run();
    });
    io.observe(root);
    document.addEventListener('visibilitychange', run);
    return () => {
      io.disconnect();
      document.removeEventListener('visibilitychange', run);
      clearInterval(timer);
    };
  });

  const name = $derived(platform === 'twitch' ? 'Twitch' : 'Kick');

  function width(e: ChatEmote) {
    const s = size(e);
    return Math.round((28 * s.width) / s.height);
  }
  const moving = $derived(play && !still);
</script>

<figure
  class="chat {platform}"
  class:live
  bind:this={root}
  style:--rows={visible}
  aria-label={label ?? `${name} chat preview`}
  onpointerenter={() => (held = true)}
  onpointerleave={() => (held = false)}
  onfocusin={() => (held = true)}
  onfocusout={() => (held = false)}
>
  <figcaption class="top">
    <span class="platform">{name}</span>
    <span class="what">Chat</span>
    {#if live && !still}
      <button class="pause" type="button" aria-pressed={paused} onclick={() => (paused = !paused)}>{paused ? 'Play' : 'Pause'}</button>
    {/if}
  </figcaption>
  <ol class="lines" aria-live="off">
    {#each shown as l (l.key)}
      <li class="line" animate:flip={{ duration: dur(250) }} in:fly={{ y: 8, duration: dur(250) }}>
        <span class="user">{#if l.look && (l.look.paint || l.look.badge)}<PaintedName name={l.user} paint={l.look.paint} badge={l.look.badge} badgeSize={16} />{:else}<bdi>{clean(l.user)}</bdi>{/if}</span><span class="colon">:</span>
        {#each l.pieces as p}
          {#if p.kind === 'text'}{p.text}{:else}
            <span class="stack" title={p.emotes.map((e) => e.name).join(' ')}>
              {#each p.emotes as e, i}
                {#if ready(e)}
                  {@const st = !moving && hasStill(e)}
                  {@const pic = pictureFor(e, display.dpr, st, width(e))}
                  <img
                    class:over={i > 0}
                    src={pic.src}
                    srcset={pic.srcset}
                    sizes={pic.srcset ? `${width(e)}px` : undefined}
                    width={width(e)}
                    height="28"
                    alt={e.name}
                    loading="lazy"
                    decoding="async"
                  />
                {:else}<span class="missing"><bdi>{clean(e.name)}</bdi></span>{/if}
              {/each}
            </span>
          {/if}
        {/each}
      </li>
    {/each}
  </ol>
</figure>

<style>
  .chat { margin: 0; display: flex; flex-direction: column; min-width: 0; border: 1px solid var(--border); border-radius: var(--radius-m);
    background: var(--bg); overflow: hidden; }
  .top { display: flex; align-items: center; gap: 8px; height: 44px; padding: 0 16px; border-bottom: 1px solid var(--border);
    font-size: var(--t-small); background: var(--surface); }
  .platform { font-weight: 600; }
  .what { color: var(--muted); }
  .lines { list-style: none; margin: 0; padding: 10px 0; display: flex; flex-direction: column; justify-content: flex-end; }
  .live .lines { height: calc(var(--rows) * 40px + 20px); overflow: hidden; }
  .line { padding: 4px 16px; font-size: var(--t-small); line-height: 32px; color: var(--text); overflow-wrap: anywhere; }
  /* The two frames differ in rhythm, not colour: Kick's lines sit a little looser. */
  .kick .line { padding-block: 5px; }
  .user { font-weight: 600; }
  .colon { margin-right: 6px; color: var(--muted); }
  .twitch .colon { color: var(--text); }
  .stack { display: inline-grid; vertical-align: middle; place-items: center; margin: -4px 0; }
  .stack img { grid-area: 1 / 1; display: block; height: var(--emote-chat); width: auto; }
  .missing { color: var(--muted); }
  .pause:focus-visible { outline-offset: -2px; }
  .pause { margin-left: auto; min-height: 40px; padding: 0 8px; border: 0; background: none; font: inherit; font-size: var(--t-small);
    color: var(--muted); border-radius: var(--radius-btn); cursor: pointer; }
  @media (hover: hover) and (pointer: fine) { .pause:hover { color: var(--text); } }
  @media (pointer: coarse) { .pause { min-height: 44px; padding: 0 12px; } }
</style>
