<script lang="ts" module>
  import type { SetMenuQuery } from '$lib/gql/graphql';
  // What the menu learned per set, for 30 seconds: opening it again is instant.
  const known = new Map<string, { at: number; data: SetMenuQuery }>();
  export type SetChange = { kind: 'capacity' | 'active' | 'deleted'; id: string; capacity?: number };
  /** Forget what the menus learned (a set or a grant changed). */
  export function forgetSetMenus() {
    known.clear();
  }
</script>

<script lang="ts">
  // The set menu's host: one per page that shows sets (profile set cards, the browse rail, the set
  // page). It opens the menu (SetMenu) for a set at a point or under an element, asks the API what
  // you may do there, and runs what is picked: Rename hands back to the page (it renames in place),
  // Copy link, Set active on a channel, Change capacity (a dialog with the allowed range), Open the
  // set page or its editor, Delete (a dialog that says it can't be undone).
  //   <SetActions bind:this={acts} onrename={(id) => (renaming = id)} onchanged={() => reload()} />
  //   acts.open(set, event)  or  acts.open(set, buttonElement)
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { ApiError, gql } from '$lib/api';
  import { copy } from '$lib/clipboard.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { clean, count, iso } from '$lib/format';
  import { SET_CAPACITY_RANGE, SET_MENU, DELETE_SET, UPDATE_CHANNEL } from '$lib/queries';
  import { can, session } from '$lib/tool/session.svelte';
  import { notify } from '$lib/tool/notices.svelte';
  import CapacityForm from './CapacityForm.svelte';
  import SetMenu from './SetMenu.svelte';
  import type { CapacityRange } from './capacity';
  import { menuRights, platformName, setLink, type MenuRights, type MenuSet, type SetAction } from './set-menu';

  type Props = {
    here?: 'page' | 'editor' | null;
    /** Start renaming the set in place. Left out: no Rename item. */
    onrename?: (id: string) => void;
    onchanged?: (change: SetChange) => void;
  };
  let { here = null, onrename, onchanged }: Props = $props();

  let menu = $state<{ set: MenuSet; x: number; y: number; origin: HTMLElement | null } | null>(null);
  let rights = $state<MenuRights | null>(null);
  let asked = 0;

  /** Opens the menu for `set`: at the pointer for a mouse event, under `at` for an element. */
  export function open(set: MenuSet, at: MouseEvent | HTMLElement) {
    let x: number, y: number, origin: HTMLElement | null;
    if (at instanceof HTMLElement) {
      const r = at.getBoundingClientRect();
      x = r.left;
      y = r.bottom + 4;
      origin = at;
    } else {
      // A keyboard context-menu event has no pointer: under its target then.
      const t = at.currentTarget instanceof HTMLElement ? at.currentTarget : null;
      if (at.clientX === 0 && at.clientY === 0 && t) {
        const r = t.getBoundingClientRect();
        x = r.left + 8;
        y = r.bottom;
      } else {
        x = at.clientX;
        y = at.clientY;
      }
      origin = t ?? (document.activeElement as HTMLElement | null);
    }
    menu = { set, x, y, origin };
    rights = null;
    void learn(set);
  }

  async function learn(set: MenuSet) {
    const me = session.me?.id;
    if (!me) return;
    const mine = ++asked;
    const staff = can('sets.edit_any');
    const hit = known.get(set.id);
    if (hit && Date.now() - hit.at < 30_000) {
      rights = menuRights(set, hit.data, me, staff);
      return;
    }
    try {
      const data = await gql(SET_MENU, { id: set.id });
      known.set(set.id, { at: Date.now(), data });
      if (mine === asked && menu?.set.id === set.id) rights = menuRights(set, data, me, staff);
    } catch {
      if (mine === asked && menu?.set.id === set.id) rights = menuRights(set, null, me, staff);
    }
  }

  function close(refocus: boolean) {
    const origin = menu?.origin;
    menu = null;
    if (refocus) origin?.focus({ preventScroll: true });
  }

  async function act(a: SetAction) {
    if (!menu) return;
    const { set, origin } = menu;
    close(a.kind === 'copy' || a.kind === 'active');
    switch (a.kind) {
      case 'rename':
        onrename?.(set.id);
        return;
      case 'copy': {
        const ok = await copy(setLink(page.url.origin, set.id));
        notify(ok ? 'Set link copied.' : 'Couldn’t copy the link.', !ok);
        return;
      }
      case 'active':
        try {
          await gql(UPDATE_CHANNEL, { platform: a.platform as 'TWITCH' | 'KICK', id: a.platformChannelId, input: { activeSetId: set.id } });
          known.clear();
          notify(`${platformName(a.platform)} ${a.login} now shows ${iso(set.name)}.`);
          onchanged?.({ kind: 'active', id: set.id });
        } catch (err) {
          notify(err instanceof ApiError && err.message ? err.message : 'That didn’t work. Try again.', true);
        }
        return;
      case 'capacity':
        void openCapacity(set, origin);
        return;
      case 'page':
        void goto(`/sets/${encodeURIComponent(set.id)}`);
        return;
      case 'editor':
        void goto(`/me/sets/${encodeURIComponent(set.id)}`);
        return;
      case 'delete':
        removing = { set, origin };
        removeError = '';
        removeDialog?.showModal();
        return;
    }
  }

  // ---- Change capacity: a dialog with the field and the allowed range.
  let capDialog = $state<HTMLDialogElement>();
  let cap = $state<{ set: MenuSet; origin: HTMLElement | null; range: CapacityRange | null; capacity: number; error: string } | null>(null);

  async function openCapacity(set: MenuSet, origin: HTMLElement | null) {
    cap = { set, origin, range: null, capacity: set.capacity ?? 0, error: '' };
    capDialog?.showModal();
    try {
      const r = await gql(SET_CAPACITY_RANGE, { id: set.id });
      if (cap?.set.id !== set.id) return;
      if (!r.setCapacityRange) cap.error = 'You can’t change this set’s capacity.';
      else cap.range = r.setCapacityRange;
    } catch (err) {
      if (cap?.set.id === set.id) cap.error = err instanceof ApiError && err.message ? err.message : 'Couldn’t load the allowed range. Try again.';
    }
  }
  function capacitySaved(n: number) {
    const c = cap;
    capDialog?.close();
    if (!c) return;
    known.delete(c.set.id);
    if (n !== c.capacity) {
      notify(`${iso(c.set.name)} now has ${count(n)} slots.`);
      onchanged?.({ kind: 'capacity', id: c.set.id, capacity: n });
    }
  }

  // ---- Delete: a dialog, since it can't be undone.
  let removeDialog = $state<HTMLDialogElement>();
  let removing = $state<{ set: MenuSet; origin: HTMLElement | null } | null>(null);
  let removeBusy = $state(false);
  let removeError = $state('');
  async function confirmRemove() {
    if (!removing || removeBusy) return;
    removeBusy = true;
    removeError = '';
    const { set } = removing;
    try {
      await gql(DELETE_SET, { id: set.id });
      known.delete(set.id);
      removeDialog?.close();
      notify(`${iso(set.name)} deleted.`);
      onchanged?.({ kind: 'deleted', id: set.id });
    } catch (err) {
      removeError =
        err instanceof ApiError && err.code === 'set_in_use'
          ? 'A channel shows this set. Set another one active on it first.'
          : err instanceof ApiError && err.message
            ? err.message
            : 'That didn’t work. Try again.';
    } finally {
      removeBusy = false;
    }
  }
  const refocus = (o: HTMLElement | null | undefined) => o?.isConnected && o.focus({ preventScroll: true });
