// The cyex deck installer the front page links, kept current the way cyex.app/deck does it:
//   1. PINNED (deck-release.ts): the installer when this was written, shown first and kept when
//      GitHub can't be reached.
//   2. The answer this reader's browser kept from its last look (at most an hour old).
//   3. GitHub's release list, asked once the page has settled, at most once an hour per reader.
// A release made after the last deploy is picked up without one. Nothing ever moves the link to an
// older release, and any failure leaves it where it was.
import { API, KEY, PINNED, newer, pick, validCache, type DeckRelease } from './deck-release';
import { settled } from './settle';

const HOUR = 60 * 60 * 1000;

export const deck = $state<DeckRelease>({ ...PINNED });

function apply(r: DeckRelease) {
  deck.url = r.url;
  deck.version = r.version;
  deck.prerelease = r.prerelease;
}

function readKept(): unknown {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function keep() {
  try {
    localStorage.setItem(KEY, JSON.stringify({ at: Date.now(), ...deck }));
  } catch {
    // Private windows and blocked storage: the next visit asks again.
  }
}

let started = false;

export async function refreshDeck(): Promise<void> {
  if (typeof window === 'undefined' || started) return;
  started = true;
  const kept = validCache(readKept());
  if (kept && !newer(deck.version, kept.version)) apply(kept);
  if (kept && kept.at <= Date.now() && Date.now() - kept.at < HOUR) return;

  await settled(3000);
  try {
    const res = await fetch(API, { headers: { Accept: 'application/vnd.github+json' }, signal: AbortSignal.timeout(8000) });
    if (res.ok) {
      const got = pick(await res.json());
      if (got && !newer(deck.version, got.version)) apply(got);
    }
  } catch {
    // Offline, blocked or rate-limited: the pinned or kept installer stays.
  }
  // Written even when GitHub said no (a rate limit is a 403), so a reader never asks on every reload.
  keep();
}
