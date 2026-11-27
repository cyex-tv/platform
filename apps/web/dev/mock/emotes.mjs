// Mock API: emotes. See ../mock-api.mjs.
import { BASE, FACES, ME_SUMMARY, OTHERS } from './config.mjs';
import { NAME_OF, nameFor, rngFor, spec } from './art.mjs';

// ---- Data -----------------------------------------------------------------------------
export const NOW = Date.now();
export const DAY = 86400_000;

// The two approvals of an emote (ADR 0038), as its review state gives them: approved both, pending
// both, rejected and taken down both denied. Seeds below change a few (personal use pending or denied).
export const approvalsOf = (state) => {
  const v = state === 'APPROVED' ? 'APPROVED' : state === 'PENDING' ? 'PENDING' : 'DENIED';
  return { publicListing: v, personalUse: v };
};

export const emote = (i, owner = OTHERS[i % OTHERS.length], state = 'APPROVED') => {
  const s = spec(i);
  const w = s.wide ? 64 : 32;
  const idx = i < 300 ? i : i % 50;
  return {
    id: `01JD8X${String(i).padStart(20, '0')}`,
    name: nameFor(i),
    state,
    approvals: approvalsOf(state),
    processing: 'READY',
    animated: s.animated,
    tags: [...s.tags],
    host: `${BASE}/emotes/${i}`,
    createdAt: new Date(NOW - idx * 0.2 * DAY - Math.floor(rngFor(i, 3)() * 0.18 * DAY)).toISOString(),
    flags: { zeroWidth: s.zero, nsfw: false, private: false, wide: s.wide },
    visibility: 'PUBLIC',
    // Made-up but stable: a few popular ones, a long tail, some in no channel.
    channelCount: i % 11 === 0 ? 0 : (i * 37) % 91 + (i % 5 === 0 ? 400 : 0),
    owner,
    // As the worker writes them (ADR 0012): still emotes WebP and AVIF; animated ones moving WebP
    // plus still WebP and AVIF (no animated AVIF).
    files: [1, 2, 3, 4].flatMap((k) => [
      { name: `${k}x.webp`, format: 'WEBP', scale: k, width: w * k, height: 32 * k, size: 900 * k, isStatic: false },
      ...(s.animated
        ? [
            { name: `${k}x_static.webp`, format: 'WEBP', scale: k, width: w * k, height: 32 * k, size: 400 * k, isStatic: true },
            { name: `${k}x_static.avif`, format: 'AVIF', scale: k, width: w * k, height: 32 * k, size: 300 * k, isStatic: true },
          ]
        : [{ name: `${k}x.avif`, format: 'AVIF', scale: k, width: w * k, height: 32 * k, size: 600 * k, isStatic: false }]),
    ]),
  };
};

