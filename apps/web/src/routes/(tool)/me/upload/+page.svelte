<script lang="ts">
  // /me/upload: the upload form as a page, for old links, bookmarks and a new tab (Ctrl/⌘-click
  // on Upload). Everywhere else a link here opens the same form in the upload dialog
  // ($lib/upload). /upload redirects here. In your profile's frame: the files in the middle; who
  // you upload as, the rules, the guidelines and the submit area in the panel (the form hands
  // that column over through `side`).
  import type { Snippet } from 'svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import UploadForm from '$lib/upload/UploadForm.svelte';

  let side = $state<Snippet | null>(null);
</script>

<svelte:head><title>Upload - cyex.tv</title></svelte:head>

{#snippet panel()}{#if side}{@render side()}{/if}{/snippet}

<MeFrame title="Upload" label="Upload rules and submit" {panel} narrow="after" fold={false}>
  <div class="up">
    <PageHead title="Upload" line="Drop your images, name and tag each one, then upload them together." />
    <UploadForm aside bind:side />
  </div>
</MeFrame>

<style>
  .up { max-width: 1040px; }
  .up > :global(.head) { max-width: var(--measure-form); }
</style>
