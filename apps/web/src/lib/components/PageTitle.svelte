<script lang="ts">
  // Sets the tab title from inside a block, where <svelte:head> isn't allowed (an error or loading state in a layout).
  let { title }: { title: string } = $props();
</script>

<svelte:head><title>{title}</title></svelte:head>
