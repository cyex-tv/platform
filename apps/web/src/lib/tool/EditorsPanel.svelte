<script lang="ts">
  // A set's editors: the owner adds people by their Twitch or Kick login (they need to have
  // signed in to cyex.tv once) and removes them.
  import { gql } from '$lib/api';
  import type { EditSetQuery, Platform } from '$lib/gql/graphql';
  import { ADD_SET_EDITOR, FIND_ACCOUNT, REMOVE_SET_EDITOR } from '$lib/queries';
  import Avatar from './Avatar.svelte';
  import { session } from './session.svelte';
  import { fail, notify } from './notices.svelte';

  let { setId, owner, editors = $bindable() }: { setId: string; owner: boolean; editors: EditSetQuery['setEditors'] } = $props();

  let platform = $state<Platform>('TWITCH');
  let login = $state('');
  let problem = $state<string | null>(null);
  let busy = $state(false);

  async function add(e: SubmitEvent) {
    e.preventDefault();
    const name = login.trim().replace(/^@/, '').toLowerCase();
    if (!name) return;
    busy = true;
    problem = null;
    try {
      const found = (await gql(FIND_ACCOUNT, { platform, login: name })).channelByLogin;
      if (!found) {
        problem = `No one called ${name} has signed in to cyex.tv with ${platform === 'TWITCH' ? 'Twitch' : 'Kick'}.`;
        return;
      }
      if (found.account.id === session.me?.id) {
        problem = 'That’s you. Owners can edit their sets already.';
        return;
      }
      const added = (await gql(ADD_SET_EDITOR, { setId, accountId: found.account.id })).addSetEditor;
      if (added) editors = [...editors, found.account];
      notify(added ? `${found.account.displayName} can edit this set now.` : `${found.account.displayName} edits it already.`);
      login = '';
    } catch (err) {
      fail(err);
    } finally {
      busy = false;
    }
  }

  async function remove(id: string, name: string) {
    try {
      await gql(REMOVE_SET_EDITOR, { setId, accountId: id });
      editors = editors.filter((e) => e.id !== id);
      notify(`${name} no longer edits this set.`);
    } catch (err) {
      fail(err);
    }
  }
</script>

<section class="ui-panel">
  <header><h2>Editors</h2><span class="ui-hint ui-mono">{editors.length}</span></header>
  {#if editors.length}
    <ul class="ui-list">
      {#each editors as e (e.id)}
        <li>
          <Avatar src={e.avatar} name={e.displayName} size={24} />
          <span class="who">{e.displayName}</span>
          {#if owner}
            <button class="ui-icon" type="button" aria-label="Remove {e.displayName}" title="Remove" onclick={() => remove(e.id, e.displayName)}>
              <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden="true"><path d="M6 6l12 12M18 6 6 18" /></svg>
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="ui-hint pad">No editors. Editors can add, rename and remove emotes, but not delete the set.</p>
  {/if}
  {#if owner}
    <form class="add" onsubmit={add}>
      <div class="ui-seg" role="group" aria-label="Platform">
        <button type="button" aria-pressed={platform === 'TWITCH'} onclick={() => (platform = 'TWITCH')}>Twitch</button>
        <button type="button" aria-pressed={platform === 'KICK'} onclick={() => (platform = 'KICK')}>Kick</button>
      </div>
      <div class="row">
        <input class="ui-input" bind:value={login} placeholder="Login" aria-label="Their login" autocomplete="off" spellcheck="false" />
        <button class="ui-btn secondary" type="submit" disabled={busy || !login.trim()}>Add</button>
      </div>
      {#if problem}<p class="ui-error">{problem}</p>{/if}
    </form>
  {/if}
</section>

<style>
  .who { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .pad { padding: 12px 14px; }
  .add { display: grid; gap: 8px; padding: 10px; border-top: 1px solid var(--border); }
  .row { display: flex; gap: 8px; }
  .row .ui-input { background: var(--bg); }
  .ui-seg { justify-self: start; }
</style>
