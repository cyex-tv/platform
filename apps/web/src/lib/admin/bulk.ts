// The admin pages' bulk parts, kept pure so they are unit tested: what a batch verdict or a batch
// dismissal says when it comes back, and how a pasted list of logins is read.
import { count } from '$lib/format';

/** The most items one batch takes (the API's limit, as the browse grid's selection cap). */
export const BATCH_MAX = 100;

export type BatchOutcome = { done: readonly string[]; failed: readonly { id: string; code: string; message: string }[] };

/**
 * One sentence for a batch that came back: how many it did (`verb` is past tense, "approved"),
 * then how many it left alone and why, in words.
 */
export function batchSummary(outcome: BatchOutcome, verb: string, noun: [string, string] = ['upload', 'uploads']): string {
  const n = outcome.done.length;
  const head = n === 0 ? `Nothing ${verb}.` : `${count(n)} ${n === 1 ? noun[0] : noun[1]} ${verb}.`;
  if (!outcome.failed.length) return head;
  const by = new Map<string, number>();
  for (const f of outcome.failed) by.set(f.code, (by.get(f.code) ?? 0) + 1);
  const parts = [...by].map(([code, k]) => {
    switch (code) {
      case 'not_pending':
        return `${count(k)} ${k === 1 ? 'was' : 'were'} already reviewed`;
      case 'not_open':
        return `${count(k)} ${k === 1 ? 'was' : 'were'} already closed`;
      case 'not_found':
        return `${count(k)} ${k === 1 ? 'is' : 'are'} gone`;
      default:
        return `${count(k)} didn’t go through`;
    }
  });
  return `${head} ${sentence(parts)}.`;
}

function sentence(parts: string[]): string {
  const s = parts.length > 1 ? `${parts.slice(0, -1).join(', ')} and ${parts[parts.length - 1]}` : parts[0];
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/**
 * A pasted list of logins: one per line, or separated by commas, semicolons or spaces. Trimmed,
 * repeats once (ignoring case and a leading @), in the order pasted. Over `max`, the rest is
 * counted in `extra` and left out.
 */
export function parseLogins(text: string, max = BATCH_MAX): { logins: string[]; extra: number } {
  const seen = new Set<string>();
  const all: string[] = [];
  for (const raw of text.split(/[\s,;]+/)) {
    const login = raw.trim();
    if (!login) continue;
    const key = login.replace(/^@/, '').toLowerCase();
    if (seen.has(key)) continue;
    seen.add(key);
    all.push(login);
  }
  return { logins: all.slice(0, max), extra: Math.max(0, all.length - max) };
}

/** Why a pasted login has no account, in words. */
export function loginProblem(problem: string | null | undefined): string {
  switch (problem) {
    case 'not_found':
      return 'No account with that login';
    case 'ambiguous':
      return 'Two accounts: write twitch:name or kick:name';
    case 'bad_login':
      return 'Not a login';
    default:
      return problem ? 'Not found' : '';
  }
}
