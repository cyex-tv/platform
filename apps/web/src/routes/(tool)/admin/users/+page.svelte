<script lang="ts">
  // Find an account by the login it signed in with (or by its account id), then open its page.
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { gql } from '$lib/api';
  import type { Platform, UserQuery } from '$lib/gql/graphql';
  import { USER } from '$lib/queries';
  import { looksLikeId } from '$lib/admin/audit';
  import Avatar from '$lib/components/Avatar.svelte';
  import State from '$lib/components/State.svelte';
  import AccountSearch from '$lib/admin/AccountSearch.svelte';
  import PaintedName from '$lib/cosmetics/PaintedName.svelte';
  import { lookFor, NO_LOOK, type Look } from '$lib/cosmetics/store.svelte';
  import LoadGate from '$lib/tool/LoadGate.svelte';
  import PageHead from '$lib/tool/PageHead.svelte';
  import { loader } from '$lib/tool/loader.svelte';

  type Found = NonNullable<UserQuery['channelByLogin']>;

  // "Any name" searches every account by display name or login; Twitch and Kick look up one exact login.
  const nameQuery = $derived((page.url.searchParams.get('name') ?? '').trim());
  let platform = $state<Platform | 'NAME'>(
    page.url.searchParams.has('name') ? 'NAME' : (page.url.searchParams.get('platform')?.toUpperCase() as Platform) || 'TWITCH',
  );
  let login = $state(page.url.searchParams.get('name') ?? page.url.searchParams.get('login') ?? '');
  let look = $state<Look>(NO_LOOK);

  const L = loader(async ({ fetch }) => {
    if (page.url.searchParams.has('name')) return { found: null as Found | null, searched: false };
    const p = (page.url.searchParams.get('platform')?.toUpperCase() as Platform) || 'TWITCH';
    const name = (page.url.searchParams.get('login') ?? '').trim().toLowerCase();
    look = NO_LOOK;
    if (!name) return { found: null as Found | null, searched: false };
    const r = await gql(USER, { platform: p, login: name }, fetch);
    const f = r.channelByLogin;
    if (f) lookFor(f.platform, f.platformChannelId).then((l) => (look = l));
    return { found: f, searched: true };
  });

  function search(e: SubmitEvent) {
    e.preventDefault();
    const name = login.trim().replace(/^@/, '');
    // An account id goes straight to its page.
    if (platform === 'NAME') return void goto(`?name=${encodeURIComponent(login.trim().replace(/^@/, ''))}`, { keepFocus: true });
    if (looksLikeId(name)) return void goto(`/admin/users/${name.toUpperCase()}`);
    goto(`?platform=${platform.toLowerCase()}&login=${encodeURIComponent(name.toLowerCase())}`, { keepFocus: true });
  }
</script>

<svelte:head><title>Users - Admin - cyex.tv</title></svelte:head>

<PageHead title="Users" line="Find someone by name, by the login they signed in with, or by account ID." />

<form class="find ui-panel" onsubmit={search}>
  <div class="ui-seg" role="group" aria-label="Platform">
    <button type="button" aria-pressed={platform === 'TWITCH'} onclick={() => (platform = 'TWITCH')}>Twitch</button>
    <button type="button" aria-pressed={platform === 'KICK'} onclick={() => (platform = 'KICK')}>Kick</button>
    <button type="button" aria-pressed={platform === 'NAME'} onclick={() => (platform = 'NAME')}>Any name</button>
  </div>
  <input class="ui-input" bind:value={login} placeholder={platform === 'NAME' ? 'Name or login' : 'Login or account ID'} aria-label={platform === 'NAME' ? 'Name or login' : 'Login or account ID'} autocomplete="off" spellcheck="false" />
  <button class="ui-btn primary" type="submit" disabled={!login.trim()}>Find</button>
</form>

{#if page.url.searchParams.has('name')}
  {#if nameQuery}{#key nameQuery}<AccountSearch query={nameQuery} />{/key}{/if}
{:else if page.url.searchParams.get('login')}
<LoadGate source={L} label="Looking them up…">
{#snippet children(result)}
{#if !result.found}
  <State title="Nobody found.">No one with that login has signed in to cyex.tv.</State>
{:else}
  {@const f = result.found}
  <ul class="ui-panel ui-list result">
    <li>
      <Avatar src={f.account.avatar} name={f.account.displayName} size={40} tone="tool" />
      <span class="what">
        <a class="open" href="/admin/users/{encodeURIComponent(f.account.id)}"><PaintedName name={f.account.displayName} paint={look.paint} badge={look.badge} /></a>
        <span class="ui-hint"><bdi>{f.login}</bdi> on {f.platform === 'TWITCH' ? 'Twitch' : 'Kick'}</span>
      </span>
      <a class="ui-btn secondary small" href="/admin/users/{encodeURIComponent(f.account.id)}">Open account</a>
    </li>
  </ul>
{/if}
{/snippet}
</LoadGate>
{/if}

<style>
  .find { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; max-width: var(--measure-form); padding: 10px; margin-bottom: 16px; }
  .find .ui-input { flex: 1; min-width: 160px; }
  .result { max-width: var(--measure-form); }
  .what { display: grid; flex: 1; min-width: 0; }
  .open { font-weight: 600; color: var(--text); }
</style>
