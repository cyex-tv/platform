<script lang="ts">
  // Settings → Hidden: the emotes and creators you hid for yourself ("Hide for me" in a tile's
  // menu or an emote's More menu), each with Show again. Only you see this list; nobody is told.
  // A row you show again stays put, marked "Shown again", so the list never shifts under a click.
  import { gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import Section from '$lib/components/Section.svelte';
  import { clean, date } from '$lib/format';
  import { listError, personal } from '$lib/personal/lists.svelte';
  import { HIDDEN_LISTS } from '$lib/queries';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import { loader } from '$lib/tool/loader.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import Thumb from '$lib/tool/Thumb.svelte';

  const L = loader(async ({ fetch }) => (await gql(HIDDEN_LISTS, {}, fetch)));
  let shown = $state<Record<string, true>>({});
  let busy = $state('');

  async function showAgain(key: string, change: () => Promise<unknown>, done: string) {
    if (busy) return;
    busy = key;
    try {
      await change();
      shown[key] = true;
      notify(done);
    } catch (e) {
      notify(listError(e), true);
    } finally {
      busy = '';
    }
  }
</script>

<svelte:head><title>Hidden - cyex.tv</title></svelte:head>

<div class="narrow">
  <PageHead title="Hidden" line="Emotes and creators you hid for yourself. Only you see this list." />
  <LoadGate source={L} label="Loading what you hid…">
    {#snippet children(lists)}
      <Section title="Hidden emotes" count="{lists.hiddenEmotes.length} / 1000">
        {#if lists.hiddenEmotes.length === 0}
          <p class="empty">Nothing hidden. Pick “Hide for me” in an emote’s menu and it stops showing in browse, search and trending for you.</p>
        {:else}
          <ul class="rows">
            {#each lists.hiddenEmotes as h (h.emoteId)}
              {@const key = `e-${h.emoteId}`}
              {@const label = h.emote ? clean(h.emote.name) : 'An emote that’s gone'}
              <li class:gone={shown[key]}>
                {#if h.emote}
                  <Thumb emote={h.emote} name="" box={36} />
                {:else}
                  <span class="blank" aria-hidden="true"><Icon name="eye-off" size={16} /></span>
                {/if}
                <span class="what">
                  {#if h.emote}
                    <a class="name" href="/emotes/{h.emoteId}"><bdi>{label}</bdi></a>
                  {:else}
                    <span class="name">{label}</span>
                  {/if}
                  <span class="meta">
                    {#if h.emote?.owner}by <bdi>{clean(h.emote.owner.displayName)}</bdi>{' · '}{/if}Hidden <time class="ui-mono" datetime={h.hiddenAt}>{date(h.hiddenAt)}</time>
                  </span>
                </span>
                {#if shown[key]}
                  <span class="done">Shown again</span>
                {:else}
                  <button class="ui-btn ghost small" type="button" disabled={!!busy} aria-busy={busy === key}
                    aria-label="Show {label} again" onclick={() => showAgain(key, () => personal.unhideEmote(h.emoteId), `${label} shows for you again.`)}>
                    Show again
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </Section>

      <Section title="Hidden creators" count="{lists.hiddenCreators.length} / 200">
        {#if lists.hiddenCreators.length === 0}
          <p class="empty">No creators hidden. “Hide everything by …” in an emote’s menu hides all their emotes for you, new ones too.</p>
        {:else}
          <ul class="rows">
            {#each lists.hiddenCreators as h (h.accountId)}
              {@const key = `c-${h.accountId}`}
              {@const label = h.account ? clean(h.account.displayName) : 'A deleted account'}
              <li class:gone={shown[key]}>
                <Avatar src={h.account?.avatar} name={label} size={36} tone="tool" />
                <span class="what">
                  {#if h.account}
                    <a class="name" href="/users/{encodeURIComponent(h.accountId)}"><bdi>{label}</bdi></a>
                  {:else}
                    <span class="name">{label}</span>
                  {/if}
                  <span class="meta">All their emotes · Hidden <time class="ui-mono" datetime={h.hiddenAt}>{date(h.hiddenAt)}</time></span>
                </span>
                {#if shown[key]}
                  <span class="done">Shown again</span>
                {:else}
                  <button class="ui-btn ghost small" type="button" disabled={!!busy} aria-busy={busy === key}
                    aria-label="Show {label}’s emotes again" onclick={() => showAgain(key, () => personal.unhideCreator(h.accountId), `${label}’s emotes show for you again.`)}>
                    Show again
                  </button>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      </Section>
    {/snippet}
  </LoadGate>
</div>

<style>
  .rows { margin: 0; padding: 0; list-style: none; }
  .rows > li { display: flex; align-items: center; gap: 12px; min-height: 52px; padding: 8px 0; border-bottom: 1px solid var(--border); }
  .empty { margin: 8px 0; font-size: var(--t-small); color: var(--muted); }
  .what { display: grid; gap: 2px; flex: 1; min-width: 0; }
  .name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--t-ui); font-weight: 500; color: var(--text); text-decoration: none; }
  a.name:focus-visible { outline-offset: 2px; }
  @media (hover: hover) and (pointer: fine) { a.name:hover { text-decoration: underline; } }
  .meta { font-size: var(--t-small); color: var(--muted); overflow-wrap: anywhere; }
  .blank { display: grid; place-items: center; width: 36px; height: 36px; flex: none; color: var(--muted); }
  .done { flex: none; font-size: var(--t-small); color: var(--muted); white-space: nowrap; }
  .gone .what { opacity: .6; }
</style>
