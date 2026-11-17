// The GraphQL reference, generated from schema/schema.graphql when the site is built.
import {
  buildSchema,
  getNamedType,
  isEnumType,
  isInputObjectType,
  isListType,
  isNonNullType,
  isObjectType,
  isScalarType,
  type GraphQLField,
  type GraphQLInputType,
  type GraphQLNamedType,
  type GraphQLOutputType,
  type GraphQLSchema,
} from 'graphql';
import sdl from '../../../../../schema/schema.graphql?raw';
import { exampleString, paragraphs, shellQuote, type Segment } from './example';
import { baseUrl } from './rest';

export type OpKind = 'query' | 'mutation' | 'subscription';

export interface GqlArg {
  name: string;
  type: string;
  required: boolean;
  description: Segment[][];
  default?: string;
}
export interface GqlOp {
  id: string;
  kind: OpKind;
  name: string;
  description: Segment[][];
  args: GqlArg[];
  returns: string;
  deprecated?: string;
  /** The document, pretty printed. */
  document: string;
  variables: string;
  samples: { lang: string; label: string; code: string }[];
  response: string;
}
export interface GqlMember {
  name: string;
  type: string;
  description: Segment[][];
  required: boolean;
}
export interface GqlType {
  id: string;
  name: string;
  kind: 'object' | 'input' | 'enum';
  description: Segment[][];
  members: GqlMember[];
}
export interface GqlReference {
  endpoint: string;
  socket: string;
  queries: GqlOp[];
  mutations: GqlOp[];
  subscriptions: GqlOp[];
  types: GqlType[];
}

const SCALARS = new Set(['String', 'Int', 'Float', 'Boolean', 'ID']);
export const typeId = (name: string) => `gql-type-${name}`;

function inputExample(type: GraphQLInputType, name: string, seen: string[] = []): unknown {
  if (isNonNullType(type)) return inputExample(type.ofType, name, seen);
  if (isListType(type)) return [inputExample(type.ofType, name, seen)];
  if (isEnumType(type)) return type.getValues()[0].name;
  if (isInputObjectType(type)) {
    if (seen.includes(type.name)) return {};
    const fields = Object.values(type.getFields());
    const picked = fields.filter((f) => isNonNullType(f.type));
    const use = picked.length ? picked : fields.slice(0, 1);
    return Object.fromEntries(use.map((f) => [f.name, inputExample(f.type, f.name, [...seen, type.name])]));
  }
  switch (type.name) {
    case 'Int':
    case 'Float':
      return name === 'perPage' ? 20 : 1;
    case 'Boolean':
      return true;
    default:
      return exampleString(name);
  }
}

/** A selection set and the matching data, from one walk: scalars always, objects two levels deep. */
function select(type: GraphQLOutputType, name: string, depth: number, indent: string): { text: string; data: unknown } {
  if (isNonNullType(type)) return select(type.ofType, name, depth, indent);
  if (isListType(type)) {
    const inner = select(type.ofType, name, depth, indent);
    return { text: inner.text, data: [inner.data] };
  }
  if (isEnumType(type)) return { text: '', data: type.getValues()[0].name };
  if (isScalarType(type)) {
    if (type.name === 'Boolean') return { text: '', data: true };
    if (type.name === 'Int' || type.name === 'Float') return { text: '', data: name === 'total' ? 1 : name === 'capacity' ? 1000 : 1 };
    return { text: '', data: exampleString(name) };
  }
  if (!isObjectType(type)) return { text: '', data: null };
  const lines: string[] = [];
  const data: Record<string, unknown> = {};
  for (const f of Object.values(type.getFields())) {
    const named = getNamedType(f.type);
    const leaf = isScalarType(named) || isEnumType(named);
    if (!leaf && depth >= 2) continue;
    const inner = select(f.type, f.name, depth + 1, indent + '  ');
    data[f.name] = inner.data;
    lines.push(leaf ? `${indent}  ${f.name}` : `${indent}  ${f.name} {\n${inner.text}\n${indent}  }`);
  }
  return { text: lines.join('\n'), data };
}

function documentFor(kind: OpKind, field: GraphQLField<unknown, unknown>) {
  const vars = field.args.map((a) => `$${a.name}: ${a.type}`).join(', ');
  const use = field.args.map((a) => `${a.name}: $${a.name}`).join(', ');
  const named = getNamedType(field.type);
  const sel = isObjectType(named) ? select(field.type, field.name, 1, '  ') : { text: '', data: select(field.type, field.name, 1, '').data };
  const label = field.name[0].toUpperCase() + field.name.slice(1);
  const head = `${kind} ${label}${vars ? `(${vars})` : ''}`;
  const call = `${field.name}${use ? `(${use})` : ''}`;
  const document = isObjectType(named) ? `${head} {\n  ${call} {\n${sel.text}\n  }\n}` : `${head} {\n  ${call}\n}`;
  const variables = Object.fromEntries(field.args.map((a) => [a.name, inputExample(a.type, a.name)]));
  return { document, variables, data: { [field.name]: sel.data } };
}

