<script lang="ts">
  // The danger zone at the bottom of the account page: what goes and what stays in two
  // sentences, then a dialog that asks you to type one of your logins. Wrong login, a protected or
  // restricted account and the app-only restriction are answered inline in words; on success the
  // API has already cleared the cookie, so the site forgets you and goes to the front page.
  import { gql } from '$lib/api';
  import { DELETE_ACCOUNT } from '$lib/queries';
  import { leftSession } from '$lib/shell/signout';
  import { notify } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { deleteMessage } from './errors';

  let dialog: HTMLDialogElement;
  let typed = $state('');
  let busy = $state(false);
  let problem = $state('');
  const logins = $derived((session.me?.identities ?? []).map((i) => i.login));

  function open() {
    typed = '';
    problem = '';
    dialog.showModal();
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy || !typed.trim()) return;
    busy = true;
    problem = '';
    try {
      await gql(DELETE_ACCOUNT, { confirm: typed.trim() });
    } catch (err) {
      problem = deleteMessage(err);
      busy = false;
      return;
    }
    dialog.close();
    notify('Your account was deleted.');
    await leftSession();
    busy = false;
  }
</script>

<section class="ui-panel" aria-labelledby="delete-h">
  <header><h2 id="delete-h">Delete account</h2></header>
  <p class="ui-hint pad">
    This happens at once and can’t be undone: your emotes, sets, channels, cosmetics and notifications are deleted. Approved emotes that other people have in their sets stay, shown as made by “Deleted user”.
  </p>
  <div class="ui-actions row pad"><button class="ui-btn danger" type="button" onclick={open}>Delete account…</button></div>
</section>

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="delete-title" aria-describedby="delete-desc">
  <form onsubmit={submit} novalidate>
    <div class="body">
      <h2 id="delete-title">Delete your account?</h2>
      <p id="delete-desc" class="ui-hint">To confirm, type one of your logins{#if logins.length}: <bdi class="ui-mono">{logins.join(', ')}</bdi>{/if}.</p>
      <label class="ui-field">
        <span class="ui-label">Login</span>
        <input
          class="ui-input"
          bind:value={typed}
          autocomplete="off"
          autocapitalize="none"
          spellcheck="false"
          maxlength="100"
          aria-invalid={problem ? 'true' : undefined}
          aria-describedby={problem ? 'delete-err' : undefined}
        />
        {#if problem}<span class="ui-error" id="delete-err" role="alert">{problem}</span>{/if}
      </label>
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn danger" type="submit" disabled={busy || !typed.trim()}>{busy ? 'Deleting…' : 'Delete account'}</button>
    </footer>
  </form>
</dialog>

<style>
  .pad { padding: 10px 14px 12px; }
  .ui-hint.pad { padding-bottom: 12px; }
  .row { padding-top: 0; }
</style>
