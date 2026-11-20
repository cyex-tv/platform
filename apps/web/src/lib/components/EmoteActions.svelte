<script lang="ts">
  // An emote's actions, kept together: Add to set (the sets on your own channels first, marked "On
  // your channel"), Edit for the owner and for staff who may change any emote (`editHref`), and a
  // heart (your favourites, signed in), and a More menu with Copy link, Copy name, Hide for me and
  // Hide everything by the creator (signed in; Show again once hidden), Download and Report (the
  // last two open in the same panel).
  // Signed out, "Add to set" becomes "Sign in to add". `variant="bar"` is the phone's fixed bar
  // (44px targets, Add to set takes the width, Edit and More shrink to icons). Pass the page's
  // `viewer` so every copy of the actions shares one sign-in check.
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { clean, iso } from '$lib/format';
  import { useCopied } from '$lib/clipboard.svelte';
  import Icon from './Icon.svelte';
  import { unreachableMessage } from './problem';
  import Name from './Name.svelte';
  import SwapIcon from './SwapIcon.svelte';
  import Popover from './Popover.svelte';
  import EmoteDownload from './emote/EmoteDownload.svelte';
  import type { PageEmote } from './emote/emote-page';
  import ReportForm from './emote/ReportForm.svelte';
  import { Viewer } from './emote/viewer.svelte';
  import { roving } from './roving';
  import { session } from '$lib/tool/session.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import { listError, personal } from '$lib/personal/lists.svelte';

  type Props = {
    emoteId: string;
    name: string;
    viewer?: Viewer;
    variant?: 'head' | 'bar';
    picture?: Pick<PageEmote, 'host' | 'animated' | 'files'>;
    /** Where Edit goes, for people allowed to change the emote; no Edit button without it. */
    editHref?: string | null;
    /** Whether the emote can be added to sets (approved and public); otherwise only Edit and More. */
    addable?: boolean;
    /** Whether Report is offered (approved emotes). */
    reportable?: boolean;
    /** The creator, for "Hide everything by …". */
    owner?: { id: string; displayName: string } | null;
  };
  let { emoteId, name, viewer: given, variant = 'head', picture, editHref = null, addable = true, reportable = true, owner = null }: Props = $props();

  // svelte-ignore state_referenced_locally
  const viewer = given ?? new Viewer(emoteId);
  onMount(() => void viewer.load());

  let addOpen = $state(false);
  let moreOpen = $state(false);
  let mode = $state<'menu' | 'download' | 'report'>('menu');
  const linkCopy = useCopied();
  const nameCopy = useCopied();
  const shown = $derived(clean(name));
  const canDownload = $derived(!!picture && picture.files.length > 0);

  const signIn = $derived(`/signin?back=${encodeURIComponent(page.url.pathname)}`);
  const link = () => page.url.origin + page.url.pathname;
  const said = $derived(
    linkCopy.copied ? 'Link copied' : linkCopy.failed ? 'Couldn’t copy' : nameCopy.copied ? 'Name copied' : nameCopy.failed ? 'Couldn’t copy' : '',
  );
  // What adding to a set did, in one short line (not the whole list read out again).
  const setName = (id: string | null) => clean(viewer.own.find((s) => s.id === id)?.name ?? '');
  const addSaid = $derived(
    viewer.adding ? `Adding to ${setName(viewer.adding)}…` : viewer.added ? `Added to ${setName(viewer.added)}` : '',
  );
  $effect(() => personal.ensure());
  const signedIn = $derived(!!session.me);
  const fav = $derived(personal.favourites.has(emoteId));
  const hidden = $derived(personal.hiddenEmotes.has(emoteId));
  const creatorHidden = $derived(!!owner && personal.hiddenCreators.has(owner.id));
  const creatorName = $derived(clean(owner?.displayName ?? ''));
  const canHideCreator = $derived(!!owner && owner.id !== session.me?.id);
  let favBusy = $state(false);

  async function toggleFavourite() {
    if (favBusy) return;
    favBusy = true;
    try {
      if (fav) await personal.unfavourite(emoteId);
      else await personal.favourite(emoteId);
    } catch (e) {
      notify(listError(e), true);
    } finally {
      favBusy = false;
    }
  }

  async function run(change: () => Promise<unknown>, done: string) {
    moreOpen = false;
    try {
      await change();
      notify(done);
    } catch (e) {
      notify(listError(e), true);
    }
  }
  const favSaid = $derived(fav ? 'In your favourites' : '');
  const moreWidth = $derived(mode === 'report' ? '340px' : mode === 'download' ? '300px' : '220px');
