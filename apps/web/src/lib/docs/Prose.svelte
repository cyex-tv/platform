<script lang="ts">
  import type { Segment } from './example';
  import Inline from './Inline.svelte';
  let { paras, class: cls = '' }: { paras: Segment[][]; class?: string } = $props();
</script>

{#each paras as para}
  <p class={cls}><Inline paras={[para]} /></p>
{/each}