function opFrom(kind: OpKind, field: GraphQLField<unknown, unknown>): GqlOp {
  const { document, variables, data } = documentFor(kind, field);
  const hasVars = field.args.length > 0;
  const oneLine = document.replace(/\s+/g, ' ');
  const body = hasVars ? { query: oneLine, variables } : { query: oneLine };
  const endpoint = `${baseUrl}/v1/gql`;
  const curl = `curl -s ${shellQuote(endpoint)} \\\n  -H 'content-type: application/json' \\\n  -d ${shellQuote(JSON.stringify(body))}`;
  const fetchCode =
    `const res = await fetch('${endpoint}', {\n  method: 'POST',\n  headers: { 'content-type': 'application/json' },\n  credentials: 'include',\n  body: JSON.stringify({\n    query: \`${document.replace(/\n/g, '\n    ')}\`,` +
    (hasVars ? `\n    variables: ${JSON.stringify(variables, null, 2).replace(/\n/g, '\n    ')},` : '') +
    `\n  }),\n});\nconst { data, errors } = await res.json();`;
  return {
    id: `gql-${kind}-${field.name}`,
    kind,
    name: field.name,
    description: paragraphs(field.description),
    args: field.args.map((a) => ({
      name: a.name,
      type: String(a.type),
      required: isNonNullType(a.type) && a.defaultValue === undefined,
      description: paragraphs(a.description),
      default: a.defaultValue === undefined ? undefined : JSON.stringify(a.defaultValue),
    })),
    returns: String(field.type),
    deprecated: field.deprecationReason ?? undefined,
    document,
    variables: hasVars ? JSON.stringify(variables, null, 2) : '',
    samples:
      kind === 'subscription'
        ? []
        : [
            { lang: 'curl', label: 'curl', code: curl },
            { lang: 'fetch', label: 'JavaScript', code: fetchCode },
          ],
    response: JSON.stringify({ data }, null, 2),
  };
}

/** Every named type the operations touch, directly or through other types. */
function reachable(schema: GraphQLSchema, roots: GraphQLField<unknown, unknown>[]): GraphQLNamedType[] {
  const seen = new Map<string, GraphQLNamedType>();
  const visit = (t: GraphQLNamedType) => {
    if (SCALARS.has(t.name) || seen.has(t.name) || t.name.startsWith('__')) return;
    if (isScalarType(t)) return;
    seen.set(t.name, t);
    if (isObjectType(t) || isInputObjectType(t)) {
      for (const f of Object.values(t.getFields())) {
        visit(getNamedType(f.type)!);
        if ('args' in f) for (const a of f.args) visit(getNamedType(a.type)!);
      }
    }
  };
  for (const r of roots) {
    visit(getNamedType(r.type) as GraphQLNamedType);
    for (const a of r.args) visit(getNamedType(a.type) as GraphQLNamedType);
  }
  void schema;
  return [...seen.values()].sort((a, b) => a.name.localeCompare(b.name));
}

function typeFrom(t: GraphQLNamedType): GqlType {
  const base = { id: typeId(t.name), name: t.name, description: paragraphs(t.description) };
  if (isEnumType(t)) {
    return {
      ...base,
      kind: 'enum',
      members: t.getValues().map((v) => ({ name: v.name, type: '', description: paragraphs(v.description), required: false })),
    };
  }
  const input = isInputObjectType(t);
  const fields = isObjectType(t) || input ? Object.values(t.getFields()) : [];
  return {
    ...base,
    kind: input ? 'input' : 'object',
    members: fields.map((f) => ({
      name: f.name,
      type: String(f.type),
      description: paragraphs(f.description),
      required: input && isNonNullType(f.type),
    })),
  };
}

/** The whole GraphQL reference. */
export function graphqlReference(): GqlReference {
  const schema = buildSchema(sdl);
  const fieldsOf = (root: ReturnType<GraphQLSchema['getQueryType']>) => (root ? Object.values(root.getFields()) : []);
  const q = fieldsOf(schema.getQueryType());
  const m = fieldsOf(schema.getMutationType());
  const s = fieldsOf(schema.getSubscriptionType());
  return {
    endpoint: `${baseUrl}/v1/gql`,
    socket: `${baseUrl.replace(/^http/, 'ws')}/v1/gql/ws`,
    queries: q.map((f) => opFrom('query', f)),
    mutations: m.map((f) => opFrom('mutation', f)),
    subscriptions: s.map((f) => opFrom('subscription', f)),
    types: reachable(schema, [...q, ...m, ...s]).map(typeFrom),
  };
}
