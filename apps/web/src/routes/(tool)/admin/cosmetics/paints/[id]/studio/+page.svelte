<script lang="ts">
  // An existing paint in the paint studio.
  import { page } from '$app/state';
  import AdminStudio from '$lib/paint-studio/AdminStudio.svelte';

  const id = page.params.id ?? null;
</script>

<AdminStudio paintId={id} />
