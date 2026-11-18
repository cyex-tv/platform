<script lang="ts">
  // What happened on an account (7TV's activity feed): one row per change with an icon (added,
  // removed, renamed, paint or badge changed, …), a sentence with the emote's picture and the
  // names (paints drawn in the paint), who did it, and how long ago. Public. Pages back 30 at a time.
  import { onMount } from 'svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import Problem from '$lib/components/Problem.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loadActivity, type ActivityItem } from '../activity';
  import ActivityRows from '../ActivityRows.svelte';

  let { accountId }: { accountId: string } = $props();

  let items = $state<ActivityItem[]>([]);
  let first = $state(true);
  let busy = $state(false);
  /** Where the next page starts; null once the feed has ended. */
  let cursor = $state<string | null>(null);
  const more = $derived(cursor !== null);
  let problem = $state('');
  let catalog = $state<Catalog | null>(null);

  async function next() {
    if (busy) return;
    busy = true;
    problem = '';
    try {
      const got = await loadActivity(accountId, cursor);
      items = [...items, ...got.items];
      cursor = got.next;
    } catch (e) {
      problem = e instanceof Error ? e.message : 'That didn’t load.';
    } finally {
      busy = false;
      first = false;
    }
  }
  onMount(() => {
    next();
    loadCatalog().then((c) => (catalog = c)).catch(() => {});
  });


</script>

<PageHead title="Activity" line="Changes to the sets, emotes and cosmetics, newest first." />

{#if first}
  <State kind="loading" title="Loading activity…" />
{:else if problem && !items.length}
  <Problem cause="server" onretry={next} />
{:else if !items.length}
  <State title="Nothing yet.">Changes to the sets, emotes and cosmetics show up here.</State>
{:else}
  <ActivityRows {items} {catalog} />
  <div class="foot"><LoadMore hasMore={more} {busy} error={problem} total={items.length} onmore={next} /></div>
{/if}

<style>
  .foot { padding: 20px 0 0; }
</style>
