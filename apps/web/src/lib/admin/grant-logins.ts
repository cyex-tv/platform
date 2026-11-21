// The pasted list on Give to many, read as you type: how many entries look like a login (so the
// count says "12 logins" and the odd ones out are named before Check), and how many are over the
// batch size. Whether an account exists is Check's job.
import { BATCH_MAX, parseLogins } from './bulk';

const SHAPE = /^(?:(?:twitch|kick):)?@?[\w.-]{1,40}$/i;

export type LoginCount = { logins: string[]; valid: number; odd: string[]; extra: number };

export function countLogins(text: string, max = BATCH_MAX): LoginCount {
  const { logins, extra } = parseLogins(text, max);
  const odd = logins.filter((l) => !SHAPE.test(l));
  return { logins, valid: logins.length - odd.length, odd, extra };
}