// ---- Twitch global emotes -------------------------------------------------------------
// Names and ids were read from the global-emote listing at https://twitchemotes.com/ and each id was
// checked against https://static-cdn.jtvnw.net/emoticons/v2/<id>/default/dark/{1.0,2.0,3.0} (200 and an
// image). The images belong to Twitch and are only ever referenced by URL: /twitch/<n>/<k>x[_static].webp
// on this mock redirects to the CDN, so the site's `host + /1x.webp` URLs work unchanged. On that CDN
// 1.0 / 2.0 / 3.0 are 1x / 2x / 4x of the same emote (a 28 px emote is 28, 56 and 112 px); there is no 3x.
// Animated ones use the `animated` format (GIF) and `static` for the first frame.
// [name, CDN id, width at 1x, height at 1x, tags, 'a' if animated]
const TWITCH = [
  ['4Head', '354', 20, 30, 'laugh,meme'],
  ['ANELE', '3792', 28, 28, 'happy,meme'],
  ['ArsonNoSexy', '50', 18, 27, 'meme,fire'],
  ['BatChest', '115234', 28, 28, 'hype,meme'],
  ['BCWarrior', '30', 29, 27, 'hype,meme'],
  ['BigBrother', '1904', 24, 30, 'meme,face'],
  ['BloodTrail', '69', 41, 28, 'hype,meme'],
  ['BrainSlug', '115233', 28, 28, 'meme,weird'],
  ['BrokeBack', '4057', 28, 28, 'sad,meme'],
  ['CoolStoryBob', '123171', 28, 28, 'meme,sarcasm'],
  ['CorgiDerp', '49106', 27, 28, 'dog,cute'],
  ['CurseLit', '116625', 28, 28, 'hype,fire'],
  ['DendiFace', '58135', 28, 28, 'face,meme'],
  ['DogFace', '114835', 28, 28, 'dog,meme'],
  ['DoritosChip', '102242', 28, 28, 'meme,food'],
  ['FailFish', '360', 22, 30, 'sad,meme'],
  ['FrankerZ', '65', 40, 30, 'dog,happy'],
  ['FreakinStinkin', '117701', 28, 28, 'meme,face'],
  ['GrammarKing', '3632', 28, 28, 'meme,face'],
  ['HassaanChop', '20225', 28, 28, 'meme,hype'],
  ['HeyGuys', '30259', 28, 28, 'wave,happy'],
  ['Jebaited', '114836', 28, 28, 'meme,troll'],
  ['JonCarnage', '26', 20, 27, 'meme,face'],
  ['KAPOW', '133537', 28, 28, 'hype,meme'],
  ['KappaPride', '55338', 28, 28, 'pride,meme'],
  ['KappaRoss', '70433', 28, 28, 'meme,art'],
  ['KappaWealth', '81997', 28, 28, 'meme,money'],
  ['Kippa', '1901', 24, 28, 'meme,face'],
  ['KomodoHype', '81273', 28, 28, 'hype,happy'],
  ['Kreygasm', '41', 19, 27, 'hype,pog'],
  ['MrDestructoid', '28', 39, 27, 'robot,meme'],
  ['NinjaGrumpy', '138325', 28, 28, 'meme,mad'],
  ['NotLikeThis', '58765', 28, 28, 'sad,meme'],
  ['OneHand', '66', 20, 27, 'meme,hand'],
  ['OptimizePrime', '16', 22, 27, 'robot,meme'],
  ['PJSalt', '36', 36, 30, 'mad,meme'],
  ['PartyTime', '135393', 28, 28, 'happy,hype'],
  ['PeoplesChamp', '3412', 28, 28, 'hype,pog'],
  ['PermaSmug', '27509', 28, 28, 'smug,meme'],
  ['PogChamp', '305954156', 28, 28, 'pog,hype'],
  ['Poooound', '117484', 28, 28, 'hype,meme'],
  ['PopCorn', '724216', 28, 28, 'food,meme'],
  ['RaccAttack', '114870', 28, 28, 'cute,hype'],
  ['RalpherZ', '1900', 33, 30, 'dog,happy'],
  ['RedCoat', '22', 19, 27, 'meme,face'],
  ['RlyTho', '134256', 28, 28, 'meme,sarcasm'],
  ['RuleFive', '107030', 28, 28, 'meme,face'],
  ['SMOrc', '52', 32, 32, 'hype,meme'],
  ['SeriousSloth', '81249', 28, 28, 'cute,meme'],
  ['ShadyLulu', '52492', 28, 28, 'meme,face'],
  ['ShazBotstix', '87', 24, 30, 'meme,face'],
  ['StrawBeary', '114876', 28, 28, 'cute,happy'],
  ['SwiftRage', '34', 21, 28, 'mad,meme'],
  ['TBAngel', '143490', 28, 28, 'happy,cute'],
  ['TearGlove', '160403', 28, 28, 'sad,cute'],
  ['TheRinger', '18', 20, 27, 'meme,face'],
  ['TheTarFu', '111351', 28, 28, 'meme,face'],
  ['TooSpicy', '114846', 28, 28, 'meme,fire'],
  ['TriHard', '120232', 28, 28, 'hype,meme'],
  ['TwitchLit', '166263', 28, 28, 'hype,fire'],
  ['UWot', '134255', 28, 28, 'meme,confused'],
  ['UnSane', '111792', 28, 28, 'meme,mad'],
  ['UncleNox', '114856', 28, 28, 'meme,face'],
  ['VoteYea', '106293', 28, 28, 'vote,happy'],
  ['WTRuck', '114847', 28, 28, 'meme,truck'],
  ['WholeWheat', '1896', 20, 30, 'meme,food'],
  ['YouWHY', '4337', 28, 28, 'sad,meme'],
  ['bleedPurple', '62835', 28, 28, 'twitch,hype'],
  ['cmonBruh', '84608', 28, 28, 'meme,sarcasm'],
  ['imGlitch', '112290', 28, 28, 'twitch,meme'],
  ['mcaT', '35063', 28, 28, 'cat,meme'],
  ['panicBasket', '22998', 28, 28, 'meme,panic'],
  ['twitchRaid', '62836', 28, 28, 'twitch,hype'],
  ['HmmFace', 'emotesv2_e455cf70a96e44db9da533b89d40f062', 28, 28, 'meme,thinking'],
  ['Shush', 'emotesv2_819621bcb8f44566a1bd8ea63d06c58f', 28, 28, 'meme,quiet'],
  ['DinoDance', 'emotesv2_dcd06b30a5c24f6eb871e8f5edbd44f7', 28, 28, 'dance,happy', 'a'],
  ['YappingDuck', 'emotesv2_bea88a2caa044e409c1b32ee0850b825', 28, 28, 'meme,duck', 'a'],
  ['ClapHands', 'emotesv2_cb4c93c8bef44ae6888cf239f98754b7', 28, 28, 'clap,happy', 'a'],
];
const CDN = 'https://static-cdn.jtvnw.net/emoticons/v2';
const CDN_SCALE = { 1: '1.0', 2: '2.0', 3: '3.0', 4: '3.0' }; // 3x has no CDN size: the 4x file, shown smaller
export const twitchUrl = (n, k, still) => {
  const t = TWITCH[n];
  if (!t || !CDN_SCALE[k]) return null;
  const format = t[5] ? (still ? 'static' : 'animated') : 'default';
  return `${CDN}/${t[1]}/${format}/dark/${CDN_SCALE[k]}`;
};
const twitchEmote = (n, owner = OTHERS[n % OTHERS.length]) => {
  const [name, , w, h, tags, a] = TWITCH[n];
  const animated = a === 'a';
  const idx = n;
  return {
    id: `01JD8X${String(n).padStart(20, '0')}`,
    name,
    state: 'APPROVED',
    approvals: approvalsOf('APPROVED'),
    processing: 'READY',
    animated,
    tags: tags.split(','),
    host: `${BASE}/twitch/${n}`,
    createdAt: new Date(NOW - idx * 0.2 * DAY - Math.floor(rngFor(n, 3)() * 0.18 * DAY)).toISOString(),
    flags: { zeroWidth: false, nsfw: false, private: false },
    visibility: 'PUBLIC',
    channelCount: n % 11 === 0 ? 0 : (n * 37) % 91 + (n % 5 === 0 ? 400 : 0),
    owner,
    files: [1, 2, 3, 4].flatMap((k) => [
      { name: `${k}x.webp`, format: 'WEBP', scale: k, width: w * k, height: h * k, size: 900 * k, isStatic: false },
      ...(animated
        ? [
            { name: `${k}x_static.webp`, format: 'WEBP', scale: k, width: w * k, height: h * k, size: 400 * k, isStatic: true },
            { name: `${k}x_static.avif`, format: 'AVIF', scale: k, width: w * k, height: h * k, size: 300 * k, isStatic: true },
          ]
        : [{ name: `${k}x.avif`, format: 'AVIF', scale: k, width: w * k, height: h * k, size: 600 * k, isStatic: false }]),
    ]),
  };
};
// Edge cases for the emote page: a very long name (index 10).
NAME_OF.set(10, 'AbsolutelyUnreasonablyLongEmoteNameThatKeepsGoing');
export const ALL = FACES ? Array.from({ length: 300 }, (_, i) => emote(i)) : TWITCH.map((_, n) => twitchEmote(n));
// The mock account's own uploads: a few of each state.
// Emote 11 is private (no activity, no tab); emote 12 is unlisted (ADR 0020: open by link, never
// in search or browse).
ALL[11].flags.private = true;
ALL[11].visibility = 'PRIVATE';
ALL[12].visibility = 'UNLISTED';
// The listing follows the review (ADR 0038): an unlisted emote's listing was denied, and staff look
// at its personal use in the personal queue. A few more approved emotes wait there, one was denied.
ALL[12].approvals = { publicListing: 'DENIED', personalUse: 'PENDING' };
for (const i of [20, 21, 22]) ALL[i].approvals = { publicListing: 'APPROVED', personalUse: 'PENDING' };
ALL[23].approvals = { publicListing: 'APPROVED', personalUse: 'DENIED' };
// NSFW emotes aren't allowed (ADR 0020, amendment 2026/11/22): `flags.nsfw` is always false.
export const MINE = Array.from({ length: 9 }, (_, i) =>
  emote(600 + i, ME_SUMMARY, i < 2 ? 'PENDING' : i === 2 ? 'REJECTED' : 'APPROVED'));
