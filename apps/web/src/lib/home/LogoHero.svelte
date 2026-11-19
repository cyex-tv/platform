<script lang="ts">
  // The hero's logo. The flat SVG paints first and is all most of the page's readers need; on a
  // device that can take it (WebGL, no data saver, enough memory and cores, motion allowed), the
  // 3D scene loads after the page has settled and fades in over it at the same size and pose.
  // The box has a fixed shape, so nothing moves when it arrives (CLS 0), and nothing on the first
  // screen waits for it. Reduced motion, turned on at any time, puts the flat logo back.
  import { onMount } from 'svelte';
  import LogoMark from './LogoMark.svelte';
  import { capable, readEnv } from './capable';
  import { settled } from './settle';
  import type { LogoScene } from './logo-scene';

  let host: HTMLDivElement;
  let live = $state(false);

  onMount(() => {
    let scene: LogoScene | null = null;
    // A fresh canvas per start: a context once given back can't be had again from the same one.
    let canvas: HTMLCanvasElement | null = null;
    let gone = false;
    let failed = false;
    const motion = matchMedia('(prefers-reduced-motion: reduce)');

    const stop = () => {
      live = false;
      scene?.dispose();
      scene = null;
      canvas?.remove();
      canvas = null;
    };

    let starting = false;
    async function start() {
      if (scene || failed || gone || starting) return;
      starting = true;
      try {
        await settled();
        if (gone || scene || !capable(readEnv())) return;
        // Marks for the page's own timeline: when the 3D was asked for, and when it showed.
        performance.mark('logo-3d:load');
        const { mountLogo } = await import('./logo-scene');
        if (gone || motion.matches) return;
        canvas = document.createElement('canvas');
        host.append(canvas);
        scene = mountLogo(canvas, host, {
          onReady: () => {
            performance.mark('logo-3d:ready');
            live = true;
          },
          onFail: () => {
            failed = true;
            stop();
          },
        });
      } catch {
        // The chunk didn't load or WebGL refused: the flat logo is already there.
        failed = true;
        stop();
      } finally {
        starting = false;
      }
    }

    const onMotion = () => (motion.matches ? stop() : start());
    motion.addEventListener('change', onMotion);
    start();
    return () => {
      gone = true;
      motion.removeEventListener('change', onMotion);
      stop();
    };
  });
</script>

<div class="logo-hero" class:live bind:this={host} aria-hidden="true">
  <div class="flat"><LogoMark /></div>
</div>

<style>
  .logo-hero { position: relative; width: 100%; aspect-ratio: 1; contain: layout paint; }
  .flat, .logo-hero :global(canvas) { position: absolute; inset: 0; width: 100%; height: 100%; }
  .flat { transition: opacity .9s var(--ease); }
  .logo-hero :global(canvas) { display: block; opacity: 0; transition: opacity .9s var(--ease); }
  .live :global(canvas) { opacity: 1; }
  .live .flat { opacity: 0; }
</style>
