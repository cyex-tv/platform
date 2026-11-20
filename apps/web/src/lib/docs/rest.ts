// The REST reference, generated from openapi/openapi.json (ADR 0008) when the site is built.
// Nothing here is a hand-kept list of endpoints: add a path to the spec and it appears.
import spec from '../../../../../openapi/openapi.json';
import { EX, EXAMPLE_TAGS, SESSION_ENV, exampleBool, exampleNumber, exampleString, paragraphs, py, shellQuote, type Segment } from './example';

interface Schema {
  $ref?: string;
  type?: string | string[];
  format?: string;
  description?: string;
  enum?: string[];
  properties?: Record<string, Schema>;
  required?: string[];
  items?: Schema;
  oneOf?: Schema[];
  allOf?: Schema[];
  anyOf?: Schema[];
  default?: unknown;
  example?: unknown;
}
interface Parameter {
  name: string;
  in: string;
  description?: string;
  required?: boolean;
  schema?: Schema;
}
interface Operation {
  tags?: string[];
  summary?: string;
  description?: string;
  operationId?: string;
  parameters?: Parameter[];
  requestBody?: { required?: boolean; content: Record<string, { schema?: Schema }> };
  responses: Record<string, { description?: string; content?: Record<string, { schema?: Schema }> }>;
}
interface OpenApi {
  info: { title: string; description?: string; version: string };
  servers: { url: string }[];
  tags?: { name: string; description?: string }[];
  paths: Record<string, Record<string, Operation>>;
  components: { schemas: Record<string, Schema> };
}

const api = spec as unknown as OpenApi;
const METHODS = ['get', 'put', 'post', 'delete', 'options', 'head', 'patch', 'trace'];

export const baseUrl: string = api.servers[0].url.replace(/\/+$/, '');
export const restInfo = { title: api.info.title, version: api.info.version, description: api.info.description ?? '' };

export interface Field {
  name: string;
  type: string;
  required: boolean;
  description: Segment[][];
  values: string[];
  default?: string;
}
export interface ResponseRow {
  status: string;
  ok: boolean;
  description: Segment[][];
}
export interface Sample {
  lang: string;
  label: string;
  code: string;
}
export interface RestOp {
  id: string;
  method: string;
  path: string;
  tag: string;
  title: string;
  /** The rest of the summary, when it ran to more than a title. */
  summaryRest: Segment[][];
  description: Segment[][];
  signedIn: boolean;
  pathParams: Field[];
  queryParams: Field[];
  headerParams: Field[];
  bodyType?: string;
  bodyRequired?: boolean;
  bodyFields: Field[];
  responses: ResponseRow[];
  samples: Sample[];
  responseStatus: string;
  responseType: string;
  responseExample: string;
}
export interface RestGroup {
  tag: string;
  title: string;
  description: string;
  ops: RestOp[];
}

function refName(schema: Schema): string | undefined {
  return schema.$ref?.split('/').pop();
}

function resolve(schema: Schema | undefined): Schema {
  let s = schema ?? {};
  for (let i = 0; s.$ref && i < 10; i++) s = api.components.schemas[refName(s)!] ?? {};
  return s;
}

/** `string`, `Platform`, `EmoteFile[]`, `string | null`. */
function typeOf(schema: Schema | undefined): string {
  if (!schema) return 'any';
  if (schema.$ref) return refName(schema)!;
  const variants = schema.oneOf ?? schema.anyOf;
  if (variants) return variants.map(typeOf).join(' | ');
  if (schema.allOf) return schema.allOf.map(typeOf).join(' & ');
  const t = Array.isArray(schema.type) ? schema.type : [schema.type ?? 'any'];
  return t
    .map((x) => (x === 'array' ? `${typeOf(schema.items)}[]` : x === 'string' && schema.format === 'binary' ? 'file' : x))
    .join(' | ');
}

function enumOf(schema: Schema | undefined): string[] {
  return (resolve(schema).enum ?? []).map(String);
}

