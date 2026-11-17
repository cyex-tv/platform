<script lang="ts">
  // A new paint in the paint studio. `?id=` (older links) opens that paint instead.
  import { page } from '$app/state';
  import AdminStudio from '$lib/paint-studio/AdminStudio.svelte';

  const id = page.url.searchParams.get('id');
</script>

<AdminStudio paintId={id} />
