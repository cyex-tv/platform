<script lang="ts">
  // The admin gate. The admin rail (in the signed-in layout) lists the sections you may open.
  import { isAdmin, session } from '$lib/tool/session.svelte';

  let { children } = $props();

  const allowed = $derived(isAdmin(session.me));
</script>

{#if !allowed}
  <div class="ui-empty">
    <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="11" width="14" height="10" rx="1" /><path d="M8 11V7a4 4 0 0 1 8 0v4" /></svg>
    <p>Your account has no moderation or admin permissions.</p>
    <a class="ui-btn secondary" href="/me">Back to your emotes</a>
  </div>
{:else}
  {@render children()}
{/if}