/** An example value built from a schema; refs followed, cycles and depth cut off. */
function exampleOf(schema: Schema | undefined, name = '', depth = 0, seen: string[] = []): unknown {
  if (!schema) return null;
  const ctx = seen[seen.length - 1] ?? '';
  if (schema.example !== undefined) return schema.example;
  const ref = refName(schema);
  if (ref && seen.includes(ref)) return null;
  const next = ref ? [...seen, ref] : seen;
  const s = resolve(schema);
  if (s.example !== undefined) return s.example;
  if (s.enum?.length) return s.enum[0];
  const variants = (s.oneOf ?? s.anyOf)?.filter((v) => v.type !== 'null');
  if (variants?.length) return exampleOf(variants[0], name, depth, next);
  if (s.allOf?.length) {
    return Object.assign({}, ...s.allOf.map((v) => exampleOf(v, name, depth, next) as object));
  }
  const type = (Array.isArray(s.type) ? s.type.find((t) => t !== 'null') : s.type) ?? 'object';
  switch (type) {
    case 'string':
      return s.format === 'binary' ? `${EX.emoteName}.png` : exampleString(name, ctx);
    case 'integer':
    case 'number':
      return exampleNumber(name);
    case 'boolean':
      return exampleBool(name);
    case 'array':
      if (name === 'tags') return EXAMPLE_TAGS;
      return depth > 6 ? [] : [exampleOf(s.items, name, depth + 1, next)];
    default: {
      if (depth > 6) return {};
      const out: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(s.properties ?? {})) out[key] = exampleOf(value, key, depth + 1, next);
      return out;
    }
  }
}

function fieldOf(name: string, schema: Schema | undefined, required: boolean, description?: string): Field {
  const s = resolve(schema);
  return {
    name,
    type: typeOf(schema),
    required,
    description: paragraphs(description ?? s.description ?? schema?.description),
    values: enumOf(schema),
    default: s.default === undefined ? undefined : String(s.default),
  };
}

function paramFields(op: Operation, where: string): Field[] {
  return (op.parameters ?? [])
    .filter((p) => p.in === where)
    .map((p) => fieldOf(p.name, p.schema, Boolean(p.required) || p.in === 'path', p.description));
}

/** `Upload an emote. It starts pending…` becomes a title and the rest. */
function splitSummary(summary: string): [string, string] {
  const flat = summary.replace(/\s*\n\s*/g, ' ').trim();
  const cut = flat.search(/[.!?](\s|$)/);
  if (cut === -1) return [flat, ''];
  return [flat.slice(0, cut), flat.slice(cut + 1).trim()];
}

const json = (v: unknown, indent?: number) => JSON.stringify(v, null, indent);

/** Titles as short verb phrases; the spec's summary becomes the first line of the description. */
const TITLES: Record<string, string> = {
  get_channel: 'Get a channel',
  get_set: 'Get a set',
  get_global_set: 'Get the global set',
  get_emote: 'Get an emote',
  lookup_personal_sets: 'Look up personal sets',
  upload_emote: 'Upload an emote',
  login: 'Start sign-in',
  callback: 'Finish sign-in',
  logout: 'Sign out',
  authorize: 'Start app sign-in',
  get_client: 'Get an app',
  consent: 'Answer for an app',
  token: 'Get app tokens',
  revoke: 'Sign an app out',
  health: 'Check the process',
  ready: 'Check the database',
};

/** Groups by what they act on, reads first: that's what most callers want. */
const GROUPS: { tag: string; title: string }[] = [
  { tag: 'reads', title: 'Reads' },
  { tag: 'emotes', title: 'Uploads' },
  { tag: 'auth', title: 'Sign-in' },
  { tag: 'system', title: 'Health' },
];

/** Calls that act for an account: they need the session cookie and the site's Origin. */
const SIGNED_IN = new Set(['upload_emote', 'logout', 'consent']);

export const SITE = 'https://cyex.tv';
export const COOKIE = 'cyex_tv_session';

