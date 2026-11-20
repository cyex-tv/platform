<script lang="ts">
  // Test harness: a Popover with one button inside and one outside it.
  import Popover from '$lib/components/Popover.svelte';
  let { onclose }: { onclose?: () => void } = $props();
</script>

<Popover label="Filters" {onclose}>
  {#snippet trigger(props)}<button type="button" {...props}>Open</button>{/snippet}
  <button type="button">Inside</button>
</Popover>
<button type="button">Outside</button>
