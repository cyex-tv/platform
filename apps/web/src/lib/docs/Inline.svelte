<script lang="ts">
  // One run of text: plain, `code` or a link. Paragraph breaks become spaces.
  import type { Segment } from './example';
  let { paras }: { paras: Segment[][] } = $props();
  const runs = $derived(paras.flatMap((p, i) => (i ? [{ code: false, text: ' ' } as Segment, ...p] : p)));
</script>

{#each runs as s}{#if s.code}<code>{s.text}</code>{:else if s.href}<a href={s.href}>{s.text}</a>{:else}{s.text}{/if}{/each}
