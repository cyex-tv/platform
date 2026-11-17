// The REST reference, generated from openapi/openapi.json (ADR 0008) when the site is built.
// Nothing here is a hand-kept list of endpoints: add a path to the spec and it appears.
import spec from '../../../../../openapi/openapi.json';
import { exampleString, paragraphs, shellQuote, type Segment } from './example';

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
      return s.format === 'binary' ? 'cat.png' : exampleString(name);
    case 'integer':
    case 'number':
      return name === 'scale' ? 2 : name === 'size' ? 2048 : name === 'width' || name === 'height' ? 64 : 1;
    case 'boolean':
      return true;
    case 'array':
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

function samplesFor(
  method: string,
  path: string,
  op: Operation,
  pathParams: Field[],
  queryParams: Field[],
  bodyType: string | undefined,
  redirect: boolean,
  hasBody: boolean,
): Sample[] {
  const valueOf = (f: Field) => String(f.values[0] ?? exampleString(f.name));
  const filled = pathParams.reduce((p, f) => p.replace(`{${f.name}}`, encodeURIComponent(valueOf(f))), path);
  const query = queryParams
    .filter((f) => f.required)
    .map((f) => `${f.name}=${encodeURIComponent(valueOf(f))}`)
    .join('&');
  const url = `${baseUrl}${filled}${query ? `?${query}` : ''}`;
  const M = method.toUpperCase();

  const curl: string[] = [`curl ${redirect ? '-i' : '-s'}${M === 'GET' ? '' : ` -X ${M}`} ${shellQuote(url)}`];
  const init: string[] = [];
  let prelude = '';

  if (bodyType && op.requestBody) {
    const schema = op.requestBody.content[bodyType]?.schema;
    const s = resolve(schema);
    if (bodyType === 'application/json') {
      const example = exampleOf(schema);
      curl.push(`-H 'content-type: application/json'`, `-d ${shellQuote(json(example)!)}`);
      init.push(`headers: { 'content-type': 'application/json' }`, `body: JSON.stringify(${json(example, 2)!.replace(/\n/g, '\n  ')})`);
    } else if (bodyType === 'multipart/form-data') {
      const required = s.required ?? [];
      const parts = Object.keys(s.properties ?? {})
        .filter((k) => required.includes(k))
        .map((k) => {
          const binary = resolve(s.properties![k]).format === 'binary';
          return { k, binary, v: binary ? 'cat.png' : String(exampleOf(s.properties![k], k)) };
        });
      for (const p of parts) curl.push(`-F ${shellQuote(p.binary ? `${p.k}=@${p.v}` : `${p.k}=${p.v}`)}`);
      prelude =
        `const form = new FormData();\n` +
        parts.map((p) => (p.binary ? `form.append('${p.k}', fileInput.files[0]);` : `form.append('${p.k}', '${p.v}');`)).join('\n') +
        `\n\n`;
      init.push(`credentials: 'include'`, 'body: form');
    }
  }

  const opts = [...(M === 'GET' ? [] : [`method: '${M}'`]), ...init];
  const call = opts.length ? `fetch('${url}', {\n${opts.map((o) => `  ${o},`).join('\n')}\n})` : `fetch('${url}')`;
  const lines: string[] = redirect
    ? [`// Send the browser there; the platform's page takes over.`, `location.assign('${url}');`]
    : [`${prelude}const res = await ${call};`, hasBody ? 'const data = await res.json();' : 'console.log(res.status);'];
  return [
    { lang: 'curl', label: 'curl', code: curl.join(' \\\n  ') },
    { lang: 'fetch', label: 'JavaScript', code: lines.join('\n') },
  ];
}

function build(method: string, path: string, op: Operation): RestOp {
  const [title, rest] = splitSummary(op.summary ?? op.operationId ?? `${method.toUpperCase()} ${path}`);
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

  return {
    id: `rest-${op.operationId ?? `${method}-${path}`.replace(/[^a-z0-9]+/gi, '-')}`,
    method: method.toUpperCase(),
    path,
    tag: op.tags?.[0] ?? 'other',
    title,
    summaryRest: paragraphs(rest),
    description: paragraphs(op.description),
    signedIn: codes.includes('401'),
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
    samples: samplesFor(method, path, op, pathParams, queryParams, bodyType, redirect, Boolean(successBody)),
    responseStatus: success,
    responseType: successBody?.[0] ?? '',
    responseExample,
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

/** The operations grouped by tag, in the spec's tag order (untagged ones last). */
export function restGroups(): RestGroup[] {
  const ops = restOperations();
  const known = api.tags ?? [];
  const names = [...known.map((t) => t.name), ...ops.map((o) => o.tag)];
  return [...new Set(names)]
    .map((tag) => {
      const meta = known.find((t) => t.name === tag);
      return { tag, title: tag, description: meta?.description ?? '', ops: ops.filter((o) => o.tag === tag) };
    })
    .filter((g) => g.ops.length);
}
