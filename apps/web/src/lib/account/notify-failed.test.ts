import { describe, expect, it } from 'vitest';
import { describe as describeNote } from './notifications';

describe('the emote.failed notification', () => {
  it('says the emote could not be processed and opens its page', () => {
    const line = describeNote('emote.failed', JSON.stringify({ emote_id: '42', name: 'Pog' }));
    expect(line.text).toContain('couldn’t be processed');
    expect(line.href).toBe('/emotes/42');
  });
});
