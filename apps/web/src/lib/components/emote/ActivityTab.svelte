<script lang="ts">
  // The emote page's Activity tab: who uploaded it and which sets added, renamed or removed it,
  // newest first, 30 at a time. Only listed (public) emotes have any, and the page doesn't mount
  // this for the others, so nothing is asked for them. Rows are the profile feed's.
  import { onMount } from 'svelte';
  import LoadMore from '$lib/components/LoadMore.svelte';
  import State from '$lib/components/State.svelte';
  import { loadCatalog, type Catalog } from '$lib/cosmetics/store.svelte';
  import { loadEmoteActivity, type ActivityItem } from '$lib/profile/activity';
  import ActivityRows from '$lib/profile/ActivityRows.svelte';

  let { emoteId }: { emoteId: string } = $props();

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
      const got = await loadEmoteActivity(emoteId, cursor);
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

{#if first}
  <State kind="loading" title="Loading activity…" />
{:else if problem && !items.length}
  <State kind="error" title="Activity didn’t load">
    {problem}
    {#snippet actions()}<button class="ui-btn secondary" type="button" onclick={next}>Try again</button>{/snippet}
  </State>
{:else if !items.length}
  <State title="No activity yet.">Sets adding, renaming or removing this emote show up here.</State>
{:else}
  <ActivityRows {items} {catalog} pictures={false} />
  <div class="foot"><LoadMore hasMore={more} {busy} error={problem} total={items.length} onmore={next} /></div>
{/if}

<style>
  .foot { padding: 20px 0 0; }
</style>
