<script lang="ts">
  import type { Segment } from './example';
  let { paras, class: cls = '' }: { paras: Segment[][]; class?: string } = $props();
</script>

{#each paras as para}
  <p class={cls}>{#each para as s}{#if s.code}<code>{s.text}</code>{:else}{s.text}{/if}{/each}</p>
{/each}
