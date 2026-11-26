<script lang="ts">
  import Dialog from '$lib/components/Dialog.svelte';
  // Guards unsaved work. While `dirty`, closing or reloading the tab asks the browser's own
  // question, and leaving for another page inside the app opens this dialog: Stay, or Leave
  // and lose the changes. `inApp={false}` keeps only the browser's question, for work that
  // survives moving between pages (the site-wide upload dialog keeps its queue).
  import { beforeNavigate, goto } from '$app/navigation';

  let { dirty, what = 'your changes', inApp = true }: { dirty: boolean; what?: string; inApp?: boolean } = $props();

  let dialog: Dialog;
  let target: URL | null = null;
  let leaving = false;

  beforeNavigate((nav) => {
    if (!dirty || !inApp || leaving || nav.willUnload || !nav.to) return; // a real unload is the beforeunload handler's
    nav.cancel();
    target = nav.to.url;
    if (!dialog.isOpen()) dialog.open();
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

<Dialog bind:this={dialog} title="Leave without saving?" titleId="leave-title" describedby="leave-body">
  <p id="leave-body" class="ui-hint">Leaving loses {what}.</p>
  {#snippet footer()}
    <button class="ui-btn secondary" type="button" onclick={() => dialog.close()}>Stay</button>
    <button class="ui-btn danger" type="button" onclick={leave}>Leave</button>
  {/snippet}
</Dialog>
