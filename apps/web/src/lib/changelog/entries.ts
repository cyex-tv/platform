// What changed on cyex.tv, newest first: /changelog reads this, and the account menu's
// "What's new" dot compares the newest `id` with the one this device last saw (seen.svelte.ts).
// Each entry: the day it shipped (yyyy-mm-dd, shown as 2026/11/20), a short title, and a few
// lines in plain words about what you can now do. Staff-only changes go in `staff`.

export type Entry = {
  /** Stable, unique: the anchor on /changelog and what "seen" remembers. */
  id: string;
  date: string;
  title: string;
  items: string[];
  /** For moderators and admins: shown under the rest, labelled. */
  staff?: string[];
};

export const ENTRIES: Entry[] = [
  {
    id: '2026-11-20-small-touches',
    date: '2026-11-20',
    title: 'Chat preview for sets, recent searches and more',
    items: [
      'A set’s page shows a chat preview of its emotes, in Twitch or Kick: wide emotes, zero-width overlays and the owner’s painted name.',
      'The emote page lists each size’s file size under it.',
      'Set cards show how full the set is with a thin bar and “24 / 1.000”.',
      'The search field remembers your last five searches on this device, with a row to clear them.',
      'Help starts with the most-read answers and says whether the service is running.',
    ],
  },
  {
    id: '2026-11-20-announcements-notifications-collection',
    date: '2026-11-20',
    title: 'Announcements, notification settings and your collection',
    items: [
      'News, maintenance and events show as one line under the header. Close it with × and it stays closed on that device until the next one.',
      'Settings has a Notifications page: choose which kinds notify you at all, and which also pop up while you’re on cyex.tv. Turned off, a kind isn’t kept, so it won’t show up later either.',
      'Your notifications page has filters: your uploads, hand-overs, editors and cosmetics.',
      'Profiles can show a Collection: every badge and paint you own, drawn on your own name, with since when and whether it came from the shop, a code, an event or staff. It’s off until you turn it on in Settings → Account or on the tab itself, and never shows who gave you something or what it cost.',
    ],
    staff: ['Admin → System → Announcements puts up the line, with a kind, an optional link, emote or badge, and a start and an end. It needs the new Site announcements permission, and every change is in the audit log.'],
  },
  {
    id: '2026-11-20-shortcuts-previews',
    date: '2026-11-20',
    title: 'Shortcuts, link previews and this page',
    items: [
      'Press ? on any page for its keyboard shortcuts. Keyboard shortcuts in the footer opens the same list.',
      'A link to an emote, set, profile or channel shows a preview in Discord, X and other apps: the emote itself, the set’s first emotes or the avatar.',
      'With a set chosen on the Emotes tab, emotes already in it carry an In set mark.',
      'Your own unlisted and private emotes say so on their tiles. Nobody else sees the mark.',
      'This changelog. A dot on your account menu means there is something new here.',
    ],
  },
  {
    id: '2026-11-20-fixes',
    date: '2026-11-20',
    title: 'Big sets, safer search, many small fixes',
    items: [
      'Sets with a thousand emotes open in about half the time: the set editor loads 120 at a time, and its filter searches the whole set.',
      'Add emotes in the set editor has Load more and says how many you are seeing.',
      'An emote hand-over that was answered, withdrawn or ran out says so in your notifications, instead of still offering Accept.',
      'Search while typing leaves out unlisted emotes, as it always should have.',
      'Reduced motion follows the setting in Settings → Appearance everywhere, including the review queue and search.',
      'Uploading, the upload dialog, set visibility, profiles and the account pages got a round of fixes from a full test pass.',
    ],
  },
  {
    id: '2026-11-19-select-editors',
    date: '2026-11-19',
    title: 'Select many, editors who upload, one emote page',
    items: [
      'Right-click an emote, or use its ⋯ button, to add it to your set, open it in a new tab, copy its link or start selecting.',
      'Select up to 100 emotes and add them to a set in one go. Drag across the grid to pick several at once.',
      'Trending shows each emote’s place, and every emote plays as you browse.',
      'The emote page is one card, with Channels, Activity and More by the same creator in tabs.',
      'Your profile is your dashboard: your emotes, sets, cosmetics and activity, with filters and dates.',
      'Editors you trust can upload, manage and hand over emotes for you. A hand-over waits until you accept it.',
      'Badge uploads and paints made in the paint studio work end to end.',
      'A new front page, with the cyex mark in 3D.',
    ],
    staff: [
      'The review queue has tabs, search, sorting and keys: A approves, U approves as unlisted, P for the owner only, R rejects.',
      'Look up who holds a badge, paint or role, and export the list.',
    ],
  },
  {
    id: '2026-11-18-shop-sets',
    date: '2026-11-18',
    title: 'Shop, public sets and profiles',
    items: [
      'The shop: badges and paints for your name in chat, and codes to redeem.',
      'Sets have their own page anyone with the link can open. Clone one into your own sets.',
      'Profiles show a creator’s emotes, sets, badges and paints. Find people by name in search.',
      'Every emote has an activity feed: uploaded, approved, added to channels.',
      'Settings → Account lists your sessions and lets you export or delete your data.',
    ],
  },
  {
    id: '2026-11-17-first-site',
    date: '2026-11-17',
    title: 'The first cyex.tv',
    items: [
      'Browse and search emotes, upload your own, and put them in your channel’s set for Twitch and Kick.',
      'Chat badges and name paints, shown by the cyex.tv extension.',
      'Give editors a hand with your sets, with the permissions you choose.',
      'The API is public: reads for chat clients, GraphQL and live events, documented under Developers.',
    ],
  },
];

/** The newest entry: what the "What's new" dot compares with. */
export const LATEST = ENTRIES[0];

/** `2026-11-20` → `2026/11/20` (cyex dates). */
export function shownDate(date: string): string {
  return date.replaceAll('-', '/');
}

/** Entries grouped by day, newest first, for the index. */
export function byDay(entries: Entry[] = ENTRIES): { date: string; entries: Entry[] }[] {
  const out: { date: string; entries: Entry[] }[] = [];
  for (const e of entries) {
    const last = out.at(-1);
    if (last?.date === e.date) last.entries.push(e);
    else out.push({ date: e.date, entries: [e] });
  }
  return out;
}
