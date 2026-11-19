// The dev mock (dev/mock-api.mjs) must answer the site's GraphQL operations in the real API's shape.
// The schema is the committed schema/schema.graphql (generated from the Rust API; CI checks it is
// current) and the REST answers are checked against openapi/openapi.json. The operations are read
// from the site's own query files, not copied. One assertion at the end lists everything that differs.
import { spawn, type ChildProcess } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  buildSchema,
  execute,
  getNamedType,
  isEnumType,
  isInputObjectType,
  isListType,
  isNonNullType,
  parse,
  validate,
  Kind,
  type DocumentNode,
  type GraphQLInputType,
  type GraphQLSchema,
  type SelectionSetNode,
  type VariableDefinitionNode,
  typeFromAST,
} from 'graphql';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB = resolve(HERE, '../..');
const ROOT = resolve(WEB, '../..');
const schema: GraphQLSchema = buildSchema(readFileSync(resolve(ROOT, 'schema/schema.graphql'), 'utf8'));
const openapi = JSON.parse(readFileSync(resolve(ROOT, 'openapi/openapi.json'), 'utf8'));

// ---- The site's operations, read from its source ------------------------------------------------
const SOURCES = ['src/lib/queries.ts', 'src/lib/paint-studio/queries.ts'];
const fragments = new Map<string, string>();
const operations: { name: string; kind: string; text: string; file: string }[] = [];
for (const file of SOURCES) {
  const src = readFileSync(resolve(WEB, file), 'utf8');
  for (const m of src.matchAll(/graphql\(`([\s\S]*?)`\)/g)) {
    const head = m[1].match(/^\s*(query|mutation|subscription|fragment)\s+(\w+)/);
    if (!head) throw new Error(`${file}: a graphql() call is not a named operation or fragment`);
    if (head[1] === 'fragment') fragments.set(head[2], m[1]);
    else operations.push({ name: head[2], kind: head[1], text: m[1], file });
  }
}
function withFragments(text: string): string {
  const used = new Set<string>();
  const queue = [text];
  while (queue.length) {
    for (const f of queue.pop()!.matchAll(/\.\.\.\s*(\w+)/g)) {
      if (f[1] === 'on' || used.has(f[1])) continue;
      used.add(f[1]);
      const body = fragments.get(f[1]);
      if (body) queue.push(body);
    }
  }
  return [text, ...[...used].map((n) => fragments.get(n) ?? `# missing fragment ${n}`)].join('\n');
}

// ---- Mock process ---------------------------------------------------------------------------------
let child: ChildProcess | null = null;
let base = '';
const COOKIE = 'mock_session=1; mock_admin=1';

