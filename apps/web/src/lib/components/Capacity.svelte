<script lang="ts">
  // How full a set is, written one way on every surface: "24 / 1.000" to the eye, "24 of 1.000
  // emotes used" to a screen reader (see capacityLabel in lib/sets/public.ts).
  import { capacityLabel, capacityText } from '$lib/sets/public';

  type Props = { used: number; capacity: number };
  let { used, capacity }: Props = $props();
</script>

<span aria-hidden="true">{capacityText(used, capacity)}</span><span class="sr-only">{capacityLabel(used, capacity)}</span>
