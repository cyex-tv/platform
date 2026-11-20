<script lang="ts">
  // Settings → Hidden: the emotes and creators you hid for yourself ("Hide for me" in a tile's
  // menu or an emote's More menu), each with Show again. Only you see this list; nobody is told.
  // A row you show again stays put, marked "Shown again", so the list never shifts under a click.
  import { gql } from '$lib/api';
  import Avatar from '$lib/components/Avatar.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean, count, date } from '$lib/format';
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
      <div class="ui-stack">
        <section class="ui-panel" aria-labelledby="hidden-emotes-h">
          <header>
            <h2 id="hidden-emotes-h">Emotes</h2>
            <span class="ui-hint n">{count(lists.hiddenEmotes.length)} of 1,000</span>
          </header>
          {#if lists.hiddenEmotes.length === 0}
            <p class="ui-hint pad">Nothing hidden. Pick “Hide for me” in an emote’s menu and it stops showing in browse, search and trending for you.</p>
          {:else}
            <ul class="ui-list">
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
                    <span class="ui-hint meta">
                      {#if h.emote?.owner}by <bdi>{clean(h.emote.owner.displayName)}</bdi> · {/if}Hidden <time datetime={h.hiddenAt}>{date(h.hiddenAt)}</time>
                    </span>
                  </span>
                  {#if shown[key]}
                    <span class="ui-hint done">Shown again</span>
                  {:else}
                    <button class="ui-btn secondary small" type="button" disabled={!!busy} aria-busy={busy === key}
                      aria-label="Show {label} again" onclick={() => showAgain(key, () => personal.unhideEmote(h.emoteId), `${label} shows for you again.`)}>
                      Show again
                    </button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>

        <section class="ui-panel" aria-labelledby="hidden-creators-h">
          <header>
            <h2 id="hidden-creators-h">Creators</h2>
            <span class="ui-hint n">{count(lists.hiddenCreators.length)} of 200</span>
          </header>
          {#if lists.hiddenCreators.length === 0}
            <p class="ui-hint pad">No creators hidden. “Hide everything by …” in an emote’s menu hides all their emotes for you, new ones too.</p>
          {:else}
            <ul class="ui-list">
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
                    <span class="ui-hint meta">All their emotes · Hidden <time datetime={h.hiddenAt}>{date(h.hiddenAt)}</time></span>
                  </span>
                  {#if shown[key]}
                    <span class="ui-hint done">Shown again</span>
                  {:else}
                    <button class="ui-btn secondary small" type="button" disabled={!!busy} aria-busy={busy === key}
                      aria-label="Show {label}’s emotes again" onclick={() => showAgain(key, () => personal.unhideCreator(h.accountId), `${label}’s emotes show for you again.`)}>
                      Show again
                    </button>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </section>
      </div>
    {/snippet}
  </LoadGate>
</div>

<style>
  .n { margin-left: auto; font-variant-numeric: tabular-nums; }
  .pad { margin: 0; padding: 10px 14px 12px; }
  .what { display: grid; flex: 1; min-width: 0; }
  .name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; color: var(--text); text-decoration: none; }
  a.name:focus-visible { outline-offset: 2px; }
  @media (hover: hover) and (pointer: fine) { a.name:hover { text-decoration: underline; } }
  .meta { overflow-wrap: anywhere; }
  .blank { display: grid; place-items: center; width: 36px; height: 36px; flex: none; color: var(--muted); }
  .done { flex: none; padding-inline: 8px; white-space: nowrap; }
  .gone .what { opacity: .6; }
</style>
