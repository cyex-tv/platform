<script lang="ts">
  // The extension box's button: a menu of the browsers it runs in, each going to its store listing
  // (stores.ts; /download until the listings are live). The browser the page is open in leads and
  // says so.
  import { onMount } from 'svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Popover from '$lib/components/Popover.svelte';
  import { STORES, listed, storeFor, type StoreKey } from './stores';

  let open = $state(false);
  let mine = $state<StoreKey | null>(null);
  onMount(() => (mine = storeFor(navigator.userAgent, 'brave' in navigator)));

  const list = $derived(mine ? [...STORES].sort((a, b) => Number(b.key === mine) - Number(a.key === mine)) : STORES);
</script>

<Popover label="Get the extension" kind="disclosure" width="248px" bind:open>
  {#snippet trigger(props)}
    <button class="btn btn-s" type="button" {...props}>Get the extension <Icon name="chevron-down" size={16} /></button>
  {/snippet}
  <ul class="stores">
    {#each list as s (s.key)}
      <li>
        <a href={s.url} onclick={() => (open = false)}>
          <span class="name">{s.browser}</span>
          <span class="hint">{s.key === mine ? 'Your browser' : listed(s) ? s.store : 'Test build'}</span>
        </a>
      </li>
    {/each}
  </ul>
</Popover>

<style>
  .stores { list-style: none; margin: 0; padding: 0; display: grid; font-size: var(--t-nav); }
  a { display: flex; align-items: center; gap: 12px; min-height: 40px; padding: 0 10px; border-radius: var(--radius);
    color: var(--text); text-decoration: none; transition: background-color var(--fast); }
  a:focus-visible { outline-offset: -2px; }
  a:active { opacity: .6; transition: none; }
  .name { font-weight: 500; }
  .hint { margin-left: auto; color: var(--dim); font-size: var(--t-fine); }
  @media (hover: hover) and (pointer: fine) { a:hover { background: var(--row-hover); } }
</style>
