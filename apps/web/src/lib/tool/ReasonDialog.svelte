<script lang="ts">
  // Asks for a reason (reject, take down, a report note), then hands it back.
  import Dialog from '$lib/components/Dialog.svelte';
  let {
    title,
    hint,
    action,
    danger = false,
    optional = false,
    max = 1000,
    describe,
    presets = [],
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
    /** Ready reasons: one click fills the field (and can still be edited). */
    presets?: string[];
    onsubmit: (reason: string) => Promise<void> | void;
  } = $props();
  let field = $state<HTMLTextAreaElement>();

  let dialog: Dialog;
  let reason = $state('');
  let busy = $state(false);
  let problem = $state('');

  export function open() {
    reason = '';
    problem = '';
    dialog.open();
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

<Dialog bind:this={dialog} {title} onsubmit={submit}>
  <label class="ui-field">
    <span class="ui-label">{optional ? 'Note (optional)' : 'Reason'}</span>
    <textarea class="ui-input" rows="3" maxlength={max} bind:value={reason} bind:this={field} aria-describedby="rd-hint"></textarea>
    <span class="ui-hint" id="rd-hint">{hint}</span>
  </label>
  {#if presets.length}
    <div class="presets" role="group" aria-label="Common reasons">
      {#each presets as p (p)}
        <button class="ui-btn secondary small" type="button" aria-pressed={reason.trim() === p} onclick={() => { reason = p; field?.focus(); }}>{p}</button>
      {/each}
    </div>
  {/if}
  {#if problem}<p class="ui-error" role="alert">{problem}</p>{/if}
  {#snippet footer()}
    <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Cancel</button>
    <button class="ui-btn {danger ? 'danger' : 'primary'}" type="submit" disabled={busy || (!optional && !reason.trim())}>{action}</button>
  {/snippet}
</Dialog>

<style>
  textarea { resize: vertical; min-height: 72px; }
  .presets { display: flex; flex-wrap: wrap; gap: 6px; }
  .presets [aria-pressed='true'] { font-weight: 700; color: var(--text); }
</style>
