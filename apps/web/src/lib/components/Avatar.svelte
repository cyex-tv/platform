<script lang="ts">
  // A round avatar with a letter fallback (no image, or one that fails to load; a new `src`
  // tries again). Its size is fixed up front, so nothing shifts. Decorative: the name next to it
  // is the text. `tone="tool"` sits on the signed-in pages' ground (cyex-app chip grey).
  //   <Avatar src={me.avatar} name={me.displayName} size={28} />
  import { clean } from '$lib/format';

  type Props = { src?: string | null; name: string; size?: number; tone?: 'web' | 'tool' };
  let { src = null, name, size = 72, tone = 'web' }: Props = $props();

  // The src that failed to load: it stays on the letter, a different src gets a fresh try.
  let failed = $state<string | null>(null);
  const broken = $derived(!!src && failed === src);
  const initial = $derived([...clean(name).trim()][0]?.toUpperCase() ?? '?');
</script>

<span
  class="av tone-{tone}"
  style:width="{size}px"
  style:height="{size}px"
  style:font-size="{Math.round(size * (tone === 'tool' ? 0.45 : 0.42))}px"
  aria-hidden="true"
>
  {#if src && !broken}
    <img {src} alt="" width={size} height={size} onerror={() => (failed = src)} />
  {:else}
    {initial}
  {/if}
</span>

<style>
  .av { flex: none; display: inline-grid; place-items: center; overflow: hidden; border-radius: 50%; line-height: 1;
    background: var(--surface-2); border: 1px solid var(--border); color: var(--muted); font-weight: 600;
    -webkit-user-select: none; user-select: none; }
  .tone-tool { background: var(--neutral-chip, var(--surface-2)); border: 0; font-weight: 700; }
  img { display: block; width: 100%; height: 100%; object-fit: cover; }
</style>
