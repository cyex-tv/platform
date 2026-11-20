<script lang="ts">
  // The body of one closed group, drawn when it is first opened: fetches the group's JSON, shows a
  // loading state only if that takes a moment, and an error with Retry if it fails. `onready` runs
  // once the sections are in the DOM, so the scroll-spy and a deep link can measure them.
  import { tick } from 'svelte';
  import State from '$lib/components/State.svelte';
  import GqlOpView from './GqlOpView.svelte';
  import GqlTypeView from './GqlTypeView.svelte';
  import RestOpView from './RestOpView.svelte';
  import { fetchGroup, type GroupPayload } from './lazy';

  let {
    id,
    known,
    base,
    onready,
    assets = '',
    fetcher = fetch,
  }: { id: string; known: string[]; base: string; onready: () => void; assets?: string; fetcher?: typeof fetch } = $props();
  let payload = $state<GroupPayload | null>(null);
  let failed = $state(false);

  async function load() {
    failed = false;
    try {
      payload = await fetchGroup(id, assets, fetcher);
      await tick();
      onready();
    } catch {
      failed = true;
    }
  }
  load();
</script>

{#if payload}
  {#if payload.kind === 'rest'}
    {#each payload.ops as op (op.id)}<RestOpView {op} {base} />{/each}
  {:else if payload.kind === 'ops'}
    {#each payload.ops as op (op.id)}<GqlOpView {op} {known} />{/each}
  {:else}
    {#each payload.types as t (t.id)}<GqlTypeView {t} {known} />{/each}
  {/if}
{:else if failed}
  <State kind="error" title="This section did not load." bare>
    {#snippet children()}Check the connection and try again.{/snippet}
    {#snippet actions()}<button type="button" class="ui-btn" onclick={load}>Retry</button>{/snippet}
  </State>
{:else}
  <div class="wait"><State kind="loading" title="Loading" bare /></div>
{/if}

<style>
  /* A fast load never shows the spinner. */
  .wait { animation: show 0s linear 250ms backwards; }
  @keyframes show { from { visibility: hidden; } }
</style>
