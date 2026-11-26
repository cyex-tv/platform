// Mock API: GraphQL over GET with automatic persisted queries (ADR 0032), as the API answers them:
// a known hash runs its stored text, an unknown one is `PersistedQueryNotFound`, text with a wrong
// hash is `persisted_query_mismatch`, and only queries run over GET (`post_required`, 405). The
// cookie is never read over GET, so the request runs signed out.
import { createHash } from 'node:crypto';

const STORE = new Map();
const MAX = 5000;

const error = (status, message, code) => ({ status, body: { data: null, errors: [{ message, extensions: { code } }] } });

/** The SHA-256 of a query text, lowercase hex. */
export const hashOf = (text) => createHash('sha256').update(text).digest('hex');

/**
 * The GraphQL request a `GET /v1/gql` query string asks for, or the error answer.
 * @param {URLSearchParams} params
 * @returns {{ request: { query: string, variables?: object, operationName?: string } } | { status: number, body: unknown }}
 */
export function getRequest(params) {
  let query = params.get('query') ?? '';
  let variables;
  let extensions = {};
  try {
    if (params.get('variables')) variables = JSON.parse(params.get('variables'));
    if (params.get('extensions')) extensions = JSON.parse(params.get('extensions'));
  } catch {
    return error(400, '`variables` and `extensions` must be JSON.', 'bad_request');
  }
  const pq = extensions?.persistedQuery;
  if (pq) {
    const hash = pq.sha256Hash;
    if (pq.version !== 1 || typeof hash !== 'string' || !/^[0-9a-f]{64}$/.test(hash))
      return error(200, 'persistedQuery needs version 1 and a sha256Hash (64 lowercase hex digits).', 'bad_persisted_query');
    if (!query) {
      if (!STORE.has(hash)) return error(200, 'PersistedQueryNotFound', 'persisted_query_not_found');
      query = STORE.get(hash);
    } else if (hashOf(query) !== hash) {
      return error(200, 'sha256Hash isn’t the SHA-256 of the query sent with it.', 'persisted_query_mismatch');
    } else {
      if (STORE.size >= MAX) STORE.delete(STORE.keys().next().value);
      STORE.set(hash, query);
    }
  }
  if (/(^|[}\s])(mutation|subscription)\b/.test(query))
    return error(405, 'Over GET only queries run. Send mutations and subscriptions with POST.', 'post_required');
  return { request: { query, variables, operationName: params.get('operationName') ?? undefined } };
}
