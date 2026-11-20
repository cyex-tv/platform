import { error, redirect } from '@sveltejs/kit';
import { ApiError, gql } from '$lib/api';
import type { Platform } from '$lib/gql/graphql';
import { CHANNEL, PUBLIC_SET } from '$lib/queries';
import { channelMeta } from '$lib/meta';
import type { PageLoad } from './$types';

const PLATFORMS: Record<string, Platform> = { twitch: 'TWITCH', kick: 'KICK' };

export const load: PageLoad = async ({ fetch, params, url }) => {
  const platform = PLATFORMS[params.platform.toLowerCase()];
  if (!platform) error(404, 'cyex.tv has Twitch and Kick channels.');
  // Logins are case-insensitive: one address per channel.
  const key = params.platform.toLowerCase();
  const login = params.login.toLowerCase();
  if (key !== params.platform || login !== params.login) {
    redirect(308, `/channels/${key}/${encodeURIComponent(login)}${url.search}`);
  }
  try {
    const { channelByLogin } = await gql(CHANNEL, { platform, login: params.login }, fetch);
    if (!channelByLogin) error(404, `${params.login} isn't on cyex.tv yet.`);
    const set = channelByLogin.activeSetId
      ? (await gql(PUBLIC_SET, { id: channelByLogin.activeSetId }, fetch)).set
      : null;
    return { channel: channelByLogin, set, meta: channelMeta(channelByLogin, set ?? null) };
  } catch (e) {
    if (e instanceof ApiError) error(503, e.message);
    throw e;
  }
};
