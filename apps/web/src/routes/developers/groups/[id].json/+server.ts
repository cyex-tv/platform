import { error, json } from '@sveltejs/kit';
import { groupPayload, lazyGroupIds } from '$lib/docs/lazy.server';
import type { EntryGenerator, RequestHandler } from './$types';

// One closed /developers group's body, built at build time like the page.
export const prerender = true;

export const entries: EntryGenerator = () => lazyGroupIds().map((id) => ({ id }));

export const GET: RequestHandler = ({ params }) => {
  const payload = groupPayload(params.id);
  if (!payload) error(404, 'No such group');
  return json(payload);
};
