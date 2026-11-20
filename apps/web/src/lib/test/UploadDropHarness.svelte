<script lang="ts">
  // Test harness: the upload form inside something that takes drops too, as the site-wide upload
  // dialog does (it adds what is dropped anywhere on it).
  import UploadForm from '$lib/upload/UploadForm.svelte';
  let { inDialog = true }: { inDialog?: boolean } = $props();
  let form = $state<UploadForm>();
</script>

<div data-testid="outer" role="presentation" ondragover={(e) => e.preventDefault()} ondrop={(e) => { e.preventDefault(); form?.add(e.dataTransfer?.files); }}>
  <UploadForm bind:this={form} {inDialog} />
</div>
