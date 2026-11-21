// The sets the browse page adds to: yours and those you edit with EMOTES (never a global one).
// One of them is the target ("Adding to"): it starts as your channel's active set, and a pick is
// remembered on this device. Every set knows which emotes it holds (`held`, so a tile can say
// "In Main" and a menu Add or Remove) and their pictures (`items`, for the tray), and adds and
// removes one emote with the same mutations the set editor uses.
//
// `put` and `take` are what the page's controls call (the tile's +, its menu, a drop on the rail,
// the tray): they refuse a full set before asking the API, say what happened in a toast with Undo,
// and work on one emote or many (a selection), one request after another like the selection bar.
import { SvelteMap, SvelteSet } from 'svelte/reactivity';
import { ApiError, gql } from '$lib/api';
import type { Picture } from '$lib/emote';
import { clean } from '$lib/format';
import { grantOn } from '$lib/tool/editors';
import { notify } from '$lib/tool/notices.svelte';
import { session } from '$lib/tool/session.svelte';
import { ADD_SET_EMOTE, MY_CHANNELS, MY_SETS, REMOVE_SET_EMOTE } from '$lib/queries';

/** An emote as the tray shows it. */
export type SetItem = Picture & { id: string; name: string };

export type TargetSet = {
  id: string;
  name: string;
  capacity: number;
  held: SvelteSet<string>;
  /** The emotes it holds, oldest first (the tray lists them newest first). */
  items: SvelteMap<string, SetItem>;
  /** A channel shows it (LIVE). */
  active: boolean;
  /** Whose set it is: yours (`mine`) or an account you edit for; the rail groups by it. */
  ownerId: string;
  ownerName: string;
  mine: boolean;
  /** The channels of its owner, so the filter finds a set by channel. */
  channels: string[];
};

const KEY = 'cyex.browse.target';

function remembered(): string {
  try {
    return localStorage.getItem(KEY) ?? '';
  } catch {
    return '';
  }
}

/** `Main` for your set, `nora_byte · Main` for one you edit for: what toasts and menus call it. */
export function setLabel(s: Pick<TargetSet, 'name' | 'mine' | 'ownerName'>): string {
  return s.mine || !s.ownerName ? clean(s.name) : `${clean(s.ownerName)} · ${clean(s.name)}`;
}

/** Room left in a set. */
export const free = (s: Pick<TargetSet, 'capacity' | 'held'>) => Math.max(0, s.capacity - s.held.size);

class Target {
  sets = $state<TargetSet[] | null>(null);
  error = $state('');
  chosenId = $state('');
  #loading: Promise<void> | null = null;
  #for = '';

  get current(): TargetSet | undefined {
    return this.sets?.find((s) => s.id === this.chosenId);
  }

  byId(id: string): TargetSet | undefined {
    return this.sets?.find((s) => s.id === id);
  }

