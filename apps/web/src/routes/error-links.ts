// Where the error page sends you instead of a dead end. Pure, so it is tested: the first link is
// the closest place to what the address looked like (a shop link to the shop, a help or docs link
// to help, anything else, such as an emote, a person or a set, to search), then the rest, each once. The front page
// is not listed: the problem's own button already goes there.

export type Way = { href: string; label: string; icon: 'search' | 'shop' | 'help' | 'changelog' };

const SEARCH: Way = { href: '/search', label: 'Browse emotes', icon: 'search' };
const SHOP: Way = { href: '/shop', label: 'The shop', icon: 'shop' };
const HELP: Way = { href: '/help', label: 'Help', icon: 'help' };
const NEW: Way = { href: '/changelog', label: 'What changed lately', icon: 'changelog' };

/** The places to offer on an error page for this path, the closest first. */
export function elsewhere(pathname: string): Way[] {
  const p = pathname.toLowerCase();
  const first = /^\/(shop|redeem)\b/.test(p)
    ? SHOP
    : /^\/(help|guidelines|developers|changelog|download)\b/.test(p)
      ? HELP
      : SEARCH;
  return [first, ...[SEARCH, SHOP, HELP, NEW].filter((w) => w !== first)];
}
