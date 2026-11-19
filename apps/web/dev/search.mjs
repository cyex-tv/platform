// The mock's search matching, kept apart from the server so a test can load it. It follows the
// real API (crates/db/src/search.rs): the same tiers, and pg_trgm's `%` for near misses (trigrams
// of the lowercased text, padded with two spaces in front and one behind, similarity >= 0.3).

export const nameWords = (n) => n.replace(/([a-z0-9])([A-Z])/g, '$1 $2').replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
  .replace(/([A-Za-z])([0-9])/g, '$1 $2').replace(/([0-9])([A-Za-z])/g, '$1 $2').toLowerCase().split(/[\s_.\-]+/).filter(Boolean);

// pg_trgm works per word: each run of letters or digits is padded and cut into trigrams.
const grams = (t) => {
  const g = new Set();
  for (const w of t.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter(Boolean)) {
    const p = `  ${w} `;
    for (let i = 0; i + 3 <= p.length; i++) g.add(p.slice(i, i + 3));
  }
  return g;
};

/** pg_trgm similarity(a, b): shared trigrams over all trigrams, 0 to 1. */
export const similar = (a, b) => {
  const x = grams(a);
  const y = grams(b);
  let n = 0;
  for (const g of x) if (y.has(g)) n++;
  return x.size + y.size - n === 0 ? 0 : n / (x.size + y.size - n);
};

/** The `%` operator: similar enough to count as a near miss. */
export const near = (a, b) => similar(a, b) >= 0.3;

/** 1 exact, 2 same ignoring case, 3 prefix, 4 word or initials prefix, 5 substring, 6 near miss (3+ letters), 0 none. */
export function quickTier(name, q) {
  const l = name.toLowerCase();
  const ql = q.toLowerCase();
  if (name === q) return 1;
  if (l === ql) return 2;
  if (l.startsWith(ql)) return 3;
  const words = nameWords(name);
  if (words.some((w) => w.startsWith(ql)) || words.map((w) => w[0]).join('').startsWith(ql)) return 4;
  if (l.includes(ql)) return 5;
  if ([...ql].length >= 3 && near(name, q)) return 6;
  return 0;
}

/**
 * How well a person matches `text` (lowercase, trimmed, no @): the best over every linked login and
 * display name. 3 exact login, 2 login prefix, 1 display name prefix, 0.5 contained or near miss,
 * -1 no match. `score` is the best similarity, for ordering within a tier.
 */
export function personMatch(identities, text) {
  let tier = -1;
  let score = 0;
  for (const i of identities) {
    const login = i.login.toLowerCase();
    const display = i.displayName.toLowerCase();
    const t = login === text ? 3 : login.startsWith(text) ? 2 : display.startsWith(text) ? 1
      : login.includes(text) || display.includes(text) || near(login, text) || near(display, text) ? 0.5 : -1;
    tier = Math.max(tier, t);
    score = Math.max(score, similar(login, text), similar(display, text));
  }
  return { tier, score };
}
