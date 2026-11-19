<script lang="ts">
  // Guards unsaved work. While `dirty`, closing or reloading the tab asks the browser's own
  // question, and leaving for another page inside the app opens this dialog: Stay, or Leave
  // and lose the changes.
  import { beforeNavigate, goto } from '$app/navigation';

  let { dirty, what = 'your changes' }: { dirty: boolean; what?: string } = $props();

  let dialog: HTMLDialogElement;
  let target: URL | null = null;
  let leaving = false;

  beforeNavigate((nav) => {
    if (!dirty || leaving || nav.willUnload || !nav.to) return; // a real unload is the beforeunload handler's
    nav.cancel();
    target = nav.to.url;
    if (!dialog.open) dialog.showModal();
  });

  function leave() {
    const to = target;
    leaving = true;
    dialog.close();
    if (to) void goto(to).finally(() => (leaving = false));
    else leaving = false;
  }
</script>

<svelte:window
  onbeforeunload={(e) => {
    if (!dirty) return;
    e.preventDefault();
    e.returnValue = '';
  }}
/>

<dialog class="ui-dialog" bind:this={dialog} aria-labelledby="leave-title" aria-describedby="leave-body">
  <div class="body">
    <h2 id="leave-title">Leave without saving?</h2>
    <p id="leave-body" class="ui-hint">Leaving loses {what}.</p>
  </div>
  <footer>
    <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Stay</button>
    <button class="ui-btn danger" type="button" onclick={leave}>Leave</button>
  </footer>
</dialog>
