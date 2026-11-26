// The header search, reachable from anywhere: focusSearch() puts the cursor in it (on phones it
// opens the field in the header first). "/" outside a field does the same, unless the page has
// its own search field (marked data-page-search), which then takes it. No shortcut hint on screen.

import { now, PHONE } from '$lib/media.svelte';

export const headerSearch = $state({ phoneOpen: false, focusTick: 0, query: '' });

export function focusSearch(query?: string) {
  if (query !== undefined) headerSearch.query = query;
  if (now(PHONE)) headerSearch.phoneOpen = true;
  headerSearch.focusTick += 1;
}
