<script lang="ts">
  // The link preview tags for one page (lib/meta.ts builds them). The root layout renders one,
  // from the page's load (`meta`) or the site's own card, so the tags are in the server's HTML.
  import type { Meta } from '$lib/meta';

  let { meta }: { meta: Meta } = $props();
</script>

<svelte:head>
  {#if meta.description}
    <meta name="description" content={meta.description} />
    <meta property="og:description" content={meta.description} />
    <meta name="twitter:description" content={meta.description} />
  {/if}
  <link rel="canonical" href={meta.url} />
  <meta property="og:site_name" content="cyex.tv" />
  <meta property="og:type" content={meta.type} />
  {#if meta.title}<meta property="og:title" content={meta.title} /><meta name="twitter:title" content={meta.title} />{/if}
  <meta property="og:url" content={meta.url} />
  {#each meta.images as img (img.url)}
    <meta property="og:image" content={img.url} />
    {#if img.width && img.height}
      <meta property="og:image:width" content={String(img.width)} />
      <meta property="og:image:height" content={String(img.height)} />
    {/if}
    <meta property="og:image:alt" content={img.alt} />
  {/each}
  <meta name="twitter:card" content={meta.card} />
  {#if meta.images[0]}
    <meta name="twitter:image" content={meta.images[0].url} />
    <meta name="twitter:image:alt" content={meta.images[0].alt} />
  {/if}
  {#if meta.noindex}<meta name="robots" content="noindex" />{/if}
</svelte:head>
