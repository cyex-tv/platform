<script lang="ts">
  // The admin gate. The admin rail (in the signed-in layout) lists the sections you may open.
  import State from '$lib/components/State.svelte';
  import { isAdmin, session } from '$lib/tool/session.svelte';

  let { children } = $props();

  const allowed = $derived(isAdmin(session.me));
</script>

{#if !allowed}
  <State level={1} role="status" title="No access here.">
    Your account has no moderation or admin permissions.
    {#snippet actions()}<a class="ui-btn secondary" href="/me">Back to your emotes</a>{/snippet}
  </State>
{:else}
  {@render children()}
{/if}