function freePort(from: number, to: number): Promise<number> {
  return new Promise((ok, fail) => {
    const tryPort = (p: number) => {
      if (p > to) return fail(new Error('no free port'));
      const s = createServer();
      s.once('error', () => tryPort(p + 1));
      s.listen(p, '127.0.0.1', () => s.close(() => ok(p)));
    };
    tryPort(from);
  });
}
async function startMock(): Promise<{ base: string; proc: ChildProcess }> {
  const port = await freePort(9341, 9349);
  const proc = spawn(process.execPath, ['dev/mock-api.mjs'], {
    cwd: WEB,
    env: { ...process.env, PORT: String(port), MOCK_ADMIN: '1', SITE_ORIGIN: 'http://localhost:5173' },
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  await new Promise<void>((ok, fail) => {
    const t = setTimeout(() => fail(new Error('the mock did not start')), 15000);
    proc.stdout!.on('data', (d) => {
      if (String(d).includes('mock api on')) {
        clearTimeout(t);
        ok();
      }
    });
    proc.once('exit', () => fail(new Error('the mock exited')));
  });
  return { base: `http://localhost:${port}`, proc };
}

async function gql(query: string, variables: Record<string, unknown>, cookie = COOKIE) {
  const res = await fetch(`${base}/v1/gql`, {
    method: 'POST',
    headers: { 'content-type': 'application/json', cookie },
    body: JSON.stringify({ query, variables }),
  });
  return (await res.json()) as { data?: Record<string, unknown> | null; errors?: { message: string; extensions?: { code?: string } }[] };
}

// ---- Variables ------------------------------------------------------------------------------------
type Ctx = Record<string, any>;
const ctx: Ctx = {};

function pick(obj: any, path: string): any {
  return path.split('.').reduce((o, k) => (o == null ? o : o[k]), obj);
}

function stringFor(name: string): string {
  const n = name.toLowerCase();
  if (/(^|_)(since|until|before)$|at$/.test(n)) return '2026-11-10T12:00:00.000Z';
  if (n === 'login') return 'lumen_fox';
  if (n === 'query' || n === 'q') return 'cat';
  if (n === 'code') return 'MOCK-0000-0000';
  if (n === 'confirm') return 'delete my account';
  if (n === 'color') return '#336699';
  if (n === 'name' || n === 'title') return 'MockName';
  if (n === 'tooltip') return 'A mock tooltip';
  if (n === 'host') return 'https://cdn.example.test/x';
  if (n === 'reason' || n === 'note' || n === 'body' || n === 'description') return 'A mock reason';
  if (n === 'owner') return ctx.otherAccount ?? '01JD8X0000000000000000OAK1';
  if (n === 'targetkind') return 'emote';
  if (n === 'action') return 'emote.approved';
  if (n === 'currency') return 'USD';
  if (n === 'email') return 'mock@example.test';
  return 'mock';
}
function idFor(name: string): string | undefined {
  const n = name.toLowerCase();
  const map: Record<string, string | undefined> = {
    emoteid: ctx.emoteId,
    setid: ctx.setId,
    accountid: ctx.otherAccount,
    toaccountid: ctx.otherAccount,
    ownerid: undefined,
    roleid: ctx.roleId,
    badgeid: ctx.badgeId,
    paintid: ctx.paintId,
    imageid: ctx.imageId,
    itemid: ctx.itemId,
    cosmeticid: ctx.badgeId,
    owner: ctx.otherAccount,
  };
  if (n in map) return map[n];
  return undefined;
}

function valueFor(type: GraphQLInputType, name: string, depth = 0): unknown {
  if (isNonNullType(type)) return valueFor(type.ofType, name, depth);
  if (isListType(type)) return [valueFor(type.ofType, name, depth)];
  const named = getNamedType(type);
  if (isEnumType(named)) return named.getValues()[0].name;
  if (isInputObjectType(named)) {
    if (depth > 3) return {};
    const out: Record<string, unknown> = {};
    for (const [k, f] of Object.entries(named.getFields())) {
      if (isNonNullType(f.type)) out[k] = valueFor(f.type, k, depth + 1);
    }
    return out;
  }
  switch (named.name) {
    case 'Int':
      return /per_?page|limit/i.test(name) ? 5 : /page/i.test(name) ? 1 : /price|amount|cents|count|max|uses|days/i.test(name) ? 100 : 1;
    case 'Float':
      return 0.5;
    case 'Boolean':
      return false;
    default: {
      const id = idFor(name);
      return id ?? stringFor(name);
    }
  }
}

// Per-operation variables where the generic guess is not enough (the mock is a stateful fake).
const OVERRIDES: Record<string, (c: Ctx) => Record<string, unknown>> = {
  Search: () => ({ query: 'cat' }),
  FindPeople: () => ({ query: 'lumen' }),
  Emote: (c) => ({ id: c.emoteId }),
  EmoteTrending: (c) => ({ id: c.emoteId }),
  EmoteChannels: (c) => ({ id: c.emoteId }),
  PublicAccount: (c) => ({ id: c.otherAccount }),
  Set: (c) => ({ id: c.setId }),
  PublicSet: (c) => ({ id: c.setId }),
  EditSet: (c) => ({ id: c.setId }),
  CloneSet: (c) => ({ id: c.setId, name: 'Cloned mock set', visibility: 'LISTED' }),
  UpdateEmote: (c) => ({ id: c.myEmoteId, input: { name: 'MockRenamed', tags: ['cat'], zeroWidth: false, nsfw: false } }),
  DeleteEmote: (c) => ({ id: c.deletableEmoteId }),
  CreateSet: () => ({ input: { name: 'Mock set', kind: 'PERSONAL', visibility: 'UNLISTED' } }),
  UpdateSet: (c) => ({ id: c.setId, input: { name: 'Mock set renamed' } }),
  DeleteSet: (c) => ({ id: c.deletableSetId }),
  AddSetEmote: (c) => ({ setId: c.setId, emoteId: c.emoteId, input: { name: 'MockAlias' } }),
  UpdateSetEmote: (c) => ({ setId: c.setId, emoteId: c.emoteId, input: { name: 'MockAlias2' } }),
  RemoveSetEmote: (c) => ({ setId: c.setId, emoteId: c.emoteId }),
  AddSetEditor: (c) => ({ setId: c.setId, accountId: c.otherAccount }),
  RemoveSetEditor: (c) => ({ setId: c.setId, accountId: c.otherAccount }),
  UpdateChannel: (c) => ({ platform: c.channelPlatform, id: c.channelId, input: { allowPersonalEmotes: true } }),
  AddAccountEditor: (c) => ({ input: { accountId: c.otherAccount, permissions: ['EMOTE_UPLOAD'] } }),
  UpdateAccountEditor: (c) => ({ accountId: c.otherAccount, permissions: ['EMOTE_UPLOAD'] }),
  RemoveAccountEditor: (c) => ({ accountId: c.otherAccount }),
  SetDisplayPlatform: () => ({ platform: 'TWITCH' }),
  Unlink: () => ({ platform: 'KICK' }),
  MarkRead: (c) => ({ ids: c.noteId ? [c.noteId] : [] }),
  RevokeSession: (c) => ({ id: c.otherSessionId }),
  DeleteAccount: () => ({ confirm: 'lumen_fox' }),
  Approve: (c) => ({ id: c.pendingEmoteId, visibility: 'PUBLIC' }),
  Reject: (c) => ({ id: c.pendingEmoteId2, reason: 'A mock reason' }),
  Takedown: (c) => ({ id: c.emoteId, reason: 'A mock reason' }),
  EmoteHistory: (c) => ({ emoteId: c.emoteId }),
  EmoteNotes: (c) => ({ emoteId: c.emoteId }),
  AddEmoteNote: (c) => ({ emoteId: c.emoteId, body: 'A mock note' }),
  DeleteEmoteNote: (c) => ({ id: c.emoteNoteId }),
  ResolveReport: (c) => ({ id: c.reportId, action: 'DISMISS', note: 'A mock note' }),
  CreateRole: () => ({ input: { name: 'Mock role', position: 5, permissions: [] } }),
  UpdateRole: (c) => ({ id: c.newRoleId, input: { name: 'Mock role 2' } }),
  DeleteRole: (c) => ({ id: c.newRoleId }),
  User: () => ({ platform: 'TWITCH', login: 'quietoak' }),
  Channel: () => ({ platform: 'TWITCH', login: 'quietoak' }),
  FindAccount: () => ({ platform: 'TWITCH', login: 'quietoak' }),
  UserDetails: (c) => ({ accountId: c.otherAccount, status: true, roles: true }),
  Restrict: (c) => ({ accountId: c.otherAccount, uploadsBlocked: false, blocked: false }),
  PlaceRestriction: (c) => ({ accountId: c.otherAccount, input: { kind: 'UPLOADS', reason: 'A mock reason' } }),
  LiftRestriction: (c) => ({ accountId: c.otherAccount, kind: 'UPLOADS', reason: 'A mock reason' }),
  AssignRole: (c) => ({ accountId: c.otherAccount, roleId: c.roleId }),
  UnassignRole: (c) => ({ accountId: c.otherAccount, roleId: c.roleId }),
  Audit: () => ({ limit: 5 }),
  ReportEmote: (c) => ({ id: c.emoteId, reason: 'A mock reason' }),
  SearchByOwner: (c) => ({ owner: c.otherAccount, page: 1, perPage: 5, sort: 'NEW' }),
  TransferEmote: (c) => ({ id: c.myEmoteId, toAccountId: c.editorAccount ?? c.meId }),
  EmoteTransferRequests: () => ({}),
  AcceptEmoteTransfer: (c) => ({ id: c.transferId }),
  DeclineEmoteTransfer: (c) => ({ id: c.transferId }),
  CancelEmoteTransfer: (c) => ({ id: c.transferId }),
  UserCosmetics: (c) => ({ platform: 'TWITCH', userIds: [c.otherPlatformId ?? '1'] }),
  AccountCosmetics: (c) => ({ accountIds: [c.otherAccount] }),
  MyCosmetics: () => ({}),
  SelectCosmetics: (c) => ({ badgeId: c.badgeId, paintId: c.paintId }),
  CosmeticImage: (c) => ({ id: c.imageId }),
  StudioCosmeticImage: (c) => ({ id: c.imageId }),
  Entitlements: () => ({ kind: 'BADGE', page: 1, perPage: 5 }),
  CreateBadge: (c) => ({ input: { name: 'Conformance badge', tooltip: 'A mock tooltip', imageId: c.imageId } }),
  UpdateBadge: (c) => ({ id: c.newBadgeId, input: { name: 'Conformance badge 2' } }),
  DeleteBadge: (c) => ({ id: c.newBadgeId }),
  DeletePaint: (c) => ({ id: c.newPaintId }),
  StudioCreatePaint: () => ({ input: PAINT_INPUT('Conformance paint') }),
  StudioUpdatePaint: (c) => ({ id: c.newPaintId, input: PAINT_INPUT('Conformance paint 2') }),
  GrantCosmetic: (c) => ({ input: { accountId: c.otherAccount, kind: 'BADGE', cosmeticId: c.badgeId, reason: 'A mock reason' } }),
  RevokeCosmetic: (c) => ({ accountId: c.otherAccount, kind: 'BADGE', cosmeticId: c.badgeId, reason: 'A mock reason' }),
  AccountRoles: (c) => ({ accountId: c.otherAccount }),
  AccountEditors: () => ({}),
  AccountSets: (c) => ({ accountId: c.otherAccount }),
  AccountActivity: (c) => ({ accountId: c.otherAccount, limit: 5 }),
  EmoteActivity: (c) => ({ emoteId: c.emoteId, limit: 5 }),
  ShopItem: (c) => ({ id: c.itemId }),
  StartCheckout: (c) => ({ itemId: c.lastItemId }),
  Order: (c) => ({ id: c.orderId }),
  RedeemCode: (c) => ({ code: c.code ?? 'MOCK-0000-0000' }),
  CreateShopItem: (c) => ({ input: { name: 'Conformance item', priceCents: 500, badgeIds: [c.badgeId], paddlePriceId: 'pri_01conformance' } }),
  UpdateShopItem: (c) => ({ id: c.newItemId, input: { name: 'Conformance item 2', priceCents: 600, badgeIds: [c.badgeId], paddlePriceId: 'pri_01conformance' } }),
  DeleteShopItem: (c) => ({ id: c.newItemId }),
  AdminOrders: () => ({ page: 1, perPage: 5 }),
  RefundOrder: (c) => ({ id: c.orderId, reason: 'A mock reason' }),
  CodeBatches: () => ({ page: 1, perPage: 5 }),
  CreateCodeBatch: (c) => ({ input: { count: 2, prefix: 'TEST', usesPerCode: 1, badgeIds: [c.newBadgeId], name: 'Conformance batch' } }),
  DisableCodeBatch: (c) => ({ id: c.batchId }),
  QuickSearch: () => ({ query: 'cat' }),
  StaffNotes: (c) => ({ accountId: c.otherAccount }),
  AddStaffNote: (c) => ({ accountId: c.otherAccount, body: 'A mock note' }),
  DeleteStaffNote: (c) => ({ id: c.staffNoteId }),
  AccountSessions: (c) => ({ accountId: c.otherAccount }),
  AccountSubscriptions: (c) => ({ accountId: c.otherAccount }),
  EntitlementGraph: (c) => ({ accountId: c.otherAccount }),
  EntitlementHolders: (c) => ({ kind: 'BADGE', id: c.badgeId }),
  AccountProfile: (c) => ({ accountId: c.otherAccount }),
  AccountEmotes: (c) => ({ accountId: c.otherAccount, page: 1, perPage: 5 }),
  AccountReports: (c) => ({ accountId: c.otherAccount }),
  AdminAccount: (c) => ({ accountId: c.otherAccount }),
  AccountRestrictions: (c) => ({ accountId: c.otherAccount }),
  AdminCounts: () => ({ review: true, reports: true }),
  ShopCounts: () => ({ items: true, orders: true, codes: true }),
  ReviewQueue: () => ({ page: 1, perPage: 5 }),
  ReportQueue: () => ({ page: 1, perPage: 5 }),
  MyOrders: () => ({ page: 1, perPage: 5 }),
};
const PAINT_INPUT = (name: string) => ({
  name,
  kind: 'LINEAR_GRADIENT',
  angle: 90,
  repeat: false,
  shape: 'CIRCLE',
  color: '#336699',
  stops: [
    { at: 0, color: '#336699' },
    { at: 1, color: '#993366' },
  ],
  shadows: [],
});

// Allowed to answer a GraphQL error, with the reason. Everything else must answer data.
const EXPECTED_ERRORS: Record<string, string> = {
  CancelEmoteTransfer: 'only the editor who asked can cancel; the one seeded request was asked by someone else, so the mock answers forbidden (same as the API)',
  AcceptEmoteTransfer: 'the seeded request is declined first, so it is no longer open (not_pending, as the API)',
};

// ---- Checks ---------------------------------------------------------------------------------------
function responseKeys(set: SelectionSetNode, doc: DocumentNode, into = new Set<string>()): Set<string> {
  for (const s of set.selections) {
    if (s.kind === Kind.FIELD) into.add(s.alias?.value ?? s.name.value);
    else if (s.kind === Kind.INLINE_FRAGMENT) responseKeys(s.selectionSet, doc, into);
    else {
      const f = doc.definitions.find((d) => d.kind === Kind.FRAGMENT_DEFINITION && d.name.value === s.name.value);
      if (f && f.kind === Kind.FRAGMENT_DEFINITION) responseKeys(f.selectionSet, doc, into);
    }
  }
  return into;
}
function fieldSelections(set: SelectionSetNode, doc: DocumentNode, key: string): SelectionSetNode[] {
  const out: SelectionSetNode[] = [];
  for (const s of set.selections) {
    if (s.kind === Kind.FIELD) {
      if ((s.alias?.value ?? s.name.value) === key && s.selectionSet) out.push(s.selectionSet);
    } else if (s.kind === Kind.INLINE_FRAGMENT) out.push(...fieldSelections(s.selectionSet, doc, key));
    else {
      const f = doc.definitions.find((d) => d.kind === Kind.FRAGMENT_DEFINITION && d.name.value === s.name.value);
      if (f && f.kind === Kind.FRAGMENT_DEFINITION) out.push(...fieldSelections(f.selectionSet, doc, key));
    }
  }
  return out;
}
function extraKeys(value: unknown, sets: SelectionSetNode[], doc: DocumentNode, path: string, out: string[]) {
  if (Array.isArray(value)) {
    for (const v of value) extraKeys(v, sets, doc, `${path}[]`, out);
    return;
  }
  if (value === null || typeof value !== 'object' || !sets.length) return;
  const allowed = new Set<string>();
  for (const s of sets) responseKeys(s, doc, allowed);
  for (const [k, v] of Object.entries(value as object)) {
    if (k === '__typename') continue;
    if (!allowed.has(k)) {
      out.push(`${path}.${k}`);
      continue;
    }
    extraKeys(v, sets.flatMap((s) => fieldSelections(s, doc, k)), doc, `${path}.${k}`, out);
  }
}

function conformance(doc: DocumentNode, data: Record<string, unknown>, opName: string, vars: Record<string, unknown>): string[] {
  const problems: string[] = [];
  const result = execute({
    schema,
    document: doc,
    rootValue: data,
    contextValue: {},
    variableValues: vars,
    fieldResolver: (source: any, _args, _ctx, info) => (source == null ? null : source[info.path.key as string] ?? null),
  });
  const res = result as { errors?: readonly { message: string; path?: readonly (string | number)[] }[] };
  for (const e of res.errors ?? []) {
    // Variables are not needed to execute against fixed data; arguments are ignored by the resolver.
    problems.push(`${opName}: ${e.path?.join('.') ?? ''} ${e.message}`);
  }
  const op = doc.definitions.find((d) => d.kind === Kind.OPERATION_DEFINITION);
  if (op && op.kind === Kind.OPERATION_DEFINITION) {
    const extras: string[] = [];
    for (const [k, v] of Object.entries(data)) {
      const sets = fieldSelections(op.selectionSet, doc, k);
      if (!responseKeys(op.selectionSet, doc).has(k)) extras.push(k);
      else extraKeys(v, sets, doc, k, extras);
    }
    for (const x of extras) problems.push(`${opName}: mock-only key ${x}`);
  }
  return problems;
}

// ---- A small OpenAPI response checker --------------------------------------------------------------
function resolveRef(s: any): any {
  if (s?.$ref) {
    const name = s.$ref.replace('#/components/schemas/', '');
    return { ...openapi.components.schemas[name], __name: name };
  }
  return s;
}
function checkSchema(value: unknown, schemaIn: any, path: string, out: string[]) {
  const s = resolveRef(schemaIn);
  if (!s) return;
  if (s.allOf) return s.allOf.forEach((x: any) => checkSchema(value, x, path, out));
  if (s.oneOf || s.anyOf) {
    const alts: any[] = s.oneOf ?? s.anyOf;
    const tries = alts.map((a) => {
      const errs: string[] = [];
      checkSchema(value, a, path, errs);
      return errs;
    });
    if (!tries.some((t) => !t.length)) out.push(`${path}: matches none of the alternatives (${tries[0]?.[0] ?? ''})`);
    return;
  }
  const types: string[] = Array.isArray(s.type) ? s.type : s.type ? [s.type] : [];
  if (value === null) {
    if (s.nullable || types.includes('null') || !types.length) return;
    return void out.push(`${path}: null where ${types.join('|')} is required`);
  }
  if (s.enum && !s.enum.includes(value)) out.push(`${path}: ${JSON.stringify(value)} is not one of ${s.enum.join(', ')}`);
  const kind = Array.isArray(value) ? 'array' : typeof value === 'number' ? (Number.isInteger(value) ? 'integer' : 'number') : typeof value;
  if (types.length && !types.some((t) => t === kind || (t === 'number' && kind === 'integer'))) {
    return void out.push(`${path}: is ${kind}, the API says ${types.join('|')}`);
  }
  if (Array.isArray(value)) value.forEach((v, i) => checkSchema(v, s.items, `${path}[${i}]`, out));
  else if (value && typeof value === 'object') {
    const props = s.properties ?? {};
    for (const r of s.required ?? []) if (!(r in (value as object))) out.push(`${path}.${r}: required, missing`);
    for (const [k, v] of Object.entries(value as object)) {
      if (k in props) checkSchema(v, props[k], `${path}.${k}`, out);
      else if (s.additionalProperties && typeof s.additionalProperties === 'object') checkSchema(v, s.additionalProperties, `${path}.${k}`, out);
      else if (s.additionalProperties !== true) out.push(`${path}.${k}: not in the API's schema`);
    }
  }
}
function checkResponse(method: string, p: string, status: number, body: unknown, label: string, out: string[]) {
  const op = openapi.paths[p]?.[method];
  const r = op?.responses?.[String(status)];
  if (!r) return void out.push(`${label}: the API documents no ${status} for ${method.toUpperCase()} ${p}`);
  const js = r.content?.['application/json']?.schema;
  if (js) checkSchema(body, js, label, out);
  else if (body !== undefined && body !== null && body !== '') out.push(`${label}: the API documents no body for ${status}`);
}

// ---- The run ---------------------------------------------------------------------------------------
const failures: string[] = [];
const stats = { validated: 0, executed: 0, rest: 0 };

beforeAll(async () => {
  const m = await startMock();
  base = m.base;
  child = m.proc;
}, 30000);
afterAll(() => {
  if (child && !child.killed) child.kill();
});

function varsFor(op: { name: string; text: string }, doc: DocumentNode): Record<string, unknown> {
  if (OVERRIDES[op.name]) return OVERRIDES[op.name](ctx);
  const def = doc.definitions.find((d) => d.kind === Kind.OPERATION_DEFINITION);
  const out: Record<string, unknown> = {};
  const defs = (def && def.kind === Kind.OPERATION_DEFINITION ? def.variableDefinitions : []) as readonly VariableDefinitionNode[] | undefined;
  for (const v of defs ?? []) {
    const t = typeFromAST(schema, v.type as any) as GraphQLInputType | undefined;
    if (!t) continue;
    if (isNonNullType(t)) out[v.variable.name.value] = valueFor(t, v.variable.name.value);
  }
  return out;
}

// Where the ids come from: the mock's own earlier answers.
const SEEDS: { op: string; take: (d: any, c: Ctx) => void }[] = [
  {
    op: 'Search',
    take: (d, c) => {
      const items = d.searchEmotes.items;
      c.emoteId = items[0]?.id;
      c.otherAccount = items.find((i: any) => i.owner?.id && i.owner.id !== c.meId)?.owner?.id;
    },
  },
  { op: 'EditorOf', take: (d, c) => (c.editorAccount = d.editorOf?.[0]?.account?.id) },
  { op: 'Me', take: (d, c) => (c.meId = d.me.id) },
  { op: 'MyEmotes', take: (d, c) => { c.myEmoteId = d.myEmotes.items[0]?.id; c.deletableEmoteId = d.myEmotes.items.at(-1)?.id; } },
  { op: 'MySets', take: (d, c) => { const s = pick(d, 'mySets') ?? pick(d, 'me.sets') ?? []; c.setId = s[0]?.id; c.deletableSetId = s.at(-1)?.id; } },
  { op: 'MyChannels', take: (d, c) => { const ch = pick(d, 'myChannels') ?? []; c.channelId = ch[0]?.platformChannelId; c.channelPlatform = ch[0]?.platform; } },
  { op: 'Cosmetics', take: (d, c) => { c.badgeId = d.cosmetics.badges[0]?.id; c.paintId = d.cosmetics.paints[0]?.id; } },
  { op: 'ShopItems', take: (d, c) => { const it = pick(d, 'shopItems') ?? []; c.itemId = it[0]?.id; c.lastItemId = it.at(-1)?.id; } },
  { op: 'Roles', take: (d, c) => { const r = pick(d, 'roles') ?? []; c.roleId = r.find((x: any) => !x.builtIn)?.id ?? r[0]?.id; } },
  { op: 'ReviewQueue', take: (d, c) => { const it = pick(d, 'reviewQueue.items') ?? []; c.pendingEmoteId = it[0]?.id; c.pendingEmoteId2 = it[1]?.id ?? it[0]?.id; } },
  { op: 'ReportQueue', take: (d, c) => { const it = pick(d, 'reportQueue.items') ?? pick(d, 'reports.items') ?? []; c.reportId = it[0]?.id; } },
  { op: 'MyNotifications', take: (d, c) => { const it = pick(d, 'myNotifications.items') ?? pick(d, 'notifications.items') ?? []; c.noteId = it[0]?.id; } },
  { op: 'MySessions', take: (d, c) => { const it = pick(d, 'mySessions') ?? []; c.otherSessionId = it.find((s: any) => !s.current)?.id ?? it[0]?.id; } },
  { op: 'MyOrders', take: (d, c) => { const it = pick(d, 'myOrders.items') ?? []; c.orderId = it[0]?.id; } },
  { op: 'CodeBatches', take: (d, c) => { const it = pick(d, 'codeBatches.items') ?? []; c.batchId = it[0]?.id; } },
  { op: 'Cosmetics', take: (d, c) => (c.imageId ??= d.cosmetics.badges[0]?.image?.id) },
];
// Mutations that make something the next ones use.
const CREATES: Record<string, (d: any, c: Ctx) => void> = {
  CreateCodeBatch: (d, c) => (c.code = d.createCodeBatch.codes[0]),
  CreateRole: (d, c) => (c.newRoleId = d.createRole.id),
  CreateBadge: (d, c) => (c.newBadgeId = d.createBadge.id),
  StudioCreatePaint: (d, c) => (c.newPaintId = d.createPaint.id),
  CreateShopItem: (d, c) => (c.newItemId = d.createShopItem.id),
  AddEmoteNote: (d, c) => (c.emoteNoteId = d.addEmoteNote?.id),
  AddStaffNote: (d, c) => (c.staffNoteId = d.addStaffNote?.id),
  EmoteTransferRequests: (d, c) => (c.transferId = d.emoteTransferRequests?.[0]?.id),
};
// Order matters for state: the ones that make things first, the destructive ones last.
const FIRST = ['Me', 'EditorOf', 'Search', 'MyEmotes', 'MySets', 'MyChannels', 'Cosmetics', 'ShopItems', 'Roles', 'ReviewQueue', 'ReportQueue', 'MyNotifications', 'MySessions', 'MyOrders', 'CodeBatches'];
const LAST = ['DeleteEmote', 'DeleteSet', 'SignOutEverywhere', 'RevokeOtherSessions', 'Unlink', 'DeleteAccount', 'RestoreAccount'];
const AFTER_CREATE = ['RedeemCode', 'TransferEmote', 'EmoteTransferRequests', 'DeclineEmoteTransfer', 'AcceptEmoteTransfer', 'CancelEmoteTransfer'];

describe('mock-api conformance with the real schema', () => {
  it('every website operation is valid against schema/schema.graphql', () => {
    const bad: string[] = [];
    for (const op of operations) {
      const text = withFragments(op.text);
      let doc: DocumentNode;
      try {
        doc = parse(text);
      } catch (e) {
        bad.push(`${op.name} (${op.file}): does not parse: ${(e as Error).message}`);
        continue;
      }
      stats.validated += 1;
      for (const err of validate(schema, doc)) bad.push(`${op.name} (${op.file}): ${err.message}`);
    }
    expect(operations.length).toBeGreaterThan(100);
    expect(bad, `invalid against the real schema:\n${bad.join('\n')}`).toEqual([]);
  });

  it('the mock answers every operation in the real schema shape', async () => {
    // Seeds first (they feed ids), then the rest in file order, the state-destroying ones last.
    const rank = (n: string) => (FIRST.includes(n) ? FIRST.indexOf(n) : LAST.includes(n) ? 1000 + LAST.indexOf(n) : AFTER_CREATE.includes(n) ? 500 + AFTER_CREATE.indexOf(n) : /^(Create|StudioCreate|Add)/.test(n) ? 50 : /^Delete/.test(n) ? 700 : 100);
    const ordered = [...operations].sort((a, b) => rank(a.name) - rank(b.name));
    const cache = new Map<string, any>();
    await fetch(`${base}/v1/auth/kick/login?link=true`, { redirect: 'manual' }); // a second identity, so Unlink can run
    for (const op of ordered) {
      const text = withFragments(op.text);
      const doc = parse(text);
      const vars = varsFor(op, doc);
      let answer: Awaited<ReturnType<typeof gql>>;
      try {
        answer = await gql(text, vars, op.name === 'RestoreAccount' ? 'mock_restore=1; mock_admin=1' : COOKIE);
      } catch (e) {
        failures.push(`${op.name}: request failed: ${(e as Error).message}`);
        continue;
      }
      stats.executed += 1;
      if (answer.errors?.length || !answer.data) {
        if (!(op.name in EXPECTED_ERRORS)) {
          failures.push(`${op.name}: the mock answered an error (${answer.errors?.map((e) => `${e.extensions?.code ?? ''} ${e.message}`).join('; ') ?? 'no data'}) for ${JSON.stringify(vars)}`);
        }
        continue;
      }
      cache.set(op.name, answer.data);
      for (const s of SEEDS) if (s.op === op.name) try { s.take(answer.data, ctx); } catch { /* shape problems are reported below */ }
      CREATES[op.name]?.(answer.data, ctx);
      failures.push(...conformance(doc, answer.data, op.name, vars));
    }
  }, 120000);

  it('badge and paint names are unique per kind (as the real API does)', async () => {
    const mk = (kind: 'Badge' | 'Paint', name: string) =>
      kind === 'Badge'
        ? gql('mutation M($input: CreateBadgeInput!) { createBadge(input: $input) { id } }', { input: { name, tooltip: 'x', imageId: ctx.imageId } })
        : gql('mutation M($input: PaintInput!) { createPaint(input: $input) { id } }', { input: PAINT_INPUT(name) });
    const a = await mk('Badge', 'Per kind probe');
    const b = await mk('Paint', 'Per kind probe');
    const c = await mk('Badge', 'per kind PROBE');
    const d = await mk('Paint', 'per kind PROBE');
    if (a.errors) failures.push(`name per kind: first badge failed: ${a.errors[0].message}`);
    if (b.errors) failures.push(`name per kind: a paint named like a badge was refused (${b.errors[0].extensions?.code}); the real API checks per kind`);
    if (c.errors?.[0]?.extensions?.code !== 'name_taken') failures.push('name per kind: a second badge with the same name (any case) was not refused with name_taken');
    if (d.errors?.[0]?.extensions?.code !== 'name_taken') failures.push('name per kind: a second paint with the same name (any case) was not refused with name_taken');
  });

  it('REST answers match openapi/openapi.json', async () => {
    const out: string[] = [];
    const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==', 'base64');
    const form = (fields: Record<string, string>, bytes = png) => {
      const f = new FormData();
      for (const [k, v] of Object.entries(fields)) f.set(k, v);
      f.set('file', new Blob([bytes], { type: 'image/png' }), 'x.png');
      return f;
    };
    const post = (path: string, body: FormData, cookie = COOKIE) => fetch(`${base}${path}`, { method: 'POST', body, headers: { cookie } });

    // Cosmetic image upload, twice with the same bytes: the same picture again.
    const first = await post('/v1/cosmetics/images', form({ kind: 'badge' }));
    stats.rest += 1;
    const firstBody = await first.json();
    checkResponse('post', '/v1/cosmetics/images', first.status, firstBody, 'POST /v1/cosmetics/images', out);
    const again = await post('/v1/cosmetics/images', form({ kind: 'badge' }));
    stats.rest += 1;
    const againBody = await again.json();
    checkResponse('post', '/v1/cosmetics/images', again.status, againBody, 'POST /v1/cosmetics/images (same bytes again)', out);
    if (againBody?.id !== firstBody?.id) out.push('POST /v1/cosmetics/images: the same bytes gave a different picture id (the API gives the same picture again)');
    const other = await post('/v1/cosmetics/images', form({ kind: 'badge' }, Buffer.concat([png, Buffer.from('x')])));
    const otherBody = await other.json();
    if (otherBody?.id === firstBody?.id) out.push('POST /v1/cosmetics/images: different bytes gave the same picture id');
    stats.rest += 1;
    for (const [label, res, status] of [
      ['signed out', await post('/v1/cosmetics/images', form({ kind: 'badge' }), ''), 401],
      ['bad kind', await post('/v1/cosmetics/images', form({ kind: 'nope' })), 400],
    ] as const) {
      stats.rest += 1;
      if (res.status !== status) out.push(`POST /v1/cosmetics/images (${label}): status ${res.status}, the API answers ${status}`);
      else checkResponse('post', '/v1/cosmetics/images', res.status, await res.json(), `POST /v1/cosmetics/images (${label})`, out);
    }

    // Emote upload.
    const em = await post('/v1/emotes', form({ name: 'ConformanceEmote', tags: 'cat,mock', zero_width: 'false', private: 'false', nsfw: 'false' }));
    stats.rest += 1;
    checkResponse('post', '/v1/emotes', em.status, await em.json(), 'POST /v1/emotes', out);
    for (const [name, status] of [['blocked', 422], ['ratelimited', 429], ['toobig', 413], ['broken', 422]] as const) {
      const r = await post('/v1/emotes', form({ name }));
      stats.rest += 1;
      checkResponse('post', '/v1/emotes', r.status, await r.json(), `POST /v1/emotes (${name})`, out);
      if (r.status !== status) out.push(`POST /v1/emotes (${name}): status ${r.status}, expected ${status}`);
    }
    const so = await post('/v1/emotes', form({ name: 'x' }), '');
    stats.rest += 1;
    checkResponse('post', '/v1/emotes', so.status, await so.json(), 'POST /v1/emotes (signed out)', out);

    // Sign-in redirect and sign-out.
    for (const platform of ['twitch', 'kick']) {
      const r = await fetch(`${base}/v1/auth/${platform}/login?return_to=/x`, { redirect: 'manual' });
      stats.rest += 1;
      const known = openapi.paths['/v1/auth/{platform}/login'].get.responses['302'];
      if (r.status !== 302 || !known) out.push(`GET /v1/auth/${platform}/login: status ${r.status}, the API answers 302`);
      if (!r.headers.get('location')) out.push(`GET /v1/auth/${platform}/login: no Location header`);
    }
    const lo = await fetch(`${base}/v1/auth/logout`, { method: 'POST', headers: { cookie: COOKIE } });
    stats.rest += 1;
    checkResponse('post', '/v1/auth/logout', lo.status, (await lo.text()) || undefined, 'POST /v1/auth/logout', out);

    failures.push(...out);
  }, 30000);

  it('collects every difference in one report', () => {
    // eslint-disable-next-line no-console
    console.log(`mock conformance: ${stats.validated} operations validated, ${stats.executed} executed against the mock, ${stats.rest} REST calls`);
    expect(failures, `\n${failures.join('\n')}\n`).toEqual([]);
  });
});
