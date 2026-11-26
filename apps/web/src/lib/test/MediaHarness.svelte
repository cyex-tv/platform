<script lang="ts">
  // Test helper: shows what lib/media.svelte answers for a query and for reduced motion.
  import { media, reducedMotion } from '$lib/media.svelte';
  let { query }: { query: string } = $props();
</script>

<p data-q>{media(query).matches}</p>
<p data-reduced>{reducedMotion.matches}</p>
