<script lang="ts">
  import Segmented from '$lib/components/Segmented.svelte';
  // Give an account a badge or paint, with a reason for the audit log (cosmetics.grant). Either
  // the cosmetic is fixed (from its admin row) and you find the account by its Twitch or Kick
  // login, or the account is fixed (admin Users) and you pick from what it doesn't own yet.
  import { gql } from '$lib/api';
  import type { CosmeticKind, Platform } from '$lib/gql/graphql';
  import { iso } from '$lib/format';
  import { GRANT_COSMETIC, USER } from '$lib/queries';
  import { fail, notify } from '$lib/tool/notices.svelte';

  type Choice = { id: string; name: string };
  type Props = {
    kind: CosmeticKind;
    /** The cosmetic to give; or leave it out and pass `choices`. */
    cosmetic?: Choice;
    choices?: Choice[];
    /** The account to give it to; or leave it out to find one by login. */
    account?: { id: string; displayName: string };
    ongranted?: () => void;
  };
  let { kind, cosmetic, choices = [], account, ongranted }: Props = $props();

  let dialog: HTMLDialogElement;
  let platform = $state<Platform>('TWITCH');
  let login = $state('');
  let pick = $state('');
  let reason = $state('');
  let busy = $state(false);
  let problem = $state('');
  const noun = $derived(kind === 'BADGE' ? 'badge' : 'paint');

  // The cosmetic for this opening (a list's row), else the `cosmetic` prop.
  let chosen = $state<Choice | undefined>();
  const fixed = $derived(chosen ?? cosmetic);

  export function open(what?: Choice) {
    chosen = what;
    login = '';
    reason = '';
    problem = '';
    pick = choices[0]?.id ?? '';
    dialog.showModal();
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    problem = '';
    const what = fixed ?? choices.find((c) => c.id === pick);
    if (!what) return void (problem = `Pick a ${noun}.`);
    if (!reason.trim()) return void (problem = 'Give a reason; it goes in the audit log.');
    busy = true;
    try {
      let to = account;
      if (!to) {
        const name = login.trim().replace(/^@/, '').toLowerCase();
        if (!name) return void (problem = 'Enter their login.');
        const found = (await gql(USER, { platform, login: name })).channelByLogin;
        if (!found) return void (problem = 'Nobody with that login has signed in to cyex.tv.');
        to = found.account;
      }
      await gql(GRANT_COSMETIC, { input: { accountId: to.id, kind, cosmeticId: what.id, reason: reason.trim() } });
      dialog.close();
      notify(`${iso(what.name)} given to ${iso(to.displayName)}.`);
      ongranted?.();
    } catch (err) {
      fail(err);
    } finally {
      busy = false;
    }
  }
</script>

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="grant-title">
  <form onsubmit={submit} novalidate>
    <div class="body">
      <h2 id="grant-title">Give {fixed ? iso(fixed.name) : `a ${noun}`}{account ? ` to ${iso(account.displayName)}` : ''}</h2>
      {#if !account}
        <div class="ui-field">
          <span class="ui-label" id="grant-plat">Account</span>
          <div class="who">
            <Segmented label="Account platform" options={[{ value: 'TWITCH', label: 'Twitch', icon: 'twitch' }, { value: 'KICK', label: 'Kick', icon: 'kick' }]} value={platform} onchange={(v) => (platform = v)} />
            <input class="ui-input" bind:value={login} placeholder="Login" aria-label="Login" autocomplete="off" spellcheck="false" />
          </div>
        </div>
      {/if}
      {#if !fixed}
        <label class="ui-field">
          <span class="ui-label">{kind === 'BADGE' ? 'Badge' : 'Paint'}</span>
          {#if choices.length}
            <select class="ui-input" bind:value={pick}>
              {#each choices as c (c.id)}<option value={c.id}>{iso(c.name)}</option>{/each}
            </select>
          {:else}
            <span class="ui-hint">They own every {noun} already.</span>
          {/if}
        </label>
      {/if}
      <label class="ui-field">
        <span class="ui-label">Reason</span>
        <textarea class="ui-input" rows="3" maxlength="500" bind:value={reason} placeholder="Why they get it, for the audit log"></textarea>
      </label>
      {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn primary" type="submit" disabled={busy || (!fixed && !choices.length)}>{busy ? 'Giving…' : 'Give'}</button>
    </footer>
  </form>
</dialog>

<style>
  .who { display: flex; flex-wrap: wrap; gap: 8px; }
  .who .ui-input { flex: 1; min-width: 160px; }
  textarea { resize: vertical; min-height: 72px; }
</style>
