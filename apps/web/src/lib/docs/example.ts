// Example values for the generated docs. One place, so the REST and GraphQL samples agree and
// every sample reads like a real request: ULIDs for cyex.tv IDs, a Twitch user ID for platform IDs.

export const EX = {
  twitchId: '12345678',
  login: 'lumen_fox',
  emoteId: '01JCYX4M8Q2N6R3T7V9W1Z5B0D',
  emoteName: 'foxwave',
  setId: '01JCYX2A7K5P8S1V4X6Z9C3E2G',
  accountId: '01JCYWZ9H3M6Q1T4W7Y0B2D5F8',
  editorId: '01JCYX0N2R5T8V1X4Z7C0E3G6J',
  roleId: '01JCYX6B3E6H9K2M5P8S1V4X7Z',
  reportId: '01JCYX7C4F7J0K3N6Q9T2W5Y8A',
  host: 'https://cdn.cyex.tv/emotes/9f2c4e7a1b3d5f8e0a2c4e6b8d0f1a3c5e7b9d1f3a5c7e9b0d2f4a6c8e0b2d4f',
  avatar: 'https://static-cdn.jtvnw.net/jtv_user_pictures/lumen_fox-profile_image-300x300.png',
  at: '2026-11-17T12:00:00Z',
} as const;

/**
 * A believable value for a field or parameter of this name. `ctx` is the type (or path) it sits
 * in, so an `id` inside an emote is an emote ID and an `id` next to a platform is a Twitch ID.
 */
export function exampleString(name: string, ctx = ''): string {
  const n = name.toLowerCase().replace(/_/g, '');
  const c = ctx.toLowerCase();
  if (n.endsWith('at') && n.length > 2 && n !== 'format') return EX.at;
  if (n === 'platformchannelid' || n === 'platformuserid') return EX.twitchId;
  if (n === 'userids') return EX.twitchId;
  if (n === 'id') {
    if (c.includes('personal') || c.includes('identity') || c.includes('platform')) return EX.twitchId;
    if (c.startsWith('emote')) return EX.emoteId;
    if (c.includes('channel')) return EX.twitchId;
    if (c.includes('set')) return EX.setId;
    if (c.includes('account') || c === 'me') return EX.accountId;
    if (c.includes('role')) return EX.roleId;
    if (c.includes('report')) return EX.reportId;
    return EX.emoteId;
  }
  if (n === 'setid' || n === 'activesetid') return EX.setId;
  if (n === 'emoteid') return EX.emoteId;
  if (n === 'accountid' || n === 'owner' || n === 'addedby' || n === 'actorid') return EX.accountId;
  if (n === 'roleid') return EX.roleId;
  if (n.endsWith('id')) return EX.emoteId;
  if (n === 'host') return EX.host;
  if (n === 'login') return EX.login;
  if (n === 'displayname') return EX.login;
  if (n === 'name') {
    if (c === 'setemote') return EX.emoteName;
    if (c.includes('set')) return `${EX.login}'s emotes`;
    if (c.includes('role')) return 'Moderator';
    if (c.includes('file')) return '2x.webp';
    return EX.emoteName;
  }
  if (n === 'query') return 'fox';
  if (n === 'reason') return 'Not an emote, a screenshot of a chat.';
  if (n === 'note') return 'Checked, it is fine.';
  if (n === 'status') return 'ok';
  if (n === 'error') return 'not_found';
  if (n === 'message') return 'No emote with that ID.';
  if (n === 'code') return 'kDq8t2Zr0mV5bX1c';
  if (n === 'state') return 'Wn3pR8sYq2';
  if (n === 'returnto') return 'https://cyex.tv/me';
  if (n === 'avatar') return EX.avatar;
  if (n === 'tags') return 'fox,wave';
  if (n === 'format') return 'webp';
  if (n === 'topic' || n === 'topics') return `channel:twitch:${EX.twitchId}`;
  if (n === 'kind') return 'emote.taken_down';
  if (n === 'payload') return `{"emote_id":"${EX.emoteId}"}`;
  if (n === 'permissions' || n === 'permission') return 'emotes.review';
  if (n === 'targetkind') return 'emote';
  if (n === 'before') return EX.reportId;
  return EX.emoteName;
}

/** A believable flag: an ordinary emote is none of nsfw, private, zero-width or static. */
export function exampleBool(name: string): boolean {
  const n = name.toLowerCase().replace(/_/g, '');
  return ['allowpersonalemotes', 'animated', 'listed', 'uploadsblocked', 'unreadonly'].includes(n);
}

/** Tags are a list of words; in the upload form they are one comma-separated string. */
export const EXAMPLE_TAGS = ['fox', 'wave'];

/** A believable number for a field of this name. */
export function exampleNumber(name: string): number {
  const n = name.toLowerCase();
  if (n === 'scale') return 2;
  if (n === 'size') return 4821;
  if (n === 'width' || n === 'height') return 56;
  if (n === 'capacity') return 1000;
  if (n === 'total') return 1;
  if (n === 'perpage' || n === 'per_page') return 20;
  if (n === 'limit') return 8;
  if (n === 'page') return 1;
  if (n.includes('count')) return 1284;
  return 1;
}

/** A run of text: plain, `code`, or a [link](href). */
export interface Segment {
  code: boolean;
  text: string;
  href?: string;
}

export function segments(text: string): Segment[] {
  const out: Segment[] = [];
  text.split('`').forEach((part, i) => {
    if (!part) return;
    if (i % 2 === 1) return out.push({ code: true, text: part });
    // Links only outside code: [label](href), where the label may hold `code`-free text.
    const re = /\[([^\]]+)\]\(([^)\s]+)\)/g;
    let last = 0;
    for (const m of part.matchAll(re)) {
      if (m.index > last) out.push({ code: false, text: part.slice(last, m.index) });
      out.push({ code: false, text: m[1], href: m[2] });
      last = m.index + m[0].length;
    }
    if (last < part.length) out.push({ code: false, text: part.slice(last) });
  });
  return out;
}

/** Paragraphs of a description: blank lines split them, single newlines are just wrapping. */
export function paragraphs(text: string | undefined | null): Segment[][] {
  if (!text) return [];
  return text
    .split(/\n\s*\n/)
    .map((p) => p.replace(/\s*\n\s*/g, ' ').trim())
    .filter(Boolean)
    .map(segments);
}

/** Single-quotes a string for a POSIX shell. */
export function shellQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

/** A JSON value written as Python. */
export function py(v: unknown, indent = 0): string {
  const pad = (k: number) => '    '.repeat(k);
  if (v === null || v === undefined) return 'None';
  if (v === true) return 'True';
  if (v === false) return 'False';
  if (typeof v === 'number') return String(v);
  if (typeof v === 'string') return JSON.stringify(v);
  if (Array.isArray(v)) {
    if (!v.length) return '[]';
    if (v.every((x) => typeof x !== 'object' || x === null)) return `[${v.map((x) => py(x)).join(', ')}]`;
    return `[\n${v.map((x) => pad(indent + 1) + py(x, indent + 1)).join(',\n')},\n${pad(indent)}]`;
  }
  const entries = Object.entries(v as Record<string, unknown>);
  if (!entries.length) return '{}';
  return `{\n${entries.map(([k, x]) => `${pad(indent + 1)}${JSON.stringify(k)}: ${py(x, indent + 1)}`).join(',\n')},\n${pad(indent)}}`;
}

/** Signed-in calls from a script carry the site session and the site's Origin (ADR 0009). */
export const SESSION_ENV = 'CYEX_TV_SESSION';