function samplesFor(
  method: string,
  path: string,
  op: Operation,
  pathParams: Field[],
  queryParams: Field[],
  bodyType: string | undefined,
  redirect: boolean,
  hasBody: boolean,
  signedIn: boolean,
): Sample[] {
  const valueOf = (f: Field) => String(f.values[0] ?? exampleString(f.name, path));
  const filled = pathParams.reduce((p, f) => p.replace(`{${f.name}}`, encodeURIComponent(valueOf(f))), path);
  const query = queryParams
    .filter((f) => f.required)
    .map((f) => `${f.name}=${encodeURIComponent(valueOf(f))}`)
    .join('&');
  const url = `${baseUrl}${filled}${query ? `?${query}` : ''}`;
  const M = method.toUpperCase();
  const lower = method.toLowerCase();

  const curl: string[] = [`curl ${redirect ? '-i' : '-s'}${M === 'GET' ? '' : ` -X ${M}`} ${shellQuote(url)}`];
  if (signedIn) curl.push(`-b "${COOKIE}=$${SESSION_ENV}"`, `-H 'origin: ${SITE}'`);
  const init: string[] = [];
  const pyArgs: string[] = [];
  let prelude = '';
  let pyPrelude = '';

  if (bodyType && op.requestBody) {
    const schema = op.requestBody.content[bodyType]?.schema;
    const s = resolve(schema);
    if (bodyType === 'application/json') {
      const example = exampleOf(schema);
      curl.push(`-H 'content-type: application/json'`, `-d ${shellQuote(json(example)!)}`);
      init.push(`headers: { 'content-type': 'application/json' }`, `body: JSON.stringify(${json(example, 2)!.replace(/\n/g, '\n  ')})`);
      pyArgs.push(`json=${py(example, 1)}`);
    } else if (bodyType === 'application/x-www-form-urlencoded') {
      // The app sign-in forms (OAuth): every field, as the guide shows them.
      // The token sample trades a code; refreshing is in the App sign-in guide.
      const value = (k: string) =>
        k === 'grant_type' ? 'authorization_code' : k === 'client_id' ? 'cyex-deck' : k === 'redirect_uri' ? 'http://127.0.0.1:53682/callback' : '…';
      const pairs = Object.keys(s.properties ?? {})
        .filter((k) => k !== 'token_type_hint' && k !== 'refresh_token')
        .map((k) => [k, value(k)] as const);
      for (const [k, v] of pairs) curl.push(`-d ${shellQuote(`${k}=${v}`)}`);
      init.push(`body: new URLSearchParams(${json(Object.fromEntries(pairs), 2)!.replace(/\n/g, '\n  ')})`);
      pyArgs.push(`data=${py(Object.fromEntries(pairs), 1)}`);
    } else if (bodyType === 'multipart/form-data') {
      const required = s.required ?? [];
      const parts = Object.keys(s.properties ?? {})
        .filter((k) => required.includes(k) || k === 'tags')
        .map((k) => {
          const binary = resolve(s.properties![k]).format === 'binary';
          return { k, binary, v: binary ? `${EX.emoteName}.png` : String(exampleOf(s.properties![k], k)) };
        });
      for (const p of parts) curl.push(`-F ${shellQuote(p.binary ? `${p.k}=@${p.v}` : `${p.k}=${p.v}`)}`);
      prelude =
        `const form = new FormData();\n` +
        parts.map((p) => (p.binary ? `form.append('${p.k}', fileInput.files[0]);` : `form.append('${p.k}', '${p.v}');`)).join('\n') +
        `\n\n`;
      init.push('body: form');
      const file = parts.find((p) => p.binary);
      const fields = parts.filter((p) => !p.binary);
      if (file) pyArgs.push(`files={"${file.k}": open("${file.v}", "rb")}`);
      if (fields.length) pyArgs.push(`data={${fields.map((p) => `"${p.k}": "${p.v}"`).join(', ')}}`);
    }
  }
  if (signedIn) {
    init.push(`credentials: 'include'`);
    pyPrelude = 'import os\n';
    pyArgs.push(`cookies={"${COOKIE}": os.environ["${SESSION_ENV}"]}`, `headers={"origin": "${SITE}"}`);
  }
  if (redirect) pyArgs.push('allow_redirects=False');

  const opts = [...(M === 'GET' ? [] : [`method: '${M}'`]), ...init];
  const call = opts.length ? `fetch('${url}', {\n${opts.map((o) => `  ${o},`).join('\n')}\n})` : `fetch('${url}')`;
  const js: string[] = redirect
    ? [`// Send the browser there; the platform's page takes over.`, `location.assign('${url}');`]
    : [`${prelude}const res = await ${call};`, hasBody ? 'const data = await res.json();' : 'console.log(res.status);'];

  const pyCall = pyArgs.length
    ? `requests.${lower}(\n    "${url}",\n${pyArgs.map((a) => `    ${a},`).join('\n')}\n)`
    : `requests.${lower}("${url}")`;
  const pyEnd = redirect ? 'print(res.headers["location"])' : hasBody ? 'data = res.json()' : 'print(res.status_code)';
  const python = `${pyPrelude}import requests\n\nres = ${pyCall}\n${pyEnd}`;

  return [
    { lang: 'curl', label: 'cURL', code: curl.join(' \\\n  ') },
    { lang: 'js', label: 'JavaScript', code: js.join('\n') },
    { lang: 'py', label: 'Python', code: python },
  ];
}