MINE[4].visibility = 'UNLISTED';
MINE[4].approvals = { publicListing: 'DENIED', personalUse: 'PENDING' };
MINE[5].approvals = { publicListing: 'APPROVED', personalUse: 'PENDING' };
// An upload whose transcode ran out of attempts: no files, shown as failed (notification
// `emote.failed`, see account.mjs).
export const FAILED = { ...emote(609, ME_SUMMARY, 'PENDING'), files: [], processing: 'FAILED' };
FAILED.name = 'BrokenUpload';
MINE.push(FAILED);

let uploads = 0;
// The next number for an emote the mock account uploads.
export const nextUpload = () => uploads++;
export const wideOf = (e) => {
  const one = e.files.find((f) => f.name === '1x.webp');
  return !!one && one.width * 4 >= one.height * 7;
};
for (const e of [...ALL, ...MINE]) e.flags.wide = wideOf(e);

// Trending (the API's emote_channel_adds rollup): made-up but stable net channel adds per window,
// growing with the window. About two thirds of the emotes trend at all.
const TREND_DAYS = { DAY: 1, WEEK: 7, MONTH: 30 };
export function trendAdds(e, window) {
  const n = ALL.indexOf(e);
  if (n < 0 || n % 3 === 0) return 0;
  const day = n % 4 === 1 ? ((n * 13) % 9) + 1 : 0;
  const week = day + ((n * 7) % 23);
  const month = week + ((n * 5) % 47);
  return { DAY: day, WEEK: week, MONTH: month }[window in TREND_DAYS ? window : 'WEEK'];
}
export const trendPool = (window) =>
  ALL.filter((e) => e.state === 'APPROVED' && e.visibility === 'PUBLIC' && trendAdds(e, window) > 0).sort(
    (a, b) => trendAdds(b, window) - trendAdds(a, window) || b.channelCount - a.channelCount || a.id.localeCompare(b.id),
  );

export const member = (e, name = e.name) => ({ name, zeroWidth: e.flags.zeroWidth, addedAt: e.createdAt, emote: e });
