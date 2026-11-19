// The set "Add" goes to on the browse page: one of your sets or one you edit with EMOTES (never
// a global one). It starts as your channel's active set, and a pick is remembered on this device.
// It knows which emotes the set holds, so a tile's menu can say Add or Remove, and it adds and
// removes one emote with the same mutations the set editor uses.
import { SvelteSet } from 'svelte/reactivity';
import { gql } from '$lib/api';
import { grantOn } from '$lib/tool/editors';
import { session } from '$lib/tool/session.svelte';
import { ADD_SET_EMOTE, MY_CHANNELS, MY_SETS, REMOVE_SET_EMOTE } from '$lib/queries';

export type TargetSet = { id: string; name: string; capacity: number; held: SvelteSet<string>; active: boolean };

const KEY = 'cyex.browse.target';

function remembered(): string {
  try {
    return localStorage.getItem(KEY) ?? '';
  } catch {
    return '';
  }
}

class Target {
  sets = $state<TargetSet[] | null>(null);
  error = $state('');
  chosenId = $state('');
  #loading: Promise<void> | null = null;
  #for = '';

  get current(): TargetSet | undefined {
    return this.sets?.find((s) => s.id === this.chosenId);
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
        const old = new Map((this.sets ?? []).map((s) => [s.id, s]));
        this.sets = [...byId.values()].map((s) => {
          const held = old.get(s.id)?.held ?? new SvelteSet<string>();
          held.clear();
          for (const m of s.emotes) held.add(m.emote.id);
          return { id: s.id, name: s.name, capacity: s.capacity, held, active: active.has(s.id) };
        });
        const ids = this.sets.map((s) => s.id);
        if (!ids.includes(this.chosenId)) {
          const saved = remembered();
          this.chosenId = ids.includes(saved) ? saved : (this.sets.find((s) => s.active)?.id ?? ids[0] ?? '');
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

  /** Adds one emote to the chosen set; throws the API's error. */
  async add(emoteId: string, set = this.current) {
    if (!set) return;
    await gql(ADD_SET_EMOTE, { setId: set.id, emoteId });
    set.held.add(emoteId);
  }

  async remove(emoteId: string, set = this.current) {
    if (!set) return;
    await gql(REMOVE_SET_EMOTE, { setId: set.id, emoteId });
    set.held.delete(emoteId);
  }
}

export const target = new Target();
