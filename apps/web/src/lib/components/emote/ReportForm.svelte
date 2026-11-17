<script lang="ts">
  // The report form, shown in the overflow menu and in the review block. Signed out, it offers
  // sign-in instead; once sent, it says what happens next.
  import { page } from '$app/state';
  import type { Viewer } from './viewer.svelte';

  let { viewer, name, oncancel }: { viewer: Viewer; name: string; oncancel?: () => void } = $props();

  const id = `report-${Math.random().toString(36).slice(2, 9)}`;
  let reason = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function send(e: SubmitEvent) {
    e.preventDefault();
    if (!reason.trim() || busy) return;
    busy = true;
    error = null;
    try {
      await viewer.report(reason.trim());
    } catch (err) {
      error = err instanceof Error ? err.message : 'That didn’t work.';
    } finally {
      busy = false;
    }
  }
</script>

<div class="report" aria-live="polite">
  {#if viewer.reported}
    <p class="head">Thanks, it’s reported.</p>
    <p class="fine">A moderator reads every report. If {name} breaks the rules, it comes down.</p>
  {:else if viewer.who === 'out'}
    <p class="head">Sign in to report it.</p>
    <p class="fine">Reports come from signed-in accounts, so moderators can ask about them.</p>
    <a class="btn btn-s" href="/signin?back={encodeURIComponent(page.url.pathname)}">Sign in</a>
  {:else if viewer.who === 'none'}
    <p class="fine">cyex.tv can’t be reached right now, so reports can’t be sent. Try again in a moment.</p>
  {:else}
    <form onsubmit={send}>
      <label class="head" for={id}>What’s wrong with {name}?</label>
      <textarea
        {id}
        class="input area"
        bind:value={reason}
        maxlength="500"
        rows="3"
        placeholder="It copies someone else’s art, it’s hateful, …"
        disabled={viewer.who === 'loading'}
        {@attach (el) => el.focus({ preventScroll: true })}
      ></textarea>
      <p class="fine">A moderator reads it. The creator doesn’t see who sent it.</p>
      <div class="row">
        <button class="btn btn-s" type="submit" disabled={busy || !reason.trim()}>{busy ? 'Sending…' : 'Send report'}</button>
        {#if oncancel}<button class="btn btn-s ghost" type="button" onclick={oncancel}>Cancel</button>{/if}
      </div>
      {#if error}<p class="fine err" role="alert">{error}</p>{/if}
    </form>
  {/if}
</div>

<style>
  p { margin: 0; }
  .report { display: grid; gap: 8px; padding: 8px; }
  form { display: grid; gap: 8px; }
  .head { font-size: var(--t-ui); font-weight: 500; color: var(--text); }
  .area { height: auto; padding: 10px 12px; resize: vertical; line-height: 1.5; }
  .row { display: flex; gap: 8px; margin-top: 4px; }
  .ghost { border-color: transparent; background: none; }
  .err { color: var(--text); }
  .report > .btn { justify-self: start; margin-top: 4px; }
</style>
