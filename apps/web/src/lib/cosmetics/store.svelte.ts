// Who shows which badge and paint, loaded once and shared by every name on the page.
//   - The catalog (`cosmetics`, public) is asked for once per visit.
//   - Chatters by platform id (`userCosmetics`) are batched: every lookup made in the same tick
//     goes out as one request of up to 100 ids, and each id is asked for once (no N+1).
//   - Anyone's account by id (`accountCosmetics`, public): `wantLooks(ids)` batches like chatters
//     and fills the same `looks`, so a name anywhere (profile, editors, search, activity) is drawn
//     in its paint from one cache, each account asked for once.
//   - An account's own selection (`myCosmetics`: yours, or one you edit with PROFILE) lives in
//     `looks`, a reactive map by account id, so the account menu and the profile change together
//     when a badge or paint is picked.
import { gql } from '$lib/api';
import type { BadgeFieldsFragment, MyCosmeticsQuery, PaintFieldsFragment, Platform } from '$lib/gql/graphql';
import { ACCOUNT_COSMETICS, COSMETICS, MY_COSMETICS, USER_COSMETICS } from '$lib/queries';
import { batches, mergeLooks, toAsk } from './merge';

export type Badge = BadgeFieldsFragment;
export type Paint = PaintFieldsFragment;
export type Look = { badge: Badge | null; paint: Paint | null };
export type Owned = MyCosmeticsQuery['myCosmetics'];
export type Catalog = { badges: Map<string, Badge>; paints: Map<string, Paint> };

export const NO_LOOK: Look = { badge: null, paint: null };

let catalogAsk: Promise<Catalog> | null = null;

/** Every badge and paint, by id. A failed request isn't kept, so the next call tries again. */
export function loadCatalog(): Promise<Catalog> {
  if (catalogAsk) return catalogAsk;
  const mine = gql(COSMETICS, {}).then(({ cosmetics }) => ({
    badges: new Map(cosmetics.badges.map((b) => [b.id, b])),
    paints: new Map(cosmetics.paints.map((p) => [p.id, p])),
  }));
  catalogAsk = mine;
  mine.catch(() => {
    if (catalogAsk === mine) catalogAsk = null;
  });
  return mine;
}

/** Forget the catalog (after staff change a badge or paint). */
export function forgetCatalog() {
  catalogAsk = null;
}

// ---- Chatters by platform id, batched -------------------------------------------------------

const MAX_IDS = 100;
const byChatter = new Map<string, Promise<Look>>();
const queued = new Map<Platform, Map<string, (l: Look) => void>>();
let flushing = false;

function flush() {
  flushing = false;
  const batches = [...queued.entries()];
  queued.clear();
  for (const [platform, waiting] of batches) {
    const ids = [...waiting.keys()];
    for (let i = 0; i < ids.length; i += MAX_IDS) {
      const chunk = ids.slice(i, i + MAX_IDS);
      Promise.all([gql(USER_COSMETICS, { platform, userIds: chunk }), loadCatalog()])
        .then(([{ userCosmetics }, cat]) => {
          const found = new Map(userCosmetics.map((u) => [u.platformUserId, u]));
          for (const id of chunk) {
            const u = found.get(id);
            waiting.get(id)!({
              badge: (u?.badgeId && cat.badges.get(u.badgeId)) || null,
              paint: (u?.paintId && cat.paints.get(u.paintId)) || null,
            });
          }
        })
        .catch(() => {
          // Names still show, just plain; a later page can ask again.
          for (const id of chunk) {
            byChatter.delete(`${platform}:${id}`);
            waiting.get(id)!(NO_LOOK);
          }
        });
    }
  }
}

