<script lang="ts">
  // The danger zone at the bottom of the account page: the 30-day undo window and what goes and
  // what stays, then a dialog that asks you to type one of your logins. Wrong login, a protected
  // or restricted account and the app-only restriction are answered inline in words; on success
  // the API has already cleared the cookie, so the site forgets you and goes to the front page.
  // Signing in again within 30 days lands on /restore (ADR 0019).
  import { gql } from '$lib/api';
  import Dialog from '$lib/components/Dialog.svelte';
  import Row from '$lib/components/Row.svelte';
  import Section from '$lib/components/Section.svelte';
  import { DELETE_ACCOUNT } from '$lib/queries';
  import { leftSession } from '$lib/shell/signout';
  import { notify } from '$lib/tool/notices.svelte';
  import { session } from '$lib/tool/session.svelte';
  import { deleteMessage } from './errors';

  let dialog: Dialog;
  let typed = $state('');
  let busy = $state(false);
  let problem = $state('');
  const logins = $derived((session.me?.identities ?? []).map((i) => i.login));

  function open() {
    typed = '';
    problem = '';
    dialog.open();
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
    notify('Your account is hidden. Sign in within 30 days to undo it.');
    await leftSession();
    busy = false;
  }
</script>

<Section title="Delete account" tone="danger" id="delete-h">
  <Row label="Delete account" hint="Your account is hidden at once and deleted for good after 30 days. Sign in before then to undo it. After that, your emotes, sets, channels, cosmetics and notifications are gone; approved emotes that other people have in their sets stay, shown as made by “Deleted user”.">
    <button class="ui-btn danger small" type="button" onclick={open}>Delete account…</button>
  </Row>
</Section>

<Dialog bind:this={dialog} title="Delete your account?" titleId="delete-title" describedby="delete-desc" onsubmit={submit}>
  <p id="delete-desc" class="ui-hint">Your account is hidden at once and deleted for good after 30 days. Sign in before then to undo it. To confirm, type one of your logins{#if logins.length}: <bdi class="ui-mono">{logins.join(', ')}</bdi>{/if}.</p>
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
  {#snippet footer()}
    <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
    <button class="ui-btn danger" type="submit" disabled={busy || !typed.trim()}>{busy ? 'Deleting…' : 'Delete account'}</button>
  {/snippet}
</Dialog>
