import { describe, expect, it } from 'vitest';
import { COPY, causeOfCode, causeOfStatus, problemTitle, resolveCause } from './problem';

describe('the one error pattern', () => {
  it('maps API codes and HTTP statuses to four causes', () => {
    expect(causeOfCode('not_found')).toBe('missing');
    expect(causeOfCode('forbidden')).toBe('forbidden');
    expect(causeOfCode('offline')).toBe('server');
    expect(causeOfCode('bad_answer')).toBe('server');
    expect(causeOfCode(null)).toBe('server');
    expect(causeOfStatus(404)).toBe('missing');
    expect(causeOfStatus(403)).toBe('forbidden');
    expect(causeOfStatus(500)).toBe('server');
    expect(causeOfStatus(400)).toBe('server');
  });
  it('blames the network when the browser is offline', () => {
    expect(resolveCause('server', false)).toBe('offline');
    expect(resolveCause('server', true)).toBe('server');
    expect(resolveCause('missing', false)).toBe('missing');
  });
  it('offers Try again only where asking again can help', () => {
    expect(COPY.server.retry && COPY.offline.retry).toBe(true);
    expect(COPY.missing.retry || COPY.forbidden.retry).toBe(false);
  });
  it('titles the page by cause, with a not-found that can name what is missing', () => {
    expect(problemTitle('server')).toBe('Something went wrong - cyex.tv');
    expect(problemTitle('missing', true, 'No emote here')).toBe('No emote here - cyex.tv');
    expect(problemTitle('missing')).toBe('Nothing here - cyex.tv');
    expect(problemTitle('server', false)).toBe('You’re offline - cyex.tv');
  });
});
