// Builds the /developers index and each group's JSON from openapi.json and schema.graphql, at
// build time (the page and the group endpoints are prerendered). Server only: it uses the GraphQL parser.
import { eventTypes, subscriptionFrames } from './events';
import { graphqlReference, type GqlOp } from './graphql';
import { restGroupId } from './groups';
import { gqlChip, gqlChipClass, typeChip, type GroupPayload, type NavGroup } from './lazy';
import { restGroups } from './rest';

let built: { rest: ReturnType<typeof restGroups>; gql: ReturnType<typeof graphqlReference> } | undefined;
const sources = () => (built ??= { rest: restGroups(), gql: graphqlReference() });

const gqlGroups = (g: ReturnType<typeof graphqlReference>) => [
  { id: 'gql-queries', title: 'GraphQL queries', ops: g.queries },
  { id: 'gql-mutations', title: 'GraphQL mutations', ops: g.mutations },
  { id: 'gql-subscriptions', title: 'GraphQL subscriptions', ops: g.subscriptions },
];
const gqlEntry = (op: GqlOp) => ({ id: op.id, chip: gqlChip[op.kind], cls: gqlChipClass[op.kind], label: op.name, search: `graphql ${op.kind}` });

/** The index, in page order: REST groups, the event socket, GraphQL operations, GraphQL types. */
export function navGroups(): NavGroup[] {
  const { rest, gql } = sources();
  return [
    ...rest.map((g) => ({
      id: restGroupId(g.tag),
      title: g.title,
      entries: g.ops.map((o) => ({ id: o.id, chip: o.method, cls: `m-${o.method.toLowerCase()}`, label: o.title, search: `${o.path} rest` })),
    })),
    { id: 'live', title: 'Live events', entries: [{ id: 'ws-events', chip: 'WS', cls: 'm-ws', label: 'Event socket', search: '/v1/events websocket socket' }] },
    ...gqlGroups(gql).map((g) => ({ id: g.id, title: g.title, entries: g.ops.map(gqlEntry) })),
    {
      id: 'gql-types',
      title: 'GraphQL types',
      entries: gql.types.map((t) => ({ id: t.id, chip: typeChip(t), cls: 'm-get', label: t.name, search: `graphql type ${t.kind}` })),
    },
  ];
}

/** What the page itself needs besides the index. */
export function pageFacts() {
  const { rest, gql } = sources();
  return {
    restCount: rest.reduce((n, g) => n + g.ops.length, 0),
    gqlCount: gql.queries.length + gql.mutations.length + gql.subscriptions.length,
    known: gql.types.map((t) => t.name),
    events: eventTypes(gql),
  };
}

/** The ids of every group with a JSON body, for prerendering. */
export const lazyGroupIds = (): string[] => navGroups().map((g) => g.id).filter((id) => id !== 'live');

export function groupPayload(id: string): GroupPayload | undefined {
  const { rest, gql } = sources();
  const r = rest.find((g) => restGroupId(g.tag) === id);
  if (r) return { kind: 'rest', ops: r.ops };
  if (id === 'gql-types') return { kind: 'types', types: gql.types };
  const g = gqlGroups(gql).find((x) => x.id === id);
  if (!g) return undefined;
  return { kind: 'ops', ops: g.ops.map((op) => (op.kind === 'subscription' ? { ...op, frames: subscriptionFrames(op, gql.socket) } : op)) };
}