  /** Loads once per page view (again after `refresh`); signed out, there is nothing to load. */
  ensure() {
    const me = session.me?.id ?? '';
    if (me !== this.#for) {
      // Signed out or someone else: forget the last account's sets.
      this.#for = me;
      this.sets = null;
      this.chosenId = '';
    }
    if (!me || this.sets || this.#loading) return;
    void this.refresh();
  }

  refresh(): Promise<void> {
    this.error = '';
    this.#loading = (async () => {
      try {
        const [r, ch] = await Promise.all([gql(MY_SETS, {}), gql(MY_CHANNELS, {})]);
        const me = session.me?.id;
        const active = new Set(ch.myChannels.map((c) => c.activeSetId).filter(Boolean) as string[]);
        const writable = [...r.mySets, ...r.editorSets.filter((s) => grantOn(r.editorOf, s.owner?.id, me).has('EMOTES'))].filter(
          (s) => s.kind !== 'GLOBAL',
        );
        const byId = new Map(writable.map((s) => [s.id, s]));
        const own = new Set(r.mySets.map((s) => s.id));
        const myLogins = [...new Set(ch.myChannels.filter((c) => !c.account || c.account.id === me).map((c) => c.login))];
        const loginsOf = (owner: string) => [...new Set(ch.myChannels.filter((c) => c.account?.id === owner).map((c) => c.login))];
        const old = new Map((this.sets ?? []).map((s) => [s.id, s]));
        this.sets = [...byId.values()].map((s) => {
          const held = old.get(s.id)?.held ?? new SvelteSet<string>();
          const items = old.get(s.id)?.items ?? new SvelteMap<string, SetItem>();
          held.clear();
          items.clear();
          const members = [...s.emotes].sort((a, b) => (a.addedAt ?? '').localeCompare(b.addedAt ?? ''));
          for (const m of members) {
            held.add(m.emote.id);
            items.set(m.emote.id, { id: m.emote.id, name: m.name || m.emote.name, host: m.emote.host, animated: m.emote.animated, files: m.emote.files });
          }
          const mine = own.has(s.id);
          return {
            id: s.id,
            name: s.name,
            capacity: s.capacity,
            held,
            items,
            active: active.has(s.id),
            ownerId: s.owner?.id ?? '',
            ownerName: s.owner?.displayName ?? '',
            mine,
            channels: mine ? myLogins : loginsOf(s.owner?.id ?? ''),
          };
        });
        const ids = this.sets.map((s) => s.id);
        if (!ids.includes(this.chosenId)) {
          const saved = remembered();
          this.chosenId = ids.includes(saved) ? saved : (this.sets.find((s) => s.active && s.mine)?.id ?? this.sets.find((s) => s.active)?.id ?? ids[0] ?? '');
        }
      } catch (e) {
        this.error = e instanceof Error ? e.message : 'Couldn’t load your sets.';
      } finally {
        this.#loading = null;
      }
    })();
    return this.#loading;
  }

  choose(id: string) {
    this.chosenId = id;
    try {
      localStorage.setItem(KEY, id);
    } catch {
      /* not remembered */
    }
  }

  /** Adds one emote to a set (the chosen one by default); throws the API's error. */
  async add(emoteId: string, set = this.current, item?: SetItem) {
    if (!set) return;
    await gql(ADD_SET_EMOTE, { setId: set.id, emoteId });
    set.held.add(emoteId);
    if (item) set.items.set(emoteId, item);
  }

  async remove(emoteId: string, set = this.current) {
    if (!set) return;
    await gql(REMOVE_SET_EMOTE, { setId: set.id, emoteId });
    set.held.delete(emoteId);
    set.items.delete(emoteId);
  }

  /**
   * Adds emotes to `set` and says so in a toast with Undo. A full set is refused before anything
   * is sent; what doesn't fit is left out and counted. Returns the ids that went in.
   */
  async put(set: TargetSet, emotes: readonly SetItem[]): Promise<string[]> {
    const label = setLabel(set);
    const fresh = emotes.filter((e) => !set.held.has(e.id));
    if (!fresh.length) {
      notify(emotes.length === 1 ? `${clean(emotes[0].name)} is already in ${label}.` : `They’re all in ${label} already.`);
      return [];
    }
    const room = free(set);
    if (room === 0) {
      notify(`${label} is full (${set.held.size}/${set.capacity}). Remove an emote from it first.`, true);
      return [];
    }
    const added: string[] = [];
    let taken = 0;
    let failed = 0;
    let overflow = Math.max(0, fresh.length - room);
    let message = '';
    for (const e of fresh.slice(0, room)) {
      try {
        await this.add(e.id, set, e);
        added.push(e.id);
      } catch (err) {
        const code = err instanceof ApiError ? err.code : '';
        if (code === 'set_full') {
          overflow += 1;
          break;
        }
        if (code === 'name_taken') taken += 1;
        else {
          failed += 1;
          message = err instanceof Error ? err.message : '';
        }
      }
    }
    const one = fresh.length === 1 ? clean(fresh[0].name) : '';
    let text: string;
    if (added.length) text = one ? `Added ${one} to ${label}.` : `Added ${added.length} to ${label}.`;
    else if (taken && one) text = `${label} already has an emote called ${one}.`;
    else if (overflow && !added.length) text = `${label} is full.`;
    else text = one ? message || `Couldn’t add ${one}.` : `Nothing was added to ${label}.`;
    const extra = [
      !one && taken ? `${taken} ${taken === 1 ? 'has a name' : 'have names'} the set already uses.` : '',
      overflow && added.length ? `${overflow} didn’t fit: the set is full.` : '',
      !one && failed ? `${failed} couldn’t be added.` : '',
    ].filter(Boolean);
    const all = [text, ...extra].join(' ');
    if (added.length) {
      notify(all, false, { ms: 6000, action: { label: 'Undo', run: () => void this.#undoAdd(set, added) } });
    } else notify(all, true);
    return added;
  }

  /** Takes emotes out of `set`, with Undo in the toast. */
  async take(set: TargetSet, ids: readonly string[]): Promise<string[]> {
    const label = setLabel(set);
    const kept = ids.filter((id) => set.held.has(id));
    const back = kept.map((id) => set.items.get(id)).filter(Boolean) as SetItem[];
    const name = kept.length === 1 ? clean(set.items.get(kept[0])?.name ?? 'it') : '';
    const out: string[] = [];
    let failed = '';
    for (const id of kept) {
      try {
        await this.remove(id, set);
        out.push(id);
      } catch (err) {
        failed = err instanceof Error ? err.message : 'That didn’t work.';
      }
    }
    if (out.length) {
      const text = name ? `Removed ${name} from ${label}.` : `Removed ${out.length} from ${label}.`;
      notify(text, false, { ms: 6000, action: { label: 'Undo', run: () => void this.#undoTake(set, back.filter((b) => out.includes(b.id))) } });
    } else if (failed) notify(failed, true);
    return out;
  }

  async #undoAdd(set: TargetSet, ids: string[]) {
    try {
      for (const id of ids) await this.remove(id, set);
    } catch (e) {
      notify(e instanceof Error ? e.message : 'Couldn’t undo that.', true);
    }
  }

  async #undoTake(set: TargetSet, items: SetItem[]) {
    try {
      for (const e of items) await this.add(e.id, set, e);
    } catch (e) {
      notify(e instanceof Error ? e.message : 'Couldn’t undo that.', true);
    }
  }
}

export const target = new Target();
