<script lang="ts">
  import { apiBackend, PaintStudio } from '$lib/paint-studio';

  let { data } = $props();
  const backend = apiBackend();
</script>

<svelte:head><title>Paint studio - cyex.tv</title></svelte:head>

<div class="wrap ui-page" style:padding-block="26px 60px">
  <div class="ui-head">
    <div>
      <h1>Paint studio</h1>
      <p>Development page. Saves go to the API at PUBLIC_API_URL.</p>
    </div>
  </div>
  <PaintStudio {backend} paintId={data.paintId} />
</div>
