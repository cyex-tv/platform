<script lang="ts">
  // Find an account by the login it signed in with (or by its account id), then open its page.
  import Segmented from '$lib/components/Segmented.svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { gql } from '$lib/api';
  import type { Platform, UserQuery } from '$lib/gql/graphql';
  import { USER } from '$lib/queries';
  import { looksLikeId } from '$lib/admin/audit';
  import Icon from '$lib/components/Icon.svelte';
  import State from '$lib/components/State.svelte';
  import AccountList from '$lib/admin/AccountList.svelte';
  import AccountSearch from '$lib/admin/AccountSearch.svelte';
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

  const L = loader(async ({ fetch }) => {
    if (page.url.searchParams.has('name')) return { found: null as Found | null, searched: false };
    const p = (page.url.searchParams.get('platform')?.toUpperCase() as Platform) || 'TWITCH';
    const name = (page.url.searchParams.get('login') ?? '').trim().toLowerCase();
    if (!name) return { found: null as Found | null, searched: false };
    const r = await gql(USER, { platform: p, login: name }, fetch);
    return { found: r.channelByLogin, searched: true };
  });

  // Arrow down from the field goes into the results.
  function down(e: KeyboardEvent) {
    if (e.key !== 'ArrowDown') return;
    const first = document.querySelector<HTMLElement>('ul.adm-rows [data-item]');
    if (!first) return;
    e.preventDefault();
    first.focus();
  }

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

<form class="adm-toolbar" onsubmit={search} role="search">
  <Segmented label="Platform" size="s" options={[{ value: 'TWITCH', label: 'Twitch', icon: 'twitch' }, { value: 'KICK', label: 'Kick', icon: 'kick' }, { value: 'NAME', label: 'Any name', icon: 'user' }]} value={platform} onchange={(v) => (platform = v)} />
  <label class="adm-find grow">
    <Icon name="search" size={15} />
    <span class="sr-only">{platform === 'NAME' ? 'Name or login' : 'Login or account ID'}</span>
    <input class="ui-input" bind:value={login} onkeydown={down} placeholder={platform === 'NAME' ? 'Name or login' : 'Login or account ID'} autocomplete="off" spellcheck="false" maxlength="100" />
  </label>
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
  <AccountList items={[{ account: f.account, identities: [{ platform: f.platform, login: f.login }] }]} />
{/if}
{/snippet}
</LoadGate>
{/if}
