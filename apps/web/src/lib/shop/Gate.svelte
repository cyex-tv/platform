<script lang="ts">
  // An admin page that needs one permission: without it, the same "No access" state the admin gate
  // uses (the rail only lists what you may open, but an address can be typed).
  import type { Snippet } from 'svelte';
  import State from '$lib/components/State.svelte';
  import { can } from '$lib/tool/session.svelte';

  let { permission, children }: { permission: string; children: Snippet } = $props();
</script>

{#if can(permission)}
  {@render children()}
{:else}
  <State level={1} role="status" title="No access here.">
    This page needs the <code>{permission}</code> permission.
    {#snippet actions()}<a class="ui-btn secondary" href="/admin">Back to admin</a>{/snippet}
  </State>
{/if}
