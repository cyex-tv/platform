// The questions on /help, as data: the page renders them in groups, the index lists them, and the
// filter matches a question on its own words and its answer. Answers are short and written as the
// rest of the site: [links](/path) into the site, `code` for names you type. Nothing here loads
// anything from elsewhere.
import { paragraphs, type Segment } from '$lib/docs/example';

export interface Question {
  id: string;
  q: string;
  /** Paragraphs of the answer (blank lines split them). */
  a: Segment[][];
  /** Lowercased question and answer text, what the filter looks in. */
  hay: string;
}

export interface HelpGroup {
  id: string;
  title: string;
  questions: Question[];
}

const plain = (segs: Segment[][]) => segs.map((p) => p.map((s) => s.text).join('')).join(' ');

function group(id: string, title: string, items: [id: string, q: string, a: string][]): HelpGroup {
  return {
    id,
    title,
    questions: items.map(([qid, q, a]) => {
      const answer = paragraphs(a);
      return { id: qid, q, a: answer, hay: `${q} ${plain(answer)}`.toLowerCase() };
    }),
  };
}

export const HELP: HelpGroup[] = [
  group('getting-started', 'Getting started', [
    ['what-is-cyex-tv', 'What is cyex.tv?',
      'A home for chat emotes on Twitch and Kick. Upload emotes, collect them in sets, and show a set in your channel. Viewers with the cyex.tv extension (or a chat client that reads cyex.tv) see them in chat.'],
    ['do-i-need-an-account', 'Do I need an account?',
      'Not to look around: [browsing emotes](/search), emote pages and profiles are public. You need one to upload, make sets or use them in your channel.'],
    ['how-do-i-sign-in', 'How do I sign in?',
      '[Sign in](/signin) with Twitch or Kick. cyex.tv never sees your password; the platform tells us who you are and nothing else. You can link both platforms to one account.'],
    ['what-does-it-cost', 'Does it cost anything?',
      'No. Emotes, sets and channels are free. Badges and paints are handed out by the cyex.tv team for now.'],
  ]),
  group('extension', 'The extension', [
    ['get-the-extension', 'Where do I get the extension?',
      'It’s coming to the Chrome Web Store and Firefox Add-ons. Until then you can load the test build yourself; [Download](/download) has the steps.'],
    ['which-browsers', 'Which browsers does it work in?',
      'Chrome, Edge, Brave, Opera and other Chromium browsers, and Firefox on the desktop. Phone browsers don’t run extensions.'],
    ['other-providers', 'Does it show 7TV, BTTV and FFZ emotes too?',
      'Yes, each with its own switch in the extension’s settings. If you already run one of their extensions, cyex.tv leaves the doubled parts (emote menus, Tab completion) to it.'],
    ['emotes-not-showing', 'Emotes don’t show in chat. What now?',
      'Check that the channel has an active set (its page at `cyex.tv/channels/twitch/<login>` says which), that cyex.tv is switched on in the extension’s settings, and reload the chat once. Emotes match whole words and are case-sensitive.'],
    ['hide-twitch-parts', 'Can it hide parts of Twitch or Kick?',
      'Yes. Settings → Hide on Twitch and Hide on Kick let you turn off things you never use, one switch each. Nothing is hidden until you choose.'],
  ]),
  group('uploading', 'Uploading', [
    ['what-files', 'What files can I upload?',
      'PNG, APNG, GIF or WebP, up to 5 MB, 1000 × 1000 px and 1000 frames. Square or wide works; very tall emotes look small in chat. You can [upload several at once](/me/upload).'],
    ['why-pending', 'Why is my emote pending?',
      'Every upload is checked by a moderator before others can find it. While it’s pending you can already use it in your own sets and channel.'],
    ['zero-width', 'What is a zero-width emote?',
      'One that sits on top of the emote before it instead of next to it: hats, rain, sparkles. Mark it when you upload, or later on the emote’s page.'],
    ['private-emotes', 'What do unlisted and private mean?',
      'An unlisted emote opens for anyone with its link, and anyone with the link can add it to their sets, but it isn’t in search, browse or your profile for others. Only you (and editors you allow) can add a private emote to sets, and it isn’t listed either. Both still show in chat wherever your sets are active.'],
    ['nsfw', 'When do I mark an emote NSFW?',
      'When it isn’t fine for every stream: suggestive, gory or otherwise not for all ages. NSFW emotes are left out of browsing unless people ask for them. Nothing against Twitch’s or Kick’s rules belongs on cyex.tv at all.'],
    ['upload-refused', 'My upload was refused. Why?',
      'Usually the file is over a limit (the message says which), or it’s an image that was taken down before; those can’t be uploaded again. After 20 uploads in an hour, wait a little.'],
    ['rename-emote', 'Can I rename an emote or change its tags?',
      'Yes, on the emote’s edit page: [Your emotes](/me) → the emote → Edit. A set can also give an emote its own name (an alias) without renaming it for everyone.'],
  ]),
  group('sets-and-channels', 'Sets and channels', [
    ['what-is-a-set', 'What is a set?',
      'A list of emotes with the names they have in chat. You can keep several, for example one per season, and switch which one a channel shows.'],
    ['use-in-channel', 'How do I use a set in my channel?',
      'Make a set on [your profile](/me), add emotes from their pages or from search, then pick it as the active set under [Channels](/me/channels). Chat picks it up within seconds.'],
    ['personal-set', 'What is a personal set?',
      'Emotes you take with you: they show in your own messages in every channel that allows personal emotes. Only approved emotes work there.'],
    ['global-set', 'Why do some emotes show everywhere?',
      'Those are in the global set, which every channel shows. The cyex.tv team keeps it small.'],
    ['set-full', 'My set is full. Can it hold more?',
      'Each set has a capacity. Remove emotes you don’t use, or start a second set; staff can raise the capacity for big channels.'],
  ]),
  group('editors', 'Editors', [
    ['what-are-editors', 'What are editors?',
      'People you trust to work on your account, usually your mods. You add them by their Twitch or Kick login under [Editors](/me/editors); they need to have signed in to cyex.tv once.'],
    ['editor-permissions', 'What can an editor do?',
      'Only what you allow: emotes in your sets, the sets themselves and your channel’s active set, other editors (never more than they hold), and the badge and paint you show. Change or remove it any time.'],
    ['editor-of-someone', 'How do I work on a channel I edit?',
      'Accounts you edit are listed under [Editors](/me/editors). Open one to manage its sets and channels as you would your own.'],
  ]),
  group('cosmetics', 'Cosmetics', [
    ['what-are-cosmetics', 'What are badges and paints?',
      'A badge is a small picture before your name in chat; a paint colours your name. People with the cyex.tv extension see them on Twitch and Kick. [See them all](/cosmetics).'],
    ['get-cosmetics', 'How do I get one?',
      'For now the cyex.tv team hands them out, for testing, events and helping out. When you have one, pick it on your profile under Cosmetics.'],
    ['who-sees-cosmetics', 'Who can see which ones I own?',
      'Only you. Others see the badge and paint you show, never the list of what you own.'],
  ]),
  group('account-and-privacy', 'Account and privacy', [
    ['what-we-keep', 'What does cyex.tv keep about me?',
      'Your Twitch or Kick ID, login, display name and picture, what you upload, your sets and channels, and what you change (some changes show in your activity). No passwords, no email unless you send us one.'],
    ['sign-out-everywhere', 'How do I sign out?',
      'Account menu → Sign out signs this browser out. [Settings → Account](/me/settings/account) → Security → Sign out everywhere ends every session, the apps too. There you can also disconnect a platform you no longer use (you always keep at least one) and pick which connection’s name you go by.'],
    ['delete-account', 'Can I delete my account?',
      'Yes, yourself: Settings → Account, then Delete account, and type one of your logins to confirm. Your account is hidden at once and deleted for good after 30 days. Sign in before then to undo it. After that, your emotes, sets, channels, cosmetics and notifications are gone. Approved emotes that other people have in their sets stay there, shown as made by “Deleted user”.'],
  ]),
  group('moderation', 'Moderation and takedowns', [
    ['report-emote', 'How do I report an emote?',
      'Open the emote’s page and choose Report, with a reason. Moderators work through reports in order.'],
    ['takedown-request', 'Someone uploaded my work or a picture of me.',
      'Email abuse@cyex.tv with the emote’s link, what it copies or shows, and how to reach you. We answer within a few days, and act sooner on anything illegal. [How takedowns work](https://github.com/cyex-tv/platform/blob/main/docs/takedown.md).'],
    ['what-takedown-does', 'What happens when an emote is taken down?',
      'It leaves every set at once, its files are deleted, and the image can’t be uploaded again. Its owner is told why in their notifications.'],
    ['rejected', 'My emote was rejected. Can I try again?',
      'Yes: fix what the reason names and upload it again. The reason is in your notifications.'],
  ]),
];

export const ALL_QUESTIONS: Question[] = HELP.flatMap((g) => g.questions);

/** Whether a question matches the filter text: every word of it, anywhere in question or answer. */
export function matches(question: Question, text: string): boolean {
  const words = text.trim().toLowerCase().split(/\s+/).filter(Boolean);
  return words.every((w) => question.hay.includes(w));
}