</script>

{#if menu}
  <SetMenu set={menu.set} x={menu.x} y={menu.y} {rights} signedIn={!!session.me} {here} canRename={!!onrename} onaction={act} onclose={close} />
{/if}

<dialog class="ui-dialog small-dialog" bind:this={capDialog} aria-labelledby="cap-title" onclose={() => { refocus(cap?.origin); cap = null; }}>
  {#if cap}
    <div class="body">
      <div class="title">
        <h2 id="cap-title">Capacity of <bdi>{clean(cap.set.name)}</bdi></h2>
        <button class="ui-icon close" type="button" aria-label="Close" onclick={() => capDialog?.close()}><Icon name="x" size={18} /></button>
      </div>
      <p class="ui-hint lead">How many emotes the set can hold.</p>
      {#if cap.error}
        <p class="ui-error" role="alert">{cap.error}</p>
      {:else if !cap.range}
        <div class="ph ui-skeleton" aria-busy="true" aria-label="Loading the allowed range"></div>
      {:else}
        <CapacityForm compact setId={cap.set.id} capacity={cap.capacity} range={cap.range} onsaved={capacitySaved} oncancel={() => capDialog?.close()} />
      {/if}
    </div>
  {/if}
</dialog>

<dialog class="ui-dialog small-dialog" bind:this={removeDialog} aria-labelledby="del-title" onclose={() => { refocus(removing?.origin); removing = null; }}>
  {#if removing}
    <form method="dialog" onsubmit={(e) => { e.preventDefault(); void confirmRemove(); }}>
      <div class="body">
        <h2 id="del-title">Delete <bdi>{clean(removing.set.name)}</bdi>?</h2>
        <p class="ui-hint">The emotes stay; only this set goes. This can’t be undone.</p>
        {#if removeError}<p class="ui-error" role="alert">{removeError}</p>{/if}
      </div>
      <footer>
        <button class="ui-btn secondary" type="button" disabled={removeBusy} onclick={() => removeDialog?.close()}>Cancel</button>
        <button class="ui-btn danger" type="submit" disabled={removeBusy}>{removeBusy ? 'Deleting…' : 'Delete set'}</button>
      </footer>
    </form>
  {/if}
</dialog>

<style>
  .small-dialog { width: min(440px, calc(100vw - 24px)); }
  .title { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .title h2 { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .close { flex: none; margin: -8px -8px -8px 0; }
  /* The same title and line sizes on every page (the public set page has no tool page around it). */
  .small-dialog h2 { margin: 0; font-size: var(--t-ui, 16px); font-weight: 600; line-height: 1.35; }
  .small-dialog p { margin: 0; font-size: var(--t-small); line-height: 1.45; }
  .lead { margin: -6px 0 0; color: var(--muted); }
  .ph { height: 64px; border-radius: var(--radius-btn); }
  h2 bdi { overflow: hidden; text-overflow: ellipsis; }
</style>
