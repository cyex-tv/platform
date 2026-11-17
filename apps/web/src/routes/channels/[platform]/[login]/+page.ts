import { error } from '@sveltejs/kit';
import { ApiError, gql } from '$lib/api';
import type { Platform } from '$lib/gql/graphql';
import { CHANNEL, SET } from '$lib/queries';
import type { PageLoad } from './$types';

const PLATFORMS: Record<string, Platform> = { twitch: 'TWITCH', kick: 'KICK' };

export const load: PageLoad = async ({ fetch, params }) => {
  const platform = PLATFORMS[params.platform];
  if (!platform) error(404, 'cyex.tv has Twitch and Kick channels.');
  try {
    const { channelByLogin } = await gql(CHANNEL, { platform, login: params.login }, fetch);
    if (!channelByLogin) error(404, `${params.login} isn't on cyex.tv yet.`);
    const set = channelByLogin.activeSetId
      ? (await gql(SET, { id: channelByLogin.activeSetId }, fetch)).set
      : null;
    return { channel: channelByLogin, set };
  } catch (e) {
    if (e instanceof ApiError) error(503, e.message);
    throw e;
  }
};
