// Build plugin: keeps the generated GraphQL documents out of the shared bundle.
//
// The client preset's `graphql(...)` looks a source string up in one `documents` map, and the
// generated documents all live in one module, so every page ends up loading all 179 of them. This
// replaces each such call in the query files with the generated document's own text, built right
// there with `new TypedDocumentString(...)`, so a page only carries the documents it uses. The
// generated files stay as `npm run codegen` writes them (types still come from them), and their
// `new TypedDocumentString(` calls are marked pure so unused ones drop out.
import { readFileSync } from 'node:fs';
import type { Plugin } from 'vite';

const QUERY_FILE = /[\\/]src[\\/]lib[\\/](queries[\\/][^\\/]+\.ts|paint-studio[\\/]queries\.ts)$/;
const GENERATED = /[\\/]src[\\/]lib[\\/]gql[\\/]graphql\.ts$/;
const CALL = /\bgraphql\(\s*`((?:[^`$]|\$(?!\{))*)`\s*\)/g;
const GRAPHQL_IMPORT = /^import \{ graphql \} from '(?:\$lib\/gql|\.\.\/gql)';\r?\n/m;
const HEAD = /^\s*(query|mutation|subscription|fragment)\s+([_A-Za-z][_0-9A-Za-z]*)/;
const GENERATED_DOC = /^export const (\w+) = new TypedDocumentString\((`[^`]*`(?:,\s*\{[^}]*\})?)\)/gm;

/** The generated export a source string maps to. */
export function documentExport(source: string) {
  const m = HEAD.exec(source);
  if (!m) throw new Error('gql-documents: a graphql() call is not a named operation or fragment');
  return m[1] === 'fragment' ? `${m[2]}FragmentDoc` : `${m[2]}Document`;
}

/** Export name -> constructor arguments (text and meta) of every generated document. */
export function generatedDocuments(generatedSource: string) {
  return new Map<string, string>([...generatedSource.matchAll(GENERATED_DOC)].map((m): [string, string] => [m[1], m[2]]));
}

/** Rewrites one query file; returns null when it has no graphql() call. */
export function rewriteQueries(code: string, docs: Map<string, string>) {
  let count = 0;
  const out = code.replace(CALL, (_all: string, source: string) => {
    const name = documentExport(source);
    const args = docs.get(name);
    if (args === undefined) throw new Error(`gql-documents: ${name} is not in gql/graphql.ts (run npm run codegen)`);
    count++;
    return `/*#__PURE__*/ new TypedDocumentString(${args})`;
  });
  if (count === 0) return null;
  return `import { TypedDocumentString } from '$lib/gql/graphql';\n${out.replace(GRAPHQL_IMPORT, '')}`;
}

export function markPure(code: string) {
  return code.replace(/= new TypedDocumentString\(/g, '= /*#__PURE__*/ new TypedDocumentString(');
}

export function gqlDocuments(): Plugin {
  let docs: Map<string, string> | null = null;
  return {
    name: 'cyex-gql-documents',
    enforce: 'pre',
    transform(code: string, id: string) {
      const path = id.split('?')[0];
      if (GENERATED.test(path)) return { code: markPure(code), map: null };
      if (!QUERY_FILE.test(path)) return null;
      if (!docs) docs = generatedDocuments(readFileSync(path.replace(/src[\\/]lib[\\/].*$/, 'src/lib/gql/graphql.ts'), 'utf8'));
      const out = rewriteQueries(code, docs);
      return out ? { code: out, map: null } : null;
    },
  };
}
