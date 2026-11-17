import { describe, expect, it } from 'vitest';
import { load } from './+page';

const run = (platform: string, login: string, search = '') =>
  load({ fetch, params: { platform, login }, url: new URL(`http://x/channels/${platform}/${login}${search}`) } as never);

describe('channel address', () => {
  it('redirects a mixed-case address to the lowercase one, keeping the query', async () => {
    await expect(run('Twitch', 'LumenFox', '?a=1')).rejects.toMatchObject({ status: 308, location: '/channels/twitch/lumenfox?a=1' });
  });
  it('404s an unknown platform', async () => {
    await expect(run('youtube', 'x')).rejects.toMatchObject({ status: 404 });
  });
});
