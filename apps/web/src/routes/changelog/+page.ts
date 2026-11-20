import type { PageLoad } from './$types';

// Static: the entries are data in $lib/changelog; the page asks the API for nothing.
export const prerender = true;

export const load: PageLoad = () => ({
  footer: 'slim' as const, // the docs layout ends where its last entry does
  skipTo: 'changes', // the second skip link: past the index, to the entries
});
