<script lang="ts">
  // Settings → Appearance, saved on this device: the theme (the same switch as the header's) and
  // reduced motion (System follows the OS; On turns motion down everywhere on cyex.tv; Off keeps
  // the site's motion even when the OS asks for less).
  import { onMount } from 'svelte';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import Segmented from '$lib/components/Segmented.svelte';
  import type { MotionPref } from '$lib/account/settings';
  import { motion, setMotion, syncMotion } from '$lib/shell/motion.svelte';
  import { syncTheme, theme, toggleTheme } from '$lib/shell/theme.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';

  onMount(() => {
    syncTheme();
    syncMotion();
  });

  const THEMES = [
    { value: 'dark', label: 'Dark', icon: 'moon' },
    { value: 'light', label: 'Light', icon: 'sun' },
  ] as const;
  const MOTION: { value: MotionPref; label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'reduce', label: 'On' },
    { value: 'full', label: 'Off' },
  ];
  const HINT: Record<MotionPref, string> = {
    system: 'Follows your device: less motion when it asks for less.',
    reduce: 'No sliding, zooming or animated transitions on cyex.tv, and animated emotes and paints stay still.',
    full: 'cyex.tv keeps its motion, even when your device asks for less.',
  };
</script>

<svelte:head><title>Appearance - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Appearance" line="Saved on this device only." />
  <Section title="Theme">
    <Row label="Theme" hint="Dark or light, the same switch as the header’s.">
      <Segmented label="Theme" options={[...THEMES]} value={theme.light ? 'light' : 'dark'} onchange={(v) => { if ((v === 'light') !== theme.light) toggleTheme(); }} />
    </Row>
  </Section>
  <Section title="Motion">
    <Row label="Reduced motion">
      {#snippet hint()}<span aria-live="polite">{HINT[motion.pref]}</span>{/snippet}
      <Segmented label="Reduced motion" options={MOTION} value={motion.pref} onchange={setMotion} />
    </Row>
  </Section>
</div>
