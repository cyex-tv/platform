<script lang="ts">
  // Test harness: the select bar with the page's callbacks counted in barCalls.
  import SelectBar from '$lib/components/browse/SelectBar.svelte';
  import { barCalls } from './bar-calls';
  let { ids = [], shown = 0, note = '' }: { ids?: string[]; shown?: number; note?: string } = $props();
</script>

<SelectBar
  {ids}
  {shown}
  {note}
  signin="/sign-in?next=/search"
  onall={() => barCalls.all++}
  onclear={() => barCalls.clear++}
  onexit={() => barCalls.exit++}
  onadded={() => []}
/>
