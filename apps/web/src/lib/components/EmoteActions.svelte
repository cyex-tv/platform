<script lang="ts">
  // An emote's actions: add it to one of your sets, copy its link and, in the page head, a menu
  // with Copy name and Report. Signed out, "Add to set" becomes "Sign in to add". `variant="bar"`
  // is the phone's sticky bar (bigger targets, Add to set takes the width). Pass the page's `viewer` so every
  // copy of the actions shares one sign-in check; without it the component makes its own.
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { clean } from '$lib/format';
  import { useCopied } from '$lib/clipboard.svelte';
  import Icon from './Icon.svelte';
  import Popover from './Popover.svelte';
  import ReportForm from './emote/ReportForm.svelte';
  import { Viewer } from './emote/viewer.svelte';

  type Props = { emoteId: string; name: string; viewer?: Viewer; variant?: 'head' | 'bar' };
  let { emoteId, name, viewer: given, variant = 'head' }: Props = $props();

  // svelte-ignore state_referenced_locally
  const viewer = given ?? new Viewer(emoteId);
  onMount(() => void viewer.load());

  let addOpen = $state(false);
  let moreOpen = $state(false);
  let mode = $state<'menu' | 'report'>('menu');
  const linkCopy = useCopied();
  const nameCopy = useCopied();
  const shown = $derived(clean(name));

  const signIn = $derived(`/signin?back=${encodeURIComponent(page.url.pathname)}`);
  const link = () => page.url.origin + page.url.pathname;
  const said = $derived(
    linkCopy.copied ? 'Link copied' : linkCopy.failed ? 'Couldn’t copy' : nameCopy.copied ? 'Name copied' : nameCopy.failed ? 'Couldn’t copy' : '',
  );
</script>

<div class="actions-row {variant}">
  {#if viewer.who === 'out'}
    <a class="btn btn-primary btn-s add" href={signIn}>
      <Icon name="enter" />Sign in to add
    </a>
  {:else}
    <Popover label="Add {name} to a set" bind:open={addOpen} sheet width="300px" align={variant === 'head' ? 'end' : 'start'}>
      {#snippet trigger(props)}
        <button class="btn btn-primary btn-s add" type="button" {...props}><Icon name="plus" />Add to set</button>
      {/snippet}
      <div class="sets" aria-live="polite">
        {#if viewer.who === 'loading'}
          <p class="note fine">Checking your sets…</p>
        {:else if viewer.who === 'none'}
          <p class="note fine">cyex.tv can’t be reached right now. Try again in a moment.</p>
        {:else if viewer.own.length === 0}
          <p class="note fine">You have no sets yet. <a class="link" href="/me/sets">Make one</a></p>
        {:else}
          <p class="note fine">Add {shown} to</p>
          <ul>
            {#each viewer.own as s (s.id)}
              {@const inIt = viewer.has(s)}
              <li>
                <button
                  type="button"
                  class="set"
                  aria-disabled={inIt || viewer.adding !== null}
                  onclick={() => !inIt && viewer.add(s.id)}
                >
                  <span class="set-name" title={s.name}>{s.name}</span>
                  {#if viewer.adding === s.id}
                    <span class="state fine">Adding…</span>
                  {:else if inIt}
                    <span class="state fine"><Icon name="check" size={14} />{viewer.added === s.id ? 'Added' : 'In this set'}</span>
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
          {#if viewer.addError}<p class="note fine err" role="alert">{viewer.addError}</p>{/if}
        {/if}
      </div>
      {#snippet footer()}
        <a class="link fine" href="/me/sets">Manage your sets</a>
      {/snippet}
    </Popover>
  {/if}

  <button class="btn btn-s copy" type="button" onclick={() => linkCopy.copy(link())}>
    <Icon name={linkCopy.copied ? 'check' : 'copy'} />
    <span class="stack">
      <span class="lbl" class:off={linkCopy.state !== 'idle'}>Copy link</span>
      <span class="lbl" class:off={!linkCopy.copied}>Copied</span>
      <span class="lbl" class:off={!linkCopy.failed}>Couldn’t copy</span>
    </span>
  </button>
  <span class="sr-only" role="status">{said}</span>

  <Popover label="More actions" bind:open={moreOpen} align="end" width={mode === 'report' ? '340px' : '220px'} sheet onclose={() => (mode = 'menu')}>
      {#snippet trigger(props)}
        <button class="btn btn-s more" type="button" aria-label="More actions" {...props}><Icon name="more" size={18} /></button>
      {/snippet}
      {#if mode === 'menu'}
        <ul class="menu">
          <li>
            <button type="button" class="item" onclick={() => nameCopy.copy(name)}>
              <Icon name={nameCopy.copied ? 'check' : 'copy'} />{nameCopy.copied ? 'Name copied' : nameCopy.failed ? 'Couldn’t copy' : 'Copy name'}
            </button>
          </li>
          <li>
            <button type="button" class="item" onclick={() => (mode = 'report')}><Icon name="flag" />Report</button>
          </li>
        </ul>
      {:else}
        <ReportForm {viewer} {name} oncancel={() => (moreOpen = false)} />
      {/if}
    </Popover>
</div>

<style>
  p { margin: 0; }
  .actions-row { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; }
  .btn :global(.icon) { flex: none; }
  .btn { gap: 8px; }
  .more { width: 38px; padding: 0; }
  .bar { flex-wrap: nowrap; }
  .bar > :global(.pop-root:first-child), .bar > .add { flex: 1; }
  .bar .add { flex: 1; width: 100%; height: 44px; }
  .bar .copy { height: 44px; position: relative; }
  .bar .more { width: 44px; height: 44px; }

  .stack { display: inline-grid; justify-items: center; }
  .lbl { grid-area: 1 / 1; }
  .lbl.off { visibility: hidden; }
  .bar .add { min-width: 0; }
  .sets { display: grid; gap: 2px; grid-template-columns: minmax(0, 1fr); }
  .note { padding: 6px 8px; }
  .err { color: var(--text); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; grid-template-columns: minmax(0, 1fr); }
  .set, .item { width: 100%; display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 0 10px;
    border: 0; border-radius: var(--radius-btn); background: none; color: var(--text); font: inherit; font-size: var(--t-ui);
    text-align: left; cursor: pointer; transition: background-color var(--dur-color), transform var(--dur-press) ease-out;
    touch-action: manipulation; user-select: none; -webkit-user-select: none; }
  .item { color: var(--text); }
  .item :global(.icon) { color: var(--muted); }
  .set:active:not([aria-disabled='true']), .item:active { transform: scale(var(--press)); }
  .set[aria-disabled='true'] { cursor: default; }
  .set-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .set[aria-disabled='true'] .set-name { color: var(--muted); }
  .state { display: inline-flex; align-items: center; gap: 4px; flex: none; }
  .set:focus-visible, .item:focus-visible { outline-offset: -2px; }
  @media (max-width: 360px) {
    .bar .copy .stack { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
    .bar .copy { width: 44px; padding: 0; justify-content: center; }
  }
  @media (hover: hover) and (pointer: fine) {
    .set:hover:not([aria-disabled='true']), .item:hover { background: var(--surface-2); }
  }
</style>
