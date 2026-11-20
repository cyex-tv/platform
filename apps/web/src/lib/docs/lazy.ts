// The /developers reference is split in two: the page carries the guides and an index of every
// group (headings and entries, no bodies), and a closed group's body is fetched and drawn when it
// is opened (lazy-groups.ts). This file is the part both sides share: the shapes, the labels and
// the address of a group's JSON. It imports types only, so the GraphQL parser stays on the server.
import type { GqlOp, GqlType } from './graphql';
import type { RestOp } from './rest';

/** One line of the index. Everything the filter reads (`search`) comes from here, not from the body. */
export interface NavEntry {
  id: string;
  /** The chip text (GET, QUERY, TYPE) and its colour class (m-get, m-post, m-ws). */
  chip: string;
  cls: string;
  label: string;
  search: string;
}
export interface NavGroup {
  id: string;
  title: string;
  entries: NavEntry[];
}

export type GqlOpPayload = GqlOp & { frames?: string };
/** What a group's JSON holds, by the kind of group. */
export type GroupPayload =
  | { kind: 'rest'; ops: RestOp[] }
  | { kind: 'ops'; ops: GqlOpPayload[] }
  | { kind: 'types'; types: GqlType[] };

export const typeId = (name: string) => `gql-type-${name}`;
export const typeChip = (t: Pick<GqlType, 'kind'>) => t.kind.toUpperCase().replace('OBJECT', 'TYPE');
export const gqlChip = { query: 'QUERY', mutation: 'MUT', subscription: 'SUB' } as const;
export const gqlChipLong = { query: 'QUERY', mutation: 'MUTATION', subscription: 'SUBSCRIPTION' } as const;
export const gqlChipClass = { query: 'm-get', mutation: 'm-post', subscription: 'm-ws' } as const;

/** Groups whose body is drawn on demand. The notes and the one-entry live group come with the page. */
export const isLazy = (id: string): boolean => id !== 'notes' && id !== 'live';

export const groupUrl = (id: string, base = ''): string => `${base}/developers/groups/${id}.json`;

export async function fetchGroup(id: string, base = '', f: typeof fetch = fetch): Promise<GroupPayload> {
  const res = await f(groupUrl(id, base));
  if (!res.ok) throw new Error(`${res.status} ${groupUrl(id, base)}`);
  return res.json();
}
