// Mock API: directory. See ../mock-api.mjs.
import { ME_SUMMARY, OTHERS } from './config.mjs';
import { ALL, trendAdds } from './emotes.mjs';
import { CHANNELS } from './account.mjs';
import { SETS } from './set-data.mjs';

// A public account for `account(id)`. Unknown ids, and the one for a blocked account, give null.
const FAKE_LOGINS = ['quietoak', 'nora_byte', 'lumen_fox', 'pixelmoth', 'sipteaTV', 'byteNod', 'foxlook', 'moonhi'];
export const publicAccount = (id) => {
  const known = [ME_SUMMARY, ...OTHERS].find((a) => a.id === id);
  if (known) return known;
  const m = /^01JD8XACC(\d{17})$/.exec(id);
  return m ? { id, displayName: FAKE_LOGINS[Number(m[1]) % FAKE_LOGINS.length], avatar: null } : null;
};
// Channels showing an emote: as many as its channelCount says, newest first, from the mock owners.
export const emoteChannels = (e, limit, offset = 0) => {
  const n = Math.max(0, Math.min(limit, e.channelCount - offset));
  return Array.from({ length: n }, (_, i) => {
    const k = i + offset;
    const owner = [...OTHERS, ME_SUMMARY][k % 3];
    const login = k < 3 ? owner.displayName : `${FAKE_LOGINS[k % FAKE_LOGINS.length]}${k}`;
    return {
      platform: k % 4 === 3 ? 'KICK' : 'TWITCH',
      platformChannelId: String(7000 + Number(e.id.slice(-4)) * 100 + k),
      login,
      account: k < 3 ? owner : { id: `01JD8XACC${String(k).padStart(17, '0')}`, displayName: login, avatar: null },
      activeSetId: SETS[k % SETS.length].id,
      allowPersonalEmotes: true,
    };
  });
};
// Public channel lookup: the mock account's own channel, plus every channel emoteChannels() can
// return, each with an active set, so a link from "used in" always lands on a channel page.
const channelIndex = new Map();
export const channelByLogin = (platform, login) => {
  if (typeof login !== 'string') return null;
  if (!channelIndex.size) {
    for (const e of ALL)
      for (const c of emoteChannels(e, e.channelCount)) {
        const key = `${c.platform}/${c.login.toLowerCase()}`;
        if (!channelIndex.has(key)) channelIndex.set(key, c);
      }
  }
  if (login.toLowerCase() === 'lumen_fox') return { ...CHANNELS[0], platform };
  return channelIndex.get(`${platform}/${login.toLowerCase()}`) ?? null;
};
export const sorted = (hits, sort, window = 'WEEK') => {
  if (sort === 'TRENDING')
    return hits
      .filter((e) => trendAdds(e, window) > 0)
      .sort((a, b) => trendAdds(b, window) - trendAdds(a, window) || b.channelCount - a.channelCount || a.id.localeCompare(b.id));
  if (sort === 'TOP') return [...hits].sort((a, b) => b.channelCount - a.channelCount || a.name.localeCompare(b.name));
  if (sort === 'NEW') return [...hits].sort((a, b) => b.createdAt.localeCompare(a.createdAt));
  return hits;
};
