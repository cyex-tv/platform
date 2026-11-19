<script lang="ts">
  // A link to an account's admin page, in a list: their name when it is known (looked up once),
  // otherwise the shortened id. Nothing painted: an admin list is for finding rows, not for show.
  import { lookUp, who } from './names.svelte';
  import { clean } from '$lib/format';

  let { id, name }: { id: string | null | undefined; name?: string | null } = $props();
  $effect(() => {
    if (id && !name) lookUp([id]);
  });
  const found = $derived(id ? who(id) : undefined);
  const label = $derived(clean(name) || clean(found?.name) || '');
</script>

{#if !id}
  <span class="ui-muted">System</span>
{:else}
  <a href="/admin/users/{encodeURIComponent(id)}" title={id}>
    {#if label}<bdi>{label}</bdi>{:else}<span class="ui-mono">{id.slice(-8)}</span>{/if}
  </a>
{/if}
