// The docs' collapsible groups: which are open by default, what is remembered for the session, and
// which group a section belongs to. Pure, so it can be tested; behaviour.ts does the DOM.

/** Open or closed per group id. Only groups the reader has toggled are present. */
export type GroupState = Record<string, boolean>;

export const GROUP_KEY = 'cyexDocsGroups';

/** Closed by default, except the guide notes: they are what a first-time reader came for. */
export const defaultOpen = (id: string): boolean => id === 'notes';

export const isOpen = (state: GroupState, id: string): boolean => state[id] ?? defaultOpen(id);

/** Reads what sessionStorage held; anything that is not a map of booleans is ignored. */
export function parseGroupState(raw: string | null): GroupState {
  if (!raw) return {};
  try {
    const v: unknown = JSON.parse(raw);
    if (!v || typeof v !== 'object' || Array.isArray(v)) return {};
    return Object.fromEntries(Object.entries(v).filter((e): e is [string, boolean] => typeof e[1] === 'boolean'));
  } catch {
    return {};
  }
}

export const withGroup = (state: GroupState, id: string, open: boolean): GroupState => ({ ...state, [id]: open });

export const withAll = (ids: string[], open: boolean): GroupState => Object.fromEntries(ids.map((id) => [id, open]));

/** The control reads "Collapse all" while every group is open, "Expand all" otherwise. */
export const allOpen = (state: GroupState, ids: string[]): boolean => ids.every((id) => isOpen(state, id));

/** The id of the REST group for an OpenAPI tag. */
export const restGroupId = (tag: string): string => `rest-${tag.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')}`;

/** Singular or plural entry count for a group header's label. */
export const countLabel = (n: number): string => `${n} ${n === 1 ? 'entry' : 'entries'}`;
