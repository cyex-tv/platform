// The one error pattern. Every page that can't show what it was asked for says so in one of five
// ways, by cause: the server didn't answer, you're offline, it isn't there, it isn't yours to open,
// a payment didn't go through. Every surface that words a failed request (ApiError, upload and shop
// messages, inline notes) takes its words from here, so offline reads the same everywhere.
// The words live here and nowhere else; `Problem.svelte` draws them with State.svelte.

export type Cause = 'server' | 'offline' | 'missing' | 'forbidden' | 'payment';

export type Copy = { title: string; body: string; retry: boolean; heading: string };

export const COPY: Record<Cause, Copy> = {
  server: { title: 'cyex.tv didn’t answer.', body: 'It’s usually back within a minute.', retry: true, heading: 'Something went wrong' },
  offline: { title: 'You’re offline.', body: 'Check your connection, then try again.', retry: true, heading: 'You’re offline' },
  missing: { title: 'Nothing here.', body: 'It may have been removed, or the address is off.', retry: false, heading: 'Nothing here' },
  payment: { title: 'The payment didn’t go through.', body: 'Nothing was charged. Try again in a moment.', retry: true, heading: 'Payment failed' },
  forbidden: { title: 'You can’t open this.', body: 'Your account doesn’t have access to it.', retry: false, heading: 'No access' },
};

/** Whether the browser says it has a network. Server-side and in tests there is no navigator: online. */
export function isOnline(): boolean {
  return typeof navigator === 'undefined' ? true : navigator.onLine !== false;
}

/** The sentence for a request that never got an answer: "You're offline. ..." with no network, "cyex.tv didn't answer. ..." otherwise. */
export function unreachableMessage(online: boolean = isOnline()): string {
  const c = COPY[resolveCause('server', online)];
  return `${c.title} ${c.body}`;
}

/** The sentence for an answer the site couldn't use: the server's own words. */
export function serverMessage(): string {
  return `${COPY.server.title} ${COPY.server.body}`;
}

/** The cause an API error code stands for. Anything the server answered with but we can't place counts as the server. */
export function causeOfCode(code: string | null | undefined): Cause {
  if (code === 'not_found') return 'missing';
  if (code === 'forbidden') return 'forbidden';
  return 'server';
}

/** The cause an HTTP status stands for (the error pages). */
export function causeOfStatus(status: number): Cause {
  if (status === 404) return 'missing';
  if (status === 401 || status === 403) return 'forbidden';
  return 'server';
}

/** A request that failed while the browser says it has no network is the network's fault, not the server's. */
export function resolveCause(cause: Cause, online: boolean): Cause {
  return cause === 'server' && !online ? 'offline' : cause;
}

/** The document title for a page that is showing a problem. */
export function problemTitle(cause: Cause, online = true, missing?: string): string {
  const c = resolveCause(cause, online);
  return `${c === 'missing' && missing ? missing : COPY[c].heading} - cyex.tv`;
}

/** Moves focus to the page's h1 (or main), so a keyboard or screen reader user lands on what replaced the problem. */
export function focusPage(root: Document = document): void {
  const target = root.querySelector<HTMLElement>('main h1') ?? root.getElementById('main');
  if (!target) return;
  if (!target.hasAttribute('tabindex')) target.setAttribute('tabindex', '-1');
  target.focus();
}
