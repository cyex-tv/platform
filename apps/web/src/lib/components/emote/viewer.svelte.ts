// Who is looking at an emote page, and their sets. One per page, shared by the header actions,
// the phone action bar and the review block. Only the browser asks (only it has the cookie).
import { ApiError, gql } from '$lib/api';
import type { MySetNamesQuery } from '$lib/gql/graphql';
import { PERSONAL_NOT_APPROVED } from './approvals';
import { ADD_SET_EMOTE, MY_CHANNELS, MY_SET_NAMES, REPORT_EMOTE } from '$lib/queries';

export type ViewerSet = MySetNamesQuery['mySets'][number];

export class Viewer {
  /** `none`: cyex.tv couldn't be reached, so nothing is offered. */
  who = $state<'loading' | 'out' | 'in' | 'none'>('loading');
  sets = $state<ViewerSet[]>([]);
  /** Ids of the sets active on one of your own channels ("on your channel"). */
  live = $state<ReadonlySet<string>>(new Set());
  /** The set the emote was last added to, by id. */
  added = $state<string | null>(null);
  addError = $state<string | null>(null);
  adding = $state<string | null>(null);
  reported = $state(false);
  private started = false;

  constructor(readonly emoteId: string) {}

  /** Sets the emote can go in (not the global one), the ones on your channels first. */
  get own(): ViewerSet[] {
    const own = this.sets.filter((s) => s.kind !== 'GLOBAL');
    return [...own.filter((s) => this.live.has(s.id)), ...own.filter((s) => !this.live.has(s.id))];
  }

  has(set: ViewerSet): boolean {
    return set.emotes.some((m) => m.emote.id === this.emoteId);
  }

  async load() {
    if (this.started) return;
    this.started = true;
    try {
      // mySets itself answers signed_out when there is no session, so no separate ME call.
      const [names, channels] = await Promise.all([gql(MY_SET_NAMES, {}), gql(MY_CHANNELS, {}).catch(() => null)]);
      this.sets = names.mySets;
      // Which set each of your channels shows; a failure only drops the "on your channel" marks.
      this.live = new Set((channels?.myChannels ?? []).map((c) => c.activeSetId).filter((id): id is string => !!id));
      this.who = 'in';
    } catch (e) {
      // Signed out: offer sign-in. cyex.tv unreachable: offer nothing.
      this.who = e instanceof ApiError && e.code === 'signed_out' ? 'out' : 'none';
    }
  }

  async add(setId: string) {
    if (this.adding) return;
    this.adding = setId;
    this.addError = null;
    try {
      await gql(ADD_SET_EMOTE, { setId, emoteId: this.emoteId });
      const set = this.sets.find((s) => s.id === setId);
      if (set) set.emotes.push({ emote: { id: this.emoteId } });
      this.added = setId;
    } catch (e) {
      this.addError = e instanceof ApiError && e.code === 'personal_use_not_approved' ? PERSONAL_NOT_APPROVED : e instanceof Error ? e.message : 'That didn’t work.';
    } finally {
      this.adding = null;
    }
  }

  /** Sends a report. Throws with a message to show when it fails. */
  async report(reason: string) {
    await gql(REPORT_EMOTE, { id: this.emoteId, reason });
    this.reported = true;
  }
}
