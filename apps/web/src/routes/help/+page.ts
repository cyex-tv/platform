import type { PageLoad } from './$types';

// Static: the questions are data in $lib/help, the page asks the API for nothing.
export const prerender = true;

export const load: PageLoad = () => ({
  footer: 'slim' as const, // the docs layout ends where its last section does
  skipTo: 'help-answers', // the second skip link: past the index, to the answers
});
