<script lang="ts">
  // /me/upload: the upload form as a page, for old links, bookmarks and a new tab (Ctrl/⌘-click
  // on Upload). Everywhere else a link here opens the same form in the upload dialog
  // ($lib/upload). /upload redirects here.
  import PageHead from '$lib/tool/PageHead.svelte';
  import UploadForm from '$lib/upload/UploadForm.svelte';
  import { UPLOAD_FORMATS } from '$lib/rules';
</script>

<svelte:head><title>Upload - cyex.tv</title></svelte:head>

<div class="up">
  <PageHead title="Upload" line="{UPLOAD_FORMATS}, at most 8 MB and 1000 × 1000 px. Square or wide works best. Up to 20 at once." />
  <UploadForm />
</div>

<style>
  .up { max-width: 1040px; }
  .up > :global(.head) { max-width: var(--measure-form); }
</style>
