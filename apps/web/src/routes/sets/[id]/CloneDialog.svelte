<script lang="ts">
  // "Clone to my sets": asks for the new set's name, copies, then opens the copy in the editor.
  // What didn't come along (over the size, not approved, others' private emotes) is said in a
  // notice on the next page. Errors stay in the dialog, in words.
  // Every set is public (ADR 0017, amendment 2026/11/22): the copy shows on the cloner's profile.
  //   <CloneDialog bind:this={dlg} id={set.id} name={set.name} />  dlg.open()
  import Dialog from '$lib/components/Dialog.svelte';
  import { goto } from '$app/navigation';
  import { ApiError, gql } from '$lib/api';
  import { clean } from '$lib/format';
  import { CLONE_SET } from '$lib/queries';
  import { cloneError, cloneOutcome } from '$lib/sets/public';
  import { forgetProfile } from '$lib/profile/data.svelte';
  import { notify } from '$lib/tool/notices.svelte';

  let { id, name: source }: { id: string; name: string } = $props();

  let dialog: Dialog;
  let name = $state('');

  let busy = $state(false);
  let tried = $state(false);
  let failure = $state('');
  const nameError = $derived(tried && !name.trim() ? 'Give it a name.' : failure);

  export function open() {
    name = clean(source).slice(0, 100);
    tried = false;
    failure = '';

    dialog.open();
  }

  async function clone(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    tried = true;
    failure = '';
    const trimmed = name.trim();
    if (!trimmed) return;
    busy = true;
    try {
      const { cloneSet } = await gql(CLONE_SET, { id, name: trimmed });
      dialog.close();
      forgetProfile('library:');
      notify(cloneOutcome(cloneSet.copied, cloneSet.skipped));
      await goto(`/me/sets/${cloneSet.set.id}`);
    } catch (err) {
      failure = err instanceof ApiError ? cloneError(err.code, err.message) : 'That didn’t work. Try again.';
    } finally {
      busy = false;
    }
  }
</script>

<Dialog bind:this={dialog} title="Clone to my sets" titleId="clone-set-title" onsubmit={clone}>
  <label class="ui-field">
    <span class="ui-label">Name of your copy</span>
    <input class="ui-input" bind:value={name} maxlength="100" autocomplete="off"
      aria-invalid={nameError ? 'true' : undefined} aria-describedby="clone-set-hint{nameError ? ' clone-set-err' : ''}" />
    {#if nameError}<span class="ui-error" id="clone-set-err" role="alert">{nameError}</span>{/if}
    <span class="ui-hint" id="clone-set-hint">
      Emotes that aren’t approved yet, and other people’s private emotes, stay out.
    </span>
  </label>
  {#snippet footer()}
    <button class="btn btn-s" type="button" onclick={() => dialog.close()}>Cancel</button>
    <button class="btn btn-s btn-primary" type="submit" disabled={busy}>{busy ? 'Copying…' : 'Clone'}</button>
  {/snippet}
</Dialog>
