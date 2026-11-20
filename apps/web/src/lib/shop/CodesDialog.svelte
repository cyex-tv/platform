<script lang="ts">
  // The one view of a new batch's plain codes (the server keeps only hashes). Until the codes were
  // copied or downloaded, Esc does nothing, the way out asks first ("Close without saving") and
  // closing or reloading the tab asks the browser's own question; after that it closes normally.
  //   <CodesDialog bind:this={d} />   d.show('Launch giveaway', ['LAUNCH-AAAA-…'])
  import Icon from '$lib/components/Icon.svelte';
  import { copy } from '$lib/clipboard.svelte';
  import { clean, count } from '$lib/format';
  import { codesFilename, codesText } from './shop';

  let dialog: HTMLDialogElement;
  let codesBox = $state<HTMLTextAreaElement>();
  let shown = $state<{ name: string; codes: string[] } | null>(null);
  let copied = $state<'idle' | 'copied' | 'failed'>('idle');
  let saved = $state(false);
  let asking = $state(false);

  export function show(name: string, codes: string[]) {
    shown = { name, codes };
    copied = 'idle';
    saved = false;
    asking = false;
    dialog.showModal();
  }

  function cancel(e: Event) {
    if (saved) return;
    e.preventDefault();
    asking = true;
  }

  async function copyAll() {
    if (!shown) return;
    let ok = await copy(codesText(shown.codes));
    if (!ok && codesBox) {
      // The clipboard API is off (an insecure address, a denied permission): copy the visible box, which a modal can focus.
      codesBox.focus();
      codesBox.select();
      try {
        ok = document.execCommand('copy');
      } catch {
        ok = false;
      }
    }
    copied = ok ? 'copied' : 'failed';
    if (ok) saved = true;
  }

  function download() {
    if (!shown) return;
    const url = URL.createObjectURL(new Blob([codesText(shown.codes)], { type: 'text/plain;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = codesFilename(shown.name);
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    saved = true;
  }
</script>

<svelte:window
  onbeforeunload={(e) => {
    if (!shown || saved) return;
    e.preventDefault();
    e.returnValue = '';
  }}
/>

<dialog class="ui-dialog wide" bind:this={dialog} aria-labelledby="codes-title" oncancel={cancel} onclose={() => { shown = null; }}>
  {#if shown}
    <div class="body">
      <h2 id="codes-title">{count(shown.codes.length)} {shown.codes.length === 1 ? 'code' : 'codes'} for <bdi>{clean(shown.name)}</bdi></h2>
      <p class="warn" role="alert"><strong>You won’t see these again.</strong> cyex.tv keeps only a scrambled copy of each code. Copy or download them now, before you close this.</p>
      <label class="ui-field">
        <span class="ui-label">Codes</span>
        <textarea class="ui-input mono codes" bind:this={codesBox} readonly rows="10" wrap="off" onfocus={(e) => e.currentTarget.select()} value={shown.codes.join('\n')}></textarea>
      </label>
      <div class="ui-actions">
        <button class="ui-btn primary" type="button" onclick={copyAll}><Icon name={copied === 'copied' ? 'check' : 'copy'} size={15} />{copied === 'copied' ? 'Copied' : copied === 'failed' ? 'Couldn’t copy. Select the codes instead' : 'Copy all'}</button>
        <button class="ui-btn secondary" type="button" onclick={download}><Icon name="download" size={15} />Download .txt</button>
      </div>
    </div>
    <footer>
      {#if saved}
        <button class="ui-btn primary" type="button" onclick={() => dialog.close()}>Done</button>
      {:else if asking}
        <p class="ask" role="alert">Close without saving? These codes can’t be shown again.</p>
        <button class="ui-btn secondary" type="button" onclick={() => (asking = false)}>Go back</button>
        <button class="ui-btn danger" type="button" onclick={() => dialog.close()}>Close anyway</button>
      {:else}
        <button class="ui-btn secondary" type="button" onclick={() => (asking = true)}>Close without saving</button>
      {/if}
    </footer>
  {/if}
</dialog>

<style>
  .wide { width: min(640px, calc(100vw - 24px)); max-height: calc(100dvh - 24px); }
  .body { overflow-y: auto; overscroll-behavior: contain; }
  h2 { margin: 0; font-size: 18px; overflow-wrap: anywhere; }
  .mono { font-family: var(--font-mono); }
  .codes { resize: vertical; min-height: 160px; font-size: var(--t-small); white-space: pre; }
  .warn { margin: 0; padding: 10px 12px; border: 1px solid var(--red); border-radius: var(--radius); font-size: var(--t-small); line-height: 1.5; }
  .ask { margin: 0 auto 0 0; font-size: var(--t-small); color: var(--red); }
</style>