/** What a Twitch or Kick user shows. Lookups made in the same tick share one request. */
export function lookFor(platform: Platform, platformUserId: string): Promise<Look> {
  const key = `${platform}:${platformUserId}`;
  const known = byChatter.get(key);
  if (known) return known;
  const p = new Promise<Look>((resolve) => {
    let list = queued.get(platform);
    if (!list) queued.set(platform, (list = new Map()));
    list.set(platformUserId, resolve);
  });
  byChatter.set(key, p);
  if (!flushing) {
    flushing = true;
    queueMicrotask(flush);
  }
  return p;
}

// ---- Accounts you may see the cosmetics of (yours, or edited with PROFILE) -----------------

/** The look of accounts whose cosmetics this browser loaded, by account id. Reactive. */
export const looks = $state<Record<string, Look>>({});

const ownedAsk = new Map<string, Promise<Owned>>();
/** Accounts whose look is known or being asked for (so each is asked once). */
const askedAccounts = new Set<string>();
let accountQueue = new Set<string>();
let accountsFlushing = false;

function flushAccounts() {
  accountsFlushing = false;
  const ids = [...accountQueue];
  accountQueue = new Set();
  for (const chunk of batches(ids)) {
    gql(ACCOUNT_COSMETICS, { accountIds: chunk })
      .then(({ accountCosmetics }) => {
        // An account's own selection (loadOwned) is newer than this answer.
        const known = mergeLooks(chunk, accountCosmetics, new Set(chunk.filter((id) => ownedAsk.has(id))));
        for (const [id, l] of Object.entries(known)) looks[id] = l;
      })
      .catch(() => {
        // Names still show, just plain; a later page can ask again.
        for (const id of chunk) askedAccounts.delete(id);
      });
  }
}

/** Ask for what these accounts show; their `looks` fill in. Lookups in the same tick share one request. */
export function wantLooks(ids: readonly (string | null | undefined)[]) {
  const fresh = toAsk(ids, askedAccounts);
  if (!fresh.length) return;
  for (const id of fresh) {
    askedAccounts.add(id);
    accountQueue.add(id);
  }
  if (!accountsFlushing) {
    accountsFlushing = true;
    queueMicrotask(flushAccounts);
  }
}

/** A look the page already has (loaded with it): shared like any other, and not asked for again. */
export function seedLook(accountId: string, look: Look) {
  if (askedAccounts.has(accountId) && looks[accountId]) return;
  askedAccounts.add(accountId);
  looks[accountId] = look;
}

function lookOf(o: Pick<Owned, 'badges' | 'paints' | 'badgeId' | 'paintId'>): Look {
  return {
    badge: o.badges.find((b) => b.id === o.badgeId) ?? null,
    paint: o.paints.find((p) => p.id === o.paintId) ?? null,
  };
}

/**
 * What an account owns and shows. `mine` asks for yours (no accountId); otherwise for an account
 * you edit with PROFILE. Shared per account; `fresh` asks again.
 */
export function loadOwned(accountId: string, mine: boolean, fresh = false): Promise<Owned> {
  const known = ownedAsk.get(accountId);
  if (known && !fresh) return known;
  const p = gql(MY_COSMETICS, { accountId: mine ? null : accountId }).then(({ myCosmetics }) => {
    askedAccounts.add(accountId);
    looks[accountId] = lookOf(myCosmetics);
    return myCosmetics;
  });
  ownedAsk.set(accountId, p);
  p.catch(() => {
    if (ownedAsk.get(accountId) === p) ownedAsk.delete(accountId);
  });
  return p;
}

/** After a pick: the new look everywhere, and the cached answer updated in place. */
export function picked(accountId: string, owned: Owned, badgeId: string | null, paintId: string | null): Owned {
  const next = { ...owned, badgeId, paintId };
  looks[accountId] = lookOf(next);
  ownedAsk.set(accountId, Promise.resolve(next));
  return next;
}

/** Forget everything account-bound (signing out). */
export function forgetLooks() {
  ownedAsk.clear();
  askedAccounts.clear();
  for (const k of Object.keys(looks)) delete looks[k];
}