</script>

<div class="actions-row {variant}">
  {#if addable}
    {#if viewer.who === 'out'}
      <a class="btn btn-primary btn-s add" href={signIn}><Icon name="enter" />Sign in to add</a>
    {:else}
      <Popover label="Add {iso(name)} to a set" bind:open={addOpen} sheet width="300px">
        {#snippet trigger(props)}
          <button class="btn btn-primary btn-s add" type="button" {...props}><Icon name="plus" />Add to set</button>
        {/snippet}
        <div class="sets">
          <p class="sr-only" role="status">{addSaid}</p>
          {#if viewer.who === 'loading'}
            <p class="note fine">Checking your sets…</p>
          {:else if viewer.who === 'none'}
            <p class="note fine">{unreachableMessage()}</p>
          {:else if viewer.own.length === 0}
            <p class="note fine">You have no sets yet. <a class="link" href="/me/sets">Make one</a></p>
          {:else}
            <p class="note fine">Add <bdi>{shown}</bdi> to</p>
            <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
            <ul onkeydown={roving}>
              {#each viewer.own as s (s.id)}
                {@const inIt = viewer.has(s)}
                {@const live = viewer.live.has(s.id)}
                <li>
                  <button type="button" class="set" aria-disabled={inIt || viewer.adding !== null} onclick={() => !inIt && viewer.add(s.id)}>
                    <span class="set-text">
                      <span class="set-name" title={clean(s.name)}><Name value={s.name} /></span>
                      {#if live}<span class="live fine"><Icon name="tv" size={12} />On your channel</span>{/if}
                    </span>
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
  {/if}

  {#if editHref}
    <a class="btn btn-s edit" href={editHref} aria-label={variant === 'bar' ? 'Edit' : undefined}><Icon name="pencil" /><span class="word">Edit</span></a>
  {/if}

  {#if signedIn}
    <button class="btn btn-s heart" class:on={fav} type="button" aria-pressed={fav} aria-busy={favBusy}
      aria-label={fav ? `Remove ${iso(name)} from favourites` : `Favourite ${iso(name)}`} title={fav ? 'Remove from favourites' : 'Favourite'}
      onclick={toggleFavourite}>
      <Icon name="heart" />
    </button>
    <span class="sr-only" role="status">{favSaid}</span>
  {/if}

  <span class="sr-only" role="status">{said}</span>
  <Popover label="More actions" bind:open={moreOpen} align="end" width={moreWidth} sheet onclose={() => (mode = 'menu')}>
    {#snippet trigger(props)}
      <button class="btn btn-s more" type="button" aria-label={variant === 'bar' ? 'More actions' : undefined} {...props}>
        <span class="word">More</span><Icon name={variant === 'bar' ? 'more' : 'chevron-down'} size={variant === 'bar' ? 18 : 16} />
      </button>
    {/snippet}
    {#if mode === 'menu'}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <ul class="menu" onkeydown={roving}>
        <li>
          <button type="button" class="item" onclick={() => linkCopy.copy(link())}>
            <SwapIcon a="link" b="check" on={linkCopy.copied} />{linkCopy.copied ? 'Link copied' : linkCopy.failed ? 'Couldn’t copy' : 'Copy link'}
          </button>
        </li>
        <li>
          <button type="button" class="item" onclick={() => nameCopy.copy(name)}>
            <SwapIcon a="copy" b="check" on={nameCopy.copied} />{nameCopy.copied ? 'Name copied' : nameCopy.failed ? 'Couldn’t copy' : 'Copy name'}
          </button>
        </li>
        {#if signedIn}
          <li>
            {#if hidden}
              <button type="button" class="item" onclick={() => run(() => personal.unhideEmote(emoteId), `${shown} shows for you again.`)}><Icon name="eye" />Show again for me</button>
            {:else}
              <button type="button" class="item" onclick={() => run(() => personal.hideEmote(emoteId), `Hid ${shown} for you. Undo it here or in Settings.`)}><Icon name="eye-off" />Hide for me</button>
            {/if}
          </li>
          {#if canHideCreator && owner}
            <li>
              {#if creatorHidden}
                <button type="button" class="item" onclick={() => run(() => personal.unhideCreator(owner.id), `${creatorName}’s emotes show for you again.`)}>
                  <Icon name="user-plus" /><span class="lbl">Show everything by <bdi>{creatorName}</bdi></span>
                </button>
              {:else}
                <button type="button" class="item" onclick={() => run(() => personal.hideCreator(owner.id), `Hid everything by ${creatorName} for you.`)}>
                  <Icon name="user-x" /><span class="lbl">Hide everything by <bdi>{creatorName}</bdi></span>
                </button>
              {/if}
            </li>
          {/if}
        {/if}
        {#if canDownload}
          <li><button type="button" class="item" onclick={() => (mode = 'download')}><Icon name="download" />Download…</button></li>
        {/if}
        {#if reportable}
          <li><button type="button" class="item" onclick={() => (mode = 'report')}><Icon name="flag" />Report…</button></li>
        {/if}
      </ul>
    {:else if mode === 'download' && picture}
      <button type="button" class="back" onclick={() => (mode = 'menu')}><Icon name="chevron-left" size={16} />Download</button>
      <EmoteDownload emote={{ ...picture, name }} bare ondone={() => (moreOpen = false)} />
    {:else}
      <ReportForm {viewer} {name} oncancel={() => (moreOpen = false)} />
    {/if}
  </Popover>
</div>

<style>
  p { margin: 0; }
  .actions-row { display: flex; flex-wrap: wrap; align-items: center; justify-content: center; gap: 8px; }
  .btn :global(.icon) { flex: none; }
  .btn { gap: 8px; }
  .more { gap: 6px; padding-right: 10px; }
  .heart { width: 40px; padding: 0; justify-content: center; }
  .heart :global(.icon) { color: var(--muted); transition: color var(--dur-color), transform var(--dur-press) ease-out; }
  .heart.on :global(.icon) { color: var(--accent-text); fill: currentColor; }
  .heart:active :global(.icon) { transform: scale(.88); }
  .heart[aria-busy='true'] { opacity: .7; }
  .lbl { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .more :global(.icon) { color: var(--muted); }

  .bar { flex-wrap: nowrap; justify-content: stretch; }
  .bar > :global(.pop-root:first-child), .bar > .add { flex: 1; min-width: 0; }
  .bar .add { width: 100%; height: 44px; }
  .bar .edit, .bar .more, .bar .heart { width: 44px; height: 44px; padding: 0; justify-content: center; }
  .bar .word { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
  .bar .more :global(.icon) { color: inherit; }

  .sets { display: grid; gap: 2px; grid-template-columns: minmax(0, 1fr); }
  .note { padding: 6px 8px; }
  .err { color: var(--text); }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; grid-template-columns: minmax(0, 1fr); }
  .set, .item, .back { width: 100%; display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 0 10px;
    border: 0; border-radius: var(--radius-btn); background: none; color: var(--text); font: inherit; font-size: var(--t-ui);
    text-align: left; cursor: pointer; transition: background-color var(--dur-color), transform var(--dur-press) ease-out;
    touch-action: manipulation; user-select: none; -webkit-user-select: none; }
  .set { min-height: 44px; padding-block: 4px; }
  .item :global(.icon) { color: var(--muted); }
  .item :global(.swap) { color: var(--muted); }
  .back { gap: 6px; margin-bottom: 4px; padding-left: 4px; font-weight: 600; }
  .back :global(.icon) { color: var(--muted); }
  .set:active:not([aria-disabled='true']), .item:active, .back:active { transform: scale(var(--press)); }
  .set[aria-disabled='true'] { cursor: default; }
  .set-text { flex: 1; min-width: 0; display: grid; }
  .set-name { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .live { display: inline-flex; align-items: center; gap: 4px; color: var(--muted); }
  .live :global(.icon) { color: var(--accent-text); }
  .set[aria-disabled='true'] .set-name { color: var(--muted); }
  .state { display: inline-flex; align-items: center; gap: 4px; flex: none; }
  .set:focus-visible, .item:focus-visible, .back:focus-visible { outline-offset: -2px; }
  @media (hover: hover) and (pointer: fine) {
    .set:hover:not([aria-disabled='true']), .item:hover, .back:hover { background: var(--surface-2); }
  }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .set, :global(html:not([data-motion='full'])) .item, :global(html:not([data-motion='full'])) .back { transition: background-color var(--dur-color); } }
</style>
