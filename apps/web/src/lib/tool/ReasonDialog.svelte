<script lang="ts">
  // Asks for a reason (reject, take down, a report note), then hands it back.
  let {
    title,
    hint,
    action,
    danger = false,
    optional = false,
    onsubmit,
  }: {
    title: string;
    hint: string;
    action: string;
    danger?: boolean;
    optional?: boolean;
    onsubmit: (reason: string) => Promise<void> | void;
  } = $props();

  let dialog: HTMLDialogElement;
  let reason = $state('');
  let busy = $state(false);

  export function open() {
    reason = '';
    dialog.showModal();
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!optional && !reason.trim()) return;
    busy = true;
    try {
      await onsubmit(reason.trim());
      dialog.close();
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
        <textarea class="ui-input" rows="3" maxlength="1000" bind:value={reason}></textarea>
        <span class="ui-hint">{hint}</span>
      </label>
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
