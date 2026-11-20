// Whether this device has seen the newest changelog entry. Remembered per device (localStorage),
// like the theme: no account setting. Nothing stored counts as not seen, so a returning reader
// gets the dot once; opening /changelog (or its menu row) marks it seen.
import { LATEST } from './entries';

export const SEEN_KEY = 'cyex.changelog.seen';

function read(): string | null {
  try {
    return localStorage.getItem(SEEN_KEY);
  } catch {
    return null;
  }
}

class WhatsNew {
  /** False until the browser has read the device's answer (never on the server). */
  unseen = $state(false);

  /** Reads the stored answer; call from an effect (the browser only). */
  load(latest = LATEST.id) {
    this.unseen = read() !== latest;
  }

  markSeen(latest = LATEST.id) {
    this.unseen = false;
    try {
      localStorage.setItem(SEEN_KEY, latest);
    } catch {
      /* not remembered: the dot comes back next visit */
    }
  }
}

export const whatsNew = new WhatsNew();
