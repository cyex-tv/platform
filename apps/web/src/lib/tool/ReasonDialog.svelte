<script lang="ts">
  // Asks for a reason (reject, take down, a report note), then hands it back.
  let {
    title,
    hint,
    action,
    danger = false,
    optional = false,
    max = 1000,
    describe,
    onsubmit,
  }: {
    title: string;
    hint: string;
    action: string;
    danger?: boolean;
    optional?: boolean;
    /** The longest reason in characters (the server's limit for what this is used for). */
    max?: number;
    /** Turns a failed submit into a sentence shown in the dialog; without it the page shows why. */
    describe?: (e: unknown) => string;
    onsubmit: (reason: string) => Promise<void> | void;
  } = $props();

  let dialog: HTMLDialogElement;
  let reason = $state('');
  let busy = $state(false);
  let problem = $state('');

  export function open() {
    reason = '';
    problem = '';
    dialog.showModal();
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (busy || (!optional && !reason.trim())) return;
    busy = true;
    problem = '';
    try {
      await onsubmit(reason.trim());
      dialog.close();
    } catch (err) {
      // The dialog stays open to try again; it says why when it was given the words for it.
      if (describe) problem = describe(err);
    } finally {
      busy = false;
    }
  }
</script>

<dialog class="ui-dialog" bind:this={dialog} aria-label={title}>
  <form onsubmit={submit}>
    <div class="body">
      <h2>{title}</h2>
      <label class="ui-field">
        <span class="ui-label">{optional ? 'Note (optional)' : 'Reason'}</span>
        <textarea class="ui-input" rows="3" maxlength={max} bind:value={reason} aria-describedby="rd-hint"></textarea>
        <span class="ui-hint" id="rd-hint">{hint}</span>
      </label>
      {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
    </div>
    <footer>
      <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
      <button class="ui-btn {danger ? 'danger' : 'primary'}" type="submit" disabled={busy || (!optional && !reason.trim())}>{action}</button>
    </footer>
  </form>
</dialog>

<style>
  textarea { resize: vertical; min-height: 72px; }
</style>