function redirectTarget(id: string | undefined): string {
  if (id === 'authorize') return 'https://cyex.tv/authorize?client_id=cyex-deck&redirect_uri=…&code_challenge=…&code_challenge_method=S256&state=…';
  if (id === 'login') return 'https://id.twitch.tv/oauth2/authorize?response_type=code&client_id=…&state=…&code_challenge=…';
  return 'https://cyex.tv/me';
}

function build(method: string, path: string, op: Operation): RestOp {
  const summary = op.summary ?? op.operationId ?? `${method.toUpperCase()} ${path}`;
  const named = op.operationId ? TITLES[op.operationId] : undefined;
  const [title, rest] = named ? [named, summary] : splitSummary(summary);
  const pathParams = paramFields(op, 'path');
  const queryParams = paramFields(op, 'query');
  const headerParams = paramFields(op, 'header');

  const bodyType = op.requestBody ? Object.keys(op.requestBody.content)[0] : undefined;
  const bodySchema = bodyType ? op.requestBody!.content[bodyType].schema : undefined;
  const bodyResolved = resolve(bodySchema);
  const bodyFields = Object.entries(bodyResolved.properties ?? {}).map(([name, schema]) =>
    fieldOf(name, schema, (bodyResolved.required ?? []).includes(name)),
  );

  const codes = Object.keys(op.responses);
  const success = codes.find((c) => c.startsWith('2')) ?? codes.find((c) => c.startsWith('3')) ?? codes[0];
  const redirect = success.startsWith('3');
  const successBody = Object.entries(op.responses[success]?.content ?? {})[0];
  const responseExample = successBody ? json(exampleOf(successBody[1].schema), 2)! : '';
  const signedIn = op.operationId ? SIGNED_IN.has(op.operationId) : codes.includes('401') && !redirect;

  return {
    id: `rest-${op.operationId ?? `${method}-${path}`.replace(/[^a-z0-9]+/gi, '-')}`,
    method: method.toUpperCase(),
    path,
    tag: op.tags?.[0] ?? 'other',
    title,
    summaryRest: paragraphs(rest),
    description: paragraphs(op.description),
    signedIn,
    pathParams,
    queryParams,
    headerParams,
    bodyType,
    bodyRequired: op.requestBody?.required,
    bodyFields,
    responses: codes.map((status) => ({
      status,
      ok: status === success,
      description: paragraphs(op.responses[status].description),
    })),
    samples: samplesFor(method, path, op, pathParams, queryParams, bodyType, redirect, Boolean(successBody), signedIn),
    responseStatus: success,
    responseType: successBody?.[0] ?? (redirect ? 'redirect' : 'no body'),
    responseExample:
      responseExample || (redirect ? `HTTP/2 ${success}\nlocation: ${redirectTarget(op.operationId)}` : `HTTP/2 ${success}`),
  };
}

/** Every operation in the spec, in the spec's own order. */
export function restOperations(): RestOp[] {
  const ops: RestOp[] = [];
  for (const [path, item] of Object.entries(api.paths)) {
    for (const method of METHODS) {
      if (item[method]) ops.push(build(method, path, item[method]));
    }
  }
  return ops;
}

/** The operations grouped by what they act on (GROUPS order, then any other tag). */
export function restGroups(): RestGroup[] {
  const ops = restOperations();
  const known = api.tags ?? [];
  const names = [...GROUPS.map((g) => g.tag), ...known.map((t) => t.name), ...ops.map((o) => o.tag)];
  return [...new Set(names)]
    .map((tag) => {
      const meta = known.find((t) => t.name === tag);
      const title = GROUPS.find((g) => g.tag === tag)?.title ?? tag[0].toUpperCase() + tag.slice(1);
      // Within a group, the order of TITLES: a channel before its set, sign-in before sign-out.
      const rank = (o: RestOp) => {
        const i = Object.keys(TITLES).indexOf(o.id.replace(/^rest-/, ''));
        return i === -1 ? Infinity : i;
      };
      const mine = ops.filter((o) => o.tag === tag).sort((a, b) => rank(a) - rank(b));
      return { tag, title, description: meta?.description ?? '', ops: mine };
    })
    .filter((g) => g.ops.length);
}
