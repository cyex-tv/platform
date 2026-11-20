<script lang="ts">
  // The staff shortcut at the bottom of every profile tab. Rendered from the client session only,
  // so signed-out visitors and people without the permission never get it, not even hidden.
  import Icon from '$lib/components/Icon.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { adminUserHref, canOpenInAdmin } from './admin-link';

  let { accountId }: { accountId: string } = $props();
</script>

{#if canOpenInAdmin(session.me)}
  <div class="admin-foot">
    <a class="btn btn-s" href={adminUserHref(accountId)}><Icon name="wrench" size={15} />Open in admin</a>
  </div>
{/if}

<style>
  .admin-foot { display: flex; justify-content: flex-end; margin-top: 32px; padding-top: 16px; border-top: 1px solid var(--border); }
</style>
