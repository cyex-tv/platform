<script lang="ts" generics="T extends string">
  // The studio's choice rows (tabs, ground, shape, file type): the shared Segmented control, so a
  // selected option is bold with an accent icon here too, never a bar. `big` is the 44px size.
  import Segmented from '$lib/components/Segmented.svelte';
  import type { IconName } from '$lib/components/Icon.svelte';

  type Props = { label: string; options: { value: T; label: string; icon?: IconName }[]; value: T; onchange: (v: T) => void; big?: boolean };
  let { label, options, value, onchange, big = false }: Props = $props();
</script>

<Segmented {label} {options} {value} {onchange} size={big ? 'm' : 's'} height={big ? 44 : undefined} />
