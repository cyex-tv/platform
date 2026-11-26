// The build plugin that keeps GraphQL documents per page (dev/vite-gql-documents.ts): every
// graphql() call in the query files maps to its generated document, and none is left to the
// all-documents lookup.
import { readdirSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { documentExport, generatedDocuments, markPure, rewriteQueries } from '../../dev/vite-gql-documents';

const root = new URL('.', import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, '$1');
const docs = generatedDocuments(readFileSync(`${root}gql/graphql.ts`, 'utf8'));
const files = [...readdirSync(`${root}queries`).filter((f) => f.endsWith('.ts') && !f.endsWith('.test.ts')).map((f) => `${root}queries/${f}`), `${root}paint-studio/queries.ts`];

describe('gql documents plugin', () => {
  it('names the generated export of an operation and of a fragment', () => {
    expect(documentExport('\n  query MyNotifications {\n x }\n')).toBe('MyNotificationsDocument');
    expect(documentExport('\n  fragment EmoteFields on Emote { id }')).toBe('EmoteFieldsFragmentDoc');
  });

  it('finds every generated document', () => {
    expect(docs.size).toBeGreaterThan(150);
  });

  it('rewrites every graphql() call in the query files, leaving none', () => {
    let calls = 0;
    for (const f of files) {
      const src = readFileSync(f, 'utf8');
      const out = rewriteQueries(src, docs);
      if (!out) continue;
      calls += (src.match(/\bgraphql\(`/g) ?? []).length;
      expect(out, f).not.toMatch(/\bgraphql\(/);
      expect(out, f).toContain("import { TypedDocumentString } from '$lib/gql/graphql'");
    }
    expect(calls).toBe(docs.size);
  });

  it('marks the generated constructions pure', () => {
    expect(markPure('export const A = new TypedDocumentString(`x`)')).toContain('= /*#__PURE__*/ new TypedDocumentString(');
  });
});
