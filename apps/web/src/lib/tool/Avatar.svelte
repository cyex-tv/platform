<script lang="ts">
  let { src = null, name, size = 24 }: { src?: string | null; name: string; size?: number } = $props();
  let broken = $state(false);
</script>

{#if src && !broken}
  <img class="av" {src} alt="" width={size} height={size} onerror={() => (broken = true)} />
{:else}
  <span class="av blank" style:width="{size}px" style:height="{size}px" style:font-size="{Math.round(size * 0.45)}px" aria-hidden="true"
    >{[...name][0]?.toUpperCase() ?? '?'}</span
  >
{/if}

<style>
  .av { flex: none; border-radius: 50%; object-fit: cover; background: var(--neutral-chip); }
  .blank { display: inline-grid; place-items: center; color: var(--muted); font-weight: 700; }
</style>
