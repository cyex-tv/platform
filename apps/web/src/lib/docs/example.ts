// Example values for the generated docs. One place, so the REST and GraphQL samples agree.

/** A believable string for a field or parameter of this name. */
export function exampleString(name: string): string {
  const n = name.toLowerCase();
  if (n.endsWith('at') && n.length > 2) return '2026-11-17T12:00:00Z';
  if (n === 'platformchannelid' || n === 'platform_channel_id' || n === 'platformuserid' || n === 'platform_user_id') return '12345678';
  if (n === 'id' || n.endsWith('id') || n.endsWith('_id')) return '123';
  if (n === 'host') return 'https://cdn.cyex.tv/emotes/3f2a9c';
  if (n === 'login') return 'cyex';
  if (n === 'name' || n === 'query') return 'pog';
  if (n === 'displayname' || n === 'display_name') return 'cyex';
  if (n === 'reason' || n === 'note') return 'Spam';
  if (n === 'status') return 'ok';
  if (n === 'error') return 'not_found';
  if (n === 'message') return 'No such thing.';
  if (n === 'code') return 'abc123';
  if (n === 'state') return 'xyz789';
  if (n === 'return_to') return 'https://cyex.tv/me';
  if (n === 'avatar') return 'https://cdn.cyex.tv/avatars/12345678.webp';
  if (n === 'tags') return 'funny';
  if (n === 'format') return 'webp';
  if (n === 'topic') return 'channel:twitch:12345678';
  return 'example';
}

/** A run of text, either plain or `code`. */
export interface Segment {
  code: boolean;
  text: string;
}

export function segments(text: string): Segment[] {
  return text
    .split('`')
    .map((part, i): Segment => ({ code: i % 2 === 1, text: part }))
    .filter((s) => s.text !== '');
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
