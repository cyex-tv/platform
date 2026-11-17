<script lang="ts">
  // A round avatar with a letter fallback (no image, or one that fails to load). Size is fixed
  // up front, so nothing shifts.
  let { src = null, name, size = 72 }: { src?: string | null; name: string; size?: number } = $props();
  let broken = $state(false);
  $effect(() => {
    void src;
    broken = false;
  });
</script>

<span class="av" style:width="{size}px" style:height="{size}px" style:font-size="{Math.round(size * 0.42)}px" aria-hidden="true">
  {#if src && !broken}
    <img {src} alt="" width={size} height={size} onerror={() => (broken = true)} />
  {:else}
    {[...name][0]?.toUpperCase() ?? '?'}
  {/if}
</span>

<style>
  .av { flex: none; display: inline-grid; place-items: center; overflow: hidden; border-radius: 50%;
    background: var(--surface-2); border: 1px solid var(--border); color: var(--muted); font-weight: 600; }
  img { display: block; width: 100%; height: 100%; object-fit: cover; }
</style>
