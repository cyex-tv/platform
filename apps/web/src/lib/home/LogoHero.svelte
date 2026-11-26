<script lang="ts">
  // The hero's logo. A still image of the 3D logo paints first (one per theme) and is all most of
  // the page's readers need: it is the fallback where there's no 3D and what reduced motion keeps.
  // On a device that can take it (WebGL, no data saver, enough memory and cores, motion allowed),
  // the live scene loads after the page has settled and fades in over it in the same pose, then
  // starts to move. The box has a fixed shape, so nothing shifts when it arrives (CLS 0), and
  // nothing on the first screen waits for it. Reduced motion, turned on at any time (the system's
  // or the site's own), puts the still image back.
  import { onMount } from 'svelte';
  import { capable, motionReduced, readEnv } from './capable';
  import { settled } from './settle';
  import type { LogoScene } from './logo-scene';

  // The box is 340px wide up to 1100px and 460px above it (routes/+page.svelte): pick the 420w or 840w file.
  const sizes = '(max-width: 1099.98px) 340px, 460px';

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
        if (gone || motionReduced()) return;
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
        // The chunk didn't load or WebGL refused: the still image is already there.
        failed = true;
        stop();
      } finally {
        starting = false;
      }
    }

    const onMotion = () => (motionReduced() ? stop() : start());
    motion.addEventListener('change', onMotion);
    const site = new MutationObserver(onMotion);
    site.observe(document.documentElement, { attributes: true, attributeFilter: ['data-motion'] });
    start();
    return () => {
      gone = true;
      motion.removeEventListener('change', onMotion);
      site.disconnect();
      stop();
    };
  });
</script>

<div class="logo-hero" class:live bind:this={host} aria-hidden="true">
  <!-- Lazy, so only the one for the theme in use loads (the other is display: none). -->
  <picture class="still dark">
    <source type="image/avif" srcset="/home/logo-3d-dark-420.avif 420w, /home/logo-3d-dark.avif 840w" sizes={sizes} />
    <img src="/home/logo-3d-dark.webp" width="840" height="840" alt="" loading="lazy" decoding="async" />
  </picture>
  <picture class="still light">
    <source type="image/avif" srcset="/home/logo-3d-light-420.avif 420w, /home/logo-3d-light.avif 840w" sizes={sizes} />
    <img src="/home/logo-3d-light.webp" width="840" height="840" alt="" loading="lazy" decoding="async" />
  </picture>
</div>

<style>
  .logo-hero { position: relative; width: 100%; aspect-ratio: 1; contain: layout paint; }
  .still, .logo-hero :global(canvas) { position: absolute; inset: 0; width: 100%; height: 100%; }
  .still img { display: block; width: 100%; height: 100%; }
  .still { display: block; transition: opacity .5s var(--ease); user-select: none; -webkit-user-drag: none; }
  .light { display: none; }
  :global([data-theme='light']) .light { display: block; }
  :global([data-theme='light']) .dark { display: none; }
  .logo-hero :global(canvas) { display: block; opacity: 0; transition: opacity .5s var(--ease); }
  .live :global(canvas) { opacity: 1; }
  .live .still { opacity: 0; }
</style>
