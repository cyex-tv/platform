<script lang="ts">
  // /me/upload: the upload form as a page, for old links, bookmarks and a new tab (Ctrl/⌘-click
  // on Upload). Everywhere else a link here opens the same form in the upload dialog
  // ($lib/upload). /upload redirects here. In your profile's frame, the rules in the panel.
  import PanelBlock from '$lib/components/PanelBlock.svelte';
  import MeFrame from '$lib/profile/MeFrame.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import UploadForm from '$lib/upload/UploadForm.svelte';
  import { UPLOAD_FORMATS } from '$lib/rules';
</script>

<svelte:head><title>Upload - cyex.tv</title></svelte:head>

{#snippet side()}
  <PanelBlock title="What works">
    <dl class="kv">
      <dt>Formats</dt><dd>{UPLOAD_FORMATS.replace(' or ', ', ')}</dd>
      <dt>File size</dt><dd>8 MB at most</dd>
      <dt>Picture</dt><dd>1000 × 1000 px at most</dd>
      <dt>At once</dt><dd>Up to 20</dd>
    </dl>
    <p class="pb-hint">Square or wide works best. New emotes wait for a moderator before others can add them; you can use them in your own sets straight away.</p>
  </PanelBlock>
  <PanelBlock title="Guidelines" more={{ href: '/guidelines', label: 'Read' }}>
    <p class="pb-hint">Only upload what you made or may use.</p>
  </PanelBlock>
{/snippet}

<MeFrame title="Upload" label="Upload rules" panel={side} narrow="after">
  <div class="up">
    <PageHead title="Upload" line="Drop your images, name and tag each one, then upload them together." />
    <UploadForm />
  </div>
</MeFrame>

<style>
  .up { max-width: 1040px; }
  .up > :global(.head) { max-width: var(--measure-form); }
</style>
