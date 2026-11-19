/* eslint-disable */
import type { DocumentTypeDecoration } from '@graphql-typed-document-node/core';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
};

/** What one account shows, with the badge and paint themselves (`accountCosmetics`). */
export type AccountCosmetics = {
  __typename?: 'AccountCosmetics';
  accountId: Scalars['String']['output'];
  badge?: Maybe<Badge>;
  paint?: Maybe<Paint>;
};

/** Someone who edits an account, with what they may do. */
export type AccountEditor = {
  __typename?: 'AccountEditor';
  account: AccountSummary;
  grantedAt: Scalars['String']['output'];
  /** Who added them: the owner, or an editor with `editors`. */
  grantedBy?: Maybe<AccountSummary>;
  permissions: Array<EditorPermission>;
  updatedAt: Scalars['String']['output'];
};

/** One `searchAccounts` hit: the account's public summary and its linked logins. */
export type AccountHit = {
  __typename?: 'AccountHit';
  account: AccountSummary;
  /** Twitch first. */
  identities: Array<PublicIdentity>;
};

/**
 * An account's public profile, like 7TV's public user pages. Listed sets come from
 * `accountSets`.
 */
export type AccountProfile = {
  __typename?: 'AccountProfile';
  account: AccountSummary;
  /** Each channel's active set, for channels showing one. */
  activeSets: Array<ActiveChannelSet>;
  createdAt: Scalars['String']['output'];
  /** Linked Twitch and Kick accounts: platform, login and display name only. Twitch first. */
  identities: Array<PublicIdentity>;
  /** Roles marked public, most senior first. */
  roles: Array<PublicRole>;
};

/**
 * Reports about an account's emotes and reports it filed (`emotes.review` or
 * `emotes.takedown`), newest first, at most 50 each.
 */
export type AccountReports = {
  __typename?: 'AccountReports';
  about: Array<Report>;
  by: Array<Report>;
};

export type AccountResults = {
  __typename?: 'AccountResults';
  items: Array<AccountHit>;
  page: Scalars['Int']['output'];
  perPage: Scalars['Int']['output'];
  /** How many accounts match, counted up to 1000. */
  total: Scalars['Int']['output'];
  /** More than `total` match: show it as "1000+". */
  totalCapped: Scalars['Boolean']['output'];
};

/** One place you are signed in. Never holds the token or anything made from it. */
export type AccountSession = {
  __typename?: 'AccountSession';
  createdAt: Scalars['String']['output'];
  /** The session this request came with. */
  current: Scalars['Boolean']['output'];
  /**
   * For example `Firefox on Windows`, `cyex.tv app` or `Unknown device`, from the user agent
   * that signed in.
   */
  device: Scalars['String']['output'];
  expiresAt: Scalars['String']['output'];
  /** Pass to `revokeSession`. */
  id: Scalars['String']['output'];
  kind: SessionKind;
  /** Updated at most once an hour. */
  lastSeenAt: Scalars['String']['output'];
};

/** What an account may do (`users.restrict`). */
export type AccountStatus = {
  __typename?: 'AccountStatus';
  /** No sign-in, no sessions. */
  blocked: Scalars['Boolean']['output'];
  blockedUntil?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  /** No reports or tickets. */
  ticketsBlocked: Scalars['Boolean']['output'];
  ticketsBlockedUntil?: Maybe<Scalars['String']['output']>;
  /** No uploads. */
  uploadsBlocked: Scalars['Boolean']['output'];
  /** When each hold ends; empty while it is off or for good. */
  uploadsBlockedUntil?: Maybe<Scalars['String']['output']>;
};

/** A short reference to an account, for owners and editors. */
export type AccountSummary = {
  __typename?: 'AccountSummary';
  avatar?: Maybe<Scalars['String']['output']>;
  displayName: Scalars['String']['output'];
  id: Scalars['String']['output'];
};

/** The set a channel shows now, on a profile ("Active emotes"). */
export type ActiveChannelSet = {
  __typename?: 'ActiveChannelSet';
  /** The channel's login. */
  login: Scalars['String']['output'];
  platform: Platform;
  setId: Scalars['String']['output'];
  setName: Scalars['String']['output'];
};

/**
 * A badge or paint change: what it was and what it is now. Empty ends mean none; names are
 * empty when the cosmetic was deleted since.
 */
export type ActivityCosmeticChange = {
  __typename?: 'ActivityCosmeticChange';
  fromId?: Maybe<Scalars['String']['output']>;
  fromName?: Maybe<Scalars['String']['output']>;
  toId?: Maybe<Scalars['String']['output']>;
  toName?: Maybe<Scalars['String']['output']>;
};

/** An emote, as a feed line names it. `name` is what it was called in the set at the time. */
export type ActivityEmote = {
  __typename?: 'ActivityEmote';
  files: Array<EmoteFile>;
  /** For example `https://cdn.cyex.tv/emotes/<sha256>`. */
  host: Scalars['String']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
};

/** One line of an account's public activity feed (7TV's "Activity" tab). */
export type ActivityEntry = {
  __typename?: 'ActivityEntry';
  /** Who did it: the account or one of its editors. */
  actor: AccountSummary;
  badge?: Maybe<ActivityCosmeticChange>;
  createdAt: Scalars['String']['output'];
  emote?: Maybe<ActivityEmote>;
  fromName?: Maybe<Scalars['String']['output']>;
  /** An audit row ID; page back with it. */
  id: Scalars['String']['output'];
  kind: ActivityKind;
  paint?: Maybe<ActivityCosmeticChange>;
  set?: Maybe<ActivitySet>;
  toName?: Maybe<Scalars['String']['output']>;
};

/** What one line of an account's public activity feed says happened. */
export type ActivityKind =
  /** A channel's active set changed (`set` is the new one; `from_name`, `to_name`). */
  | 'ACTIVE_SET_CHANGED'
  /** The badge and/or paint shown changed (`badge`, `paint`). */
  | 'COSMETICS_SELECTED'
  /** An emote of the account was approved and listed. */
  | 'EMOTE_UPLOADED'
  | 'SET_CREATED'
  | 'SET_EMOTE_ADDED'
  | 'SET_EMOTE_REMOVED'
  /** The emote's name in the set changed (`from_name`, `to_name`). */
  | 'SET_EMOTE_RENAMED'
  /** A set was renamed (`from_name`, `to_name`). */
  | 'SET_RENAMED';

/**
 * A page of an activity feed. A page can hold fewer lines than asked for without being the last
 * one: the feed skips what isn't public.
 */
export type ActivityPage = {
  __typename?: 'ActivityPage';
  entries: Array<ActivityEntry>;
  /** Pass as `before` to get the next page; null when there is none. */
  next?: Maybe<Scalars['String']['output']>;
};

/** A set, as a feed line names it. */
export type ActivitySet = {
  __typename?: 'ActivitySet';
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
};

/** Who to add as an editor: an account ID, or a Twitch or Kick login. */
export type AddAccountEditorInput = {
  /** The editor's cyex.tv account ID. Or give `platform` and `login`. */
  accountId?: InputMaybe<Scalars['String']['input']>;
  login?: InputMaybe<Scalars['String']['input']>;
  /** The account to add editors to. Empty: your own. */
  ownerId?: InputMaybe<Scalars['String']['input']>;
  /** At least one. */
  permissions: Array<EditorPermission>;
  platform?: InputMaybe<Platform>;
};

export type AddSetEmoteInput = {
  /** Rename it in this set. Defaults to the emote's own name. */
  name?: InputMaybe<Scalars['String']['input']>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

/** An account as staff see it (`users.view` or `users.restrict`). */
export type AdminAccount = {
  __typename?: 'AdminAccount';
  avatar?: Maybe<Scalars['String']['output']>;
  blocked: Scalars['Boolean']['output'];
  blockedUntil?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  displayName: Scalars['String']['output'];
  id: Scalars['String']['output'];
  identities: Array<Identity>;
  /** No reports or tickets. */
  ticketsBlocked: Scalars['Boolean']['output'];
  ticketsBlockedUntil?: Maybe<Scalars['String']['output']>;
  uploadsBlocked: Scalars['Boolean']['output'];
  /** When each hold ends; empty while it is off or for good. */
  uploadsBlockedUntil?: Maybe<Scalars['String']['output']>;
};

/** One audited change (ADR 0010). `before` and `after` are JSON, as text. */
export type AuditEntry = {
  __typename?: 'AuditEntry';
  /** For example `role.created` or `set.editor_added`. */
  action: Scalars['String']['output'];
  /** The account that made the change; empty for changes the system made. */
  actorId?: Maybe<Scalars['String']['output']>;
  after?: Maybe<Scalars['String']['output']>;
  before?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  targetId: Scalars['String']['output'];
  /** For example `role`, `account` or `set`. */
  targetKind: Scalars['String']['output'];
};

/** A chat badge, shown before a chatter's name. */
export type Badge = {
  __typename?: 'Badge';
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  /** Square, 18 px at 1x. */
  image: CosmeticImage;
  name: Scalars['String']['output'];
  tooltip: Scalars['String']['output'];
  updatedAt: Scalars['String']['output'];
};

/** The roles cyex.tv comes with (ADR 0010). */
export type BuiltInRole =
  /** "Admin": can't be deleted. */
  | 'ADMIN'
  /** The top role: every permission, held by the owners only, never granted or edited. */
  | 'ROOT'
  /** "User": every account holds it without an assignment. Starts with no permissions. */
  | 'USER';

export type Channel = {
  __typename?: 'Channel';
  account: AccountSummary;
  activeSetId?: Maybe<Scalars['String']['output']>;
  /** Whether other people's personal emotes show in this channel. */
  allowPersonalEmotes: Scalars['Boolean']['output'];
  login: Scalars['String']['output'];
  platform: Platform;
  /** Twitch: the broadcaster's user ID. Kick: `broadcaster_user_id`. */
  platformChannelId: Scalars['String']['output'];
};

/** Who you are on a channel you can manage, highest first. */
export type ChannelRole =
  /** You are an editor of the channel's account. */
  | 'ACCOUNT_EDITOR'
  /** The channel is yours. */
  | 'OWNER'
  /** You edit at least one set of the channel's account. */
  | 'SET_EDITOR';

/**
 * What `startCheckout` gives: open Paddle's checkout for this transaction (Paddle.js
 * `Paddle.Checkout.open({ transactionId })` with `clientToken`, or go to `checkoutUrl`). The
 * order turns `PAID` when Paddle's webhook says so; poll `order(id)`.
 */
export type Checkout = {
  __typename?: 'Checkout';
  /** Paddle's hosted checkout link, when the Paddle account has a default payment link. */
  checkoutUrl?: Maybe<Scalars['String']['output']>;
  /** The Paddle.js client-side token, when configured. */
  clientToken?: Maybe<Scalars['String']['output']>;
  /** `sandbox` or `production`: pass to `Paddle.Environment.set`. */
  environment: Scalars['String']['output'];
  order: Order;
  transactionId: Scalars['String']['output'];
};

/** What `cloneSet` made. */
export type ClonedSet = {
  __typename?: 'ClonedSet';
  /** Emotes copied into it. */
  copied: Scalars['Int']['output'];
  /** The new channel set. */
  set: EmoteSet;
  /**
   * Emotes left out: over capacity, private emotes of others, emotes not approved, or ones
   * you can't see.
   */
  skipped: Scalars['Int']['output'];
};

export type CodeBatch = {
  __typename?: 'CodeBatch';
  /** Cosmetics a code gives besides the item's. */
  badgeIds: Array<Scalars['String']['output']>;
  codeCount: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  createdBy?: Maybe<AccountSummary>;
  disabledAt?: Maybe<Scalars['String']['output']>;
  expiresAt?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  /** The item whose cosmetics a code gives, if any. */
  itemId?: Maybe<Scalars['String']['output']>;
  itemName?: Maybe<Scalars['String']['output']>;
  name: Scalars['String']['output'];
  paintIds: Array<Scalars['String']['output']>;
  /** Codes look like `PREFIX-XXXX-XXXX-XXXX-XXXX`. */
  prefix: Scalars['String']['output'];
  /** Uses so far, over every code. */
  redemptions: Scalars['Int']['output'];
  usesPerCode: Scalars['Int']['output'];
};

export type CodeBatchInput = {
  badgeIds?: Array<Scalars['String']['input']>;
  /** 1 to 1000 codes. */
  count: Scalars['Int']['input'];
  /** RFC 3339, in the future. */
  expiresAt?: InputMaybe<Scalars['String']['input']>;
  /** A shop item whose cosmetics a code gives. */
  itemId?: InputMaybe<Scalars['String']['input']>;
  /** For staff, 1 to 64 characters. */
  name: Scalars['String']['input'];
  paintIds?: Array<Scalars['String']['input']>;
  /** 1 to 12 letters or digits (upper-cased). */
  prefix: Scalars['String']['input'];
  /** 1 to 10000, default 1. Each account can use a code once. */
  usesPerCode?: InputMaybe<Scalars['Int']['input']>;
};

export type CodeBatchPage = {
  __typename?: 'CodeBatchPage';
  items: Array<CodeBatch>;
  total: Scalars['Int']['output'];
};

/** Every badge and paint, for clients to cache (`GET /v1/cosmetics`). */
export type CosmeticCatalog = {
  __typename?: 'CosmeticCatalog';
  badges: Array<Badge>;
  paints: Array<Paint>;
};

/** A badge's or an image paint's picture, served at `{host}/{name}` like emote files. */
export type CosmeticImage = {
  __typename?: 'CosmeticImage';
  animated: Scalars['Boolean']['output'];
  createdAt: Scalars['String']['output'];
  /**
   * WebP and AVIF at 1x, 2x and 4x, and static first frames for animations. Empty until the
   * worker made them.
   */
  files: Array<EmoteFile>;
  height: Scalars['Int']['output'];
  /** For example `https://cdn.cyex.tv/cosmetics/badges/<sha256>`. */
  host: Scalars['String']['output'];
  id: Scalars['String']['output'];
  kind: CosmeticKind;
  /** The files are made: only then can a badge or paint use it. */
  ready: Scalars['Boolean']['output'];
  /** The original's size in px. */
  width: Scalars['Int']['output'];
};

/** A badge or a paint. */
export type CosmeticKind =
  | 'BADGE'
  | 'PAINT';

export type CreateBadgeInput = {
  /** A ready badge image from `POST /v1/cosmetics/images`. */
  imageId: Scalars['String']['input'];
  /** 1 to 32 characters, unique. */
  name: Scalars['String']['input'];
  /** 1 to 64 characters. */
  tooltip: Scalars['String']['input'];
};

export type CreateRoleInput = {
  description?: InputMaybe<Scalars['String']['input']>;
  name: Scalars['String']['input'];
  permissions: Array<Scalars['String']['input']>;
  position: Scalars['Int']['input'];
  /** Show it on its holders' public profiles. Default false. */
  public?: InputMaybe<Scalars['Boolean']['input']>;
};

export type CreateSetInput = {
  /** `CHANNEL` or `PERSONAL`. The global set exists already. */
  kind: SetKind;
  name: Scalars['String']['input'];
  /** Make the set for an account you edit with `sets` (ADR 0014). Empty: for yourself. */
  ownerId?: InputMaybe<Scalars['String']['input']>;
  /**
   * Empty: `LISTED` for channel sets, `UNLISTED` for the personal set. Personal sets can't be
   * `PRIVATE`.
   */
  visibility?: InputMaybe<SetVisibility>;
};

/** A new batch and its codes in plain text. This is the only time they are shown. */
export type CreatedCodeBatch = {
  __typename?: 'CreatedCodeBatch';
  batch: CodeBatch;
  codes: Array<Scalars['String']['output']>;
};

/** A channel you can manage, with your highest role on it. */
export type EditableChannel = {
  __typename?: 'EditableChannel';
  channel: Channel;
  role: ChannelRole;
};

/** An account you edit, with what you may do there. */
export type EditedAccount = {
  __typename?: 'EditedAccount';
  account: AccountSummary;
  grantedAt: Scalars['String']['output'];
  permissions: Array<EditorPermission>;
};

/** What an account editor may do on the account they edit. The account's owner has all of them. */
export type EditorPermission =
  /** Add, change and remove editors, never granting more than you hold. */
  | 'EDITORS'
  /** Add, remove, rename and alias emotes in the owner's sets, the active channel sets too. */
  | 'EMOTES'
  /** Pick the badge and paint the owner shows (ADR 0015). Never grants. */
  | 'PROFILE'
  /**
   * Create, rename and delete sets, change their capacity, and change the owner's channels'
   * active set and settings.
   */
  | 'SETS';

export type Emote = {
  __typename?: 'Emote';
  animated: Scalars['Boolean']['output'];
  /** How many channels have this emote in their active set right now. */
  channelCount: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  files: Array<EmoteFile>;
  flags: EmoteFlags;
  /** For example `https://cdn.cyex.tv/emotes/<sha256>`. */
  host: Scalars['String']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
  state: EmoteState;
  tags: Array<Scalars['String']['output']>;
  /**
   * `PUBLIC`, `UNLISTED` or `PRIVATE` (ADR 0020). `flags.private` says the same for
   * `PRIVATE`, for clients from before `UNLISTED`.
   */
  visibility: EmoteVisibility;
};

/** One file of an emote, served at `{host}/{name}`. */
export type EmoteFile = {
  __typename?: 'EmoteFile';
  format: ImageFormat;
  height: Scalars['Int']['output'];
  /** The first frame of an animated emote. */
  isStatic: Scalars['Boolean']['output'];
  /** For example `2x.webp` or `1x_static.avif`. */
  name: Scalars['String']['output'];
  /** 1 to 4. */
  scale: Scalars['Int']['output'];
  /** Bytes. */
  size: Scalars['Int']['output'];
  width: Scalars['Int']['output'];
};

export type EmoteFlags = {
  __typename?: 'EmoteFlags';
  nsfw: Scalars['Boolean']['output'];
  /** Only its owner can add it to sets. True exactly when `visibility` is `PRIVATE`. */
  private: Scalars['Boolean']['output'];
  zeroWidth: Scalars['Boolean']['output'];
};

/** A page of emotes and how many there are in all. */
export type EmotePage = {
  __typename?: 'EmotePage';
  items: Array<Emote>;
  total: Scalars['Int']['output'];
};

export type EmoteSet = {
  __typename?: 'EmoteSet';
  capacity: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  emotes: Array<SetEmote>;
  id: Scalars['String']['output'];
  kind: SetKind;
  /** `visibility` is `LISTED`: shown on the owner's profile. */
  listed: Scalars['Boolean']['output'];
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
  /**
   * How many of `capacity` the set uses. For whoever may edit the set it counts every emote in
   * it, pending ones included, so `capacity - slotsUsed` is what still fits. For anyone else it
   * counts the emotes this answer may show them (approved ones, plus their own pending ones).
   */
  slotsUsed: Scalars['Int']['output'];
  /** Who can find and open it. */
  visibility: SetVisibility;
};

/** How `searchEmotes` orders its results. */
export type EmoteSort =
  /** The newest first. */
  | 'NEW'
  /** Best match first, then the most used. The default. */
  | 'RELEVANCE'
  /** The most used first (channels showing the emote), then by name. */
  | 'TOP';

export type EmoteState =
  | 'APPROVED'
  | 'PENDING'
  | 'REJECTED'
  | 'TAKEN_DOWN';

/** Who can find an emote and add it to sets (ADR 0020). */
export type EmoteVisibility =
  /** Only its owner can add it to sets, and it is left out of listings (`flags.private`). */
  | 'PRIVATE'
  /** Found by search and browse, listed on its creator's page. New emotes start here. */
  | 'PUBLIC'
  /**
   * Anyone with its link can open it and add it to sets; search, browse, activity feeds and
   * its creator's page leave it out for everyone but its owner.
   */
  | 'UNLISTED';

/** One account owning one cosmetic. */
export type Entitlement = {
  __typename?: 'Entitlement';
  account: AccountSummary;
  cosmeticId: Scalars['String']['output'];
  grantedAt: Scalars['String']['output'];
  grantedBy?: Maybe<AccountSummary>;
  kind: CosmeticKind;
  reason?: Maybe<Scalars['String']['output']>;
  source: EntitlementSource;
};

export type EntitlementPage = {
  __typename?: 'EntitlementPage';
  items: Array<Entitlement>;
  total: Scalars['Int']['output'];
};

/**
 * How an account came to own a cosmetic: given by staff, bought in the shop (ADR 0016), from an
 * event, or from a redeem code.
 */
export type EntitlementSource =
  | 'CODE'
  | 'EVENT'
  | 'GRANT'
  | 'PURCHASE';

/** One change, as delivered on both sockets. */
export type Event = {
  __typename?: 'Event';
  /** `user.cosmetics_updated`: the badge shown now; empty for none. */
  badgeId?: Maybe<Scalars['String']['output']>;
  /** `cosmetic.*`: its ID. */
  cosmeticId?: Maybe<Scalars['String']['output']>;
  /** `cosmetic.*`: which kind of cosmetic. */
  cosmeticKind?: Maybe<CosmeticKind>;
  createdAt: Scalars['String']['output'];
  emoteId?: Maybe<Scalars['String']['output']>;
  eventType: EventType;
  /** Increases with every event. Send the last one you saw to resume. */
  id: Scalars['String']['output'];
  /** The emote's name in the set, for added and renamed. */
  name?: Maybe<Scalars['String']['output']>;
  /** `user.cosmetics_updated`: the paint shown now; empty for none. */
  paintId?: Maybe<Scalars['String']['output']>;
  setId?: Maybe<Scalars['String']['output']>;
  /** The topic it was delivered for, for example `channel:twitch:123`. */
  topic: Scalars['String']['output'];
};

/** What changed. On the event WebSocket these are written `set.emote_added` and so on. */
export type EventType =
  | 'CHANNEL_ACTIVE_SET_CHANGED'
  /** A badge or paint was made (topic `cosmetics`, ADR 0015). */
  | 'COSMETIC_CREATED'
  /** Gone: stop showing it on anyone. */
  | 'COSMETIC_DELETED'
  | 'COSMETIC_UPDATED'
  | 'EMOTE_UPDATED'
  | 'PERSONAL_UPDATED'
  /** Too much was missed: refetch what you show. */
  | 'RESYNC'
  /**
   * A listed set was made (`createSet`, `cloneSet`), on its owner's `user:` topics. Unlisted
   * and private sets make no event, so their IDs stay unknown.
   */
  | 'SET_CREATED'
  | 'SET_EMOTE_ADDED'
  | 'SET_EMOTE_REMOVED'
  | 'SET_EMOTE_RENAMED'
  /** The badge or paint a user shows changed (their `user:` topics). */
  | 'USER_COSMETICS_UPDATED';

export type GrantCosmeticInput = {
  accountId: Scalars['String']['input'];
  cosmeticId: Scalars['String']['input'];
  kind: CosmeticKind;
  /** Why, for the audit log. 1 to 500 characters. */
  reason: Scalars['String']['input'];
};

/** A Twitch or Kick account linked to a cyex.tv account. */
export type Identity = {
  __typename?: 'Identity';
  avatar?: Maybe<Scalars['String']['output']>;
  displayName: Scalars['String']['output'];
  login: Scalars['String']['output'];
  platform: Platform;
  /** The platform's numeric user ID, as text. */
  platformUserId: Scalars['String']['output'];
};

export type ImageFormat =
  | 'AVIF'
  | 'WEBP';

/** The signed-in account. */
export type Me = {
  __typename?: 'Me';
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  identities: Array<Identity>;
  /** Permission names from the account's roles (ADR 0010). */
  permissions: Array<Scalars['String']['output']>;
  /**
   * Every role the account holds, public or not, most senior first. Only you see the ones
   * that aren't public: your profile marks them hidden; others see `AccountProfile.roles`.
   */
  roles: Array<OwnRole>;
};

export type Mutation = {
  __typename?: 'Mutation';
  /**
   * Make someone an editor of your account, or of an account you edit with `EDITORS` (ADR
   * 0014). Give their account ID, or a platform and login. You can only grant permissions you
   * hold. At most 20 editors per account. Audited.
   */
  addAccountEditor: AccountEditor;
  /**
   * Make someone an editor of a set. Its owner, or their account editors with `EDITORS` and
   * `EMOTES`.
   */
  addSetEditor: Scalars['Boolean']['output'];
  /** Add an emote to a set. Personal sets take approved emotes only. */
  addSetEmote: SetEmote;
  /** Approve a pending emote. Needs `emotes.review`. Audited. */
  approveEmote: Emote;
  /**
   * Give an account a role below your highest one that grants only what you hold. Needs
   * `roles.manage` or `roles.assign`; the User and top roles can't be given. Audited.
   */
  assignRole: Scalars['Boolean']['output'];
  /**
   * Copy a set you can open into a new channel set: yours, or with `ownerId` one for an account
   * you edit with `SETS`. Name defaults to the source's; visibility to `LISTED`. Emotes go in
   * as `addSetEmote` would put them, in order, keeping their names: up to the capacity,
   * approved ones only (your own pending ones too), never other people's private emotes.
   * 30 copies an hour. `rate_limited`, `not_found`, `forbidden`, `bad_name`.
   */
  cloneSet: ClonedSet;
  /**
   * Make a badge from a ready badge picture (`POST /v1/cosmetics/images`). Needs
   * `cosmetics.manage`. Audited; publishes `cosmetic.created`.
   */
  createBadge: Badge;
  /**
   * Make a batch of 1 to 1000 redeem codes. The answer holds the codes in plain text; this is
   * the only time they are shown. Needs `codes.manage`. Audited.
   */
  createCodeBatch: CreatedCodeBatch;
  /** Make a paint. Needs `cosmetics.manage`. Audited; publishes `cosmetic.created`. */
  createPaint: Paint;
  /** Create a role. You can only grant permissions you hold. Needs `roles.manage`. Audited. */
  createRole: Role;
  /**
   * Create a channel set, or your personal set (one per account). With `ownerId`, for an
   * account you edit with `SETS`.
   */
  createSet: EmoteSet;
  /** Make a shop item: a bundle of 1 to 10 badges and paints. Needs `shop.manage`. Audited. */
  createShopItem: ShopItem;
  /**
   * Delete your account. `confirm` is one of your Twitch or Kick logins. Website only
   * (`website_only` with an app token); the owner's account can't be (`owner_protected`).
   * The account is hidden at once (profile, sets, emotes and channels read as gone, every
   * session ends, the cookie is cleared) and erased for good 30 days later: then your
   * identities, channels, sets, editor grants, cosmetics and notifications go, your approved
   * emotes that are in other people's sets stay, shown as by a deleted user, and your other
   * emotes are deleted. Signing in again before then offers to undo it (`pendingDeletion`,
   * `restoreAccount`); after it, signing in makes a new account. Audited.
   * `confirm_mismatch` when `confirm` isn't one of your logins; `restricted` while the account's
   * uploads or sign-in are blocked. 3 tries an hour (`rate_limited`).
   */
  deleteAccount: Scalars['Boolean']['output'];
  /**
   * Delete a badge: everyone loses it and stops showing it. False when there was none. Needs
   * `cosmetics.manage`. Audited.
   */
  deleteBadge: Scalars['Boolean']['output'];
  /** Delete your emote. It is removed from every set. */
  deleteEmote: Scalars['Boolean']['output'];
  /**
   * Delete a paint: everyone loses it and stops showing it. False when there was none. Needs
   * `cosmetics.manage`. Audited.
   */
  deletePaint: Scalars['Boolean']['output'];
  /**
   * Delete a role below your own highest one that you don't hold; built-in roles stay.
   * Needs `roles.manage`. Audited.
   */
  deleteRole: Scalars['Boolean']['output'];
  /**
   * Delete a set. Its owner, or their account editors with `SETS`. A channel's active set
   * can't be deleted.
   */
  deleteSet: Scalars['Boolean']['output'];
  /**
   * Delete a shop item: never listed or sold again; its orders stay. False when there was
   * none. Needs `shop.manage`. Audited.
   */
  deleteShopItem: Scalars['Boolean']['output'];
  /** Turn off every code of a batch. Needs `codes.manage`. Audited. */
  disableCodeBatch: CodeBatch;
  /**
   * A copy of everything about your account, as JSON text (`format`, `version`, `account`,
   * `identities`, `sessions`, `channels`, `emotes`, `sets`, `set_editors`, `account_editors`,
   * `editor_of`, `cosmetics`, `roles`, `reports_filed`, `notifications`, `orders`). 5 an hour
   * (`rate_limited`). Audited.
   */
  exportMyData: Scalars['String']['output'];
  /** Give an account a badge or paint, with a reason. Needs `cosmetics.grant`. Audited. */
  grantCosmetic: Entitlement;
  /**
   * Lift a restriction early, with an optional reason. Needs the same permission as placing
   * it. Audited.
   */
  liftRestriction: AccountStatus;
  /**
   * Mark your notifications read: those in `ids` (up to 100; others' IDs are ignored), or all
   * of them without `ids`. Returns how many were unread.
   */
  markNotificationsRead: Scalars['Int']['output'];
  /**
   * Restrict an account with a reason, for good or `until` a time: no uploads
   * (`users.restrict_uploads`), no reports or tickets (`users.restrict_tickets`), or a full
   * block that ends its sessions (`users.block`); `users.restrict` does all three. A running
   * restriction of the same kind is replaced. The owner, and accounts whose highest role is at
   * or above yours, are protected. Audited.
   */
  placeRestriction: AccountStatus;
  /**
   * Redeem a code (case, spaces and dashes don't matter). Wrong codes lock you out for a
   * while after a few tries (`locked_out`). Each cosmetic it gives sends a `cosmetic.granted`
   * notification (`source` `code`).
   */
  redeemCode: RedeemResult;
  /**
   * Refund a paid order: asks Paddle for a full refund when it is set up, and takes the
   * cosmetics back at once (only those no other paid order, code or staff grant covers).
   * `already_refunded` when Paddle says it was refunded already; `refund_in_progress` while
   * another refund of it runs (it may be tried again after 10 minutes); `refund_unavailable`
   * when the order has a Paddle transaction and Paddle isn't set up. Needs `shop.orders`.
   * Audited.
   */
  refundOrder: Order;
  /** Reject a pending emote with a reason. Needs `emotes.review`. Audited. */
  rejectEmote: Emote;
  /**
   * Remove an account editor: the owner, an editor with `EDITORS`, or the editor themselves
   * (with `ownerId` set to the account they leave). False when they weren't one. Audited.
   */
  removeAccountEditor: Scalars['Boolean']['output'];
  /** Remove an editor. Owner, or the editor themselves. */
  removeSetEditor: Scalars['Boolean']['output'];
  /** Remove an emote from a set. */
  removeSetEmote: Scalars['Boolean']['output'];
  /** Report an emote you can see, with a reason. At most 10 an hour, one open report per emote. */
  reportEmote: Report;
  /**
   * Dismiss a report, or take its emote down (`emotes.takedown`). Needs `emotes.review` or
   * `emotes.takedown`. Audited.
   */
  resolveReport: Report;
  /**
   * Undo `deleteAccount` within 30 days, signed in with the cookie that signing in to the
   * deleted account gave (the website's restore page). Everything shows again and the session
   * works as before. `signed_out` without such a session; `nothing_to_restore` when the window
   * has passed. Audited.
   */
  restoreAccount: Scalars['Boolean']['output'];
  /**
   * Block an account's uploads, or the whole account, for good and without a reason (see
   * `placeRestriction` for timed ones). Needs `users.restrict`, or `users.restrict_uploads` /
   * `users.block` for the one you change; the owner is protected. Audited.
   */
  restrictAccount: AccountStatus;
  /**
   * Take a badge or paint away; if they showed it, it stops. False when they didn't own it.
   * Needs `cosmetics.grant`. Audited.
   */
  revokeCosmetic: Scalars['Boolean']['output'];
  /** Sign out everywhere but here. Returns how many sessions ended. */
  revokeOtherSessions: Scalars['Int']['output'];
  /**
   * Sign out one of your sessions, the current one too (then the cookie is cleared). False
   * when you have none with that ID.
   */
  revokeSession: Scalars['Boolean']['output'];
  /**
   * Pick the badge and paint you show, from what you own (ADR 0015). Leave a field out to keep
   * it; send null or an empty string to show none. With `ownerId`, for an account you edit
   * with `PROFILE`.
   */
  selectCosmetics: OwnedCosmetics;
  /**
   * Start buying an item for yourself, or as a gift with `recipient`. Answers the Paddle
   * transaction to open in Paddle's checkout; the order turns `PAID` once Paddle confirms.
   * `checkout_unavailable` while Paddle isn't set up or the item has no Paddle price. Asking
   * again for the same item and recipient within the hour hands out the same transaction
   * (one pending order each); `checkout_busy` while another request is starting it.
   */
  startCheckout: Checkout;
  /**
   * Take an emote's image down everywhere: out of every set, its file blocked and deleted,
   * its owners told. Needs `emotes.takedown`. Audited.
   */
  takedownEmote: Emote;
  /**
   * Take a role from an account. The owner and the last `roles.manage` holder are protected.
   * Needs `roles.manage` or `roles.assign`. Audited.
   */
  unassignRole: Scalars['Boolean']['output'];
  /**
   * Unlink a platform from your account. The last identity can't be removed, and a
   * restricted or blocked account can't unlink (`restricted`).
   */
  unlinkIdentity: Me;
  /**
   * Change an account editor's permissions. `ownerId` is the account edited; empty for yours.
   * Audited.
   */
  updateAccountEditor: AccountEditor;
  /** Change a badge's name, tooltip or picture. Needs `cosmetics.manage`. Audited. */
  updateBadge: Badge;
  /**
   * Change a channel's active set and settings. The channel's owner, or their account editors
   * with `SETS`.
   */
  updateChannelSettings: Channel;
  /** Change an emote's name, tags or flags. Owner, or `emotes.edit_any`. */
  updateEmote: Emote;
  /** Replace every field of a paint. Needs `cosmetics.manage`. Audited. */
  updatePaint: Paint;
  /**
   * Change a role below your own highest one that you don't hold. The top role can't be
   * changed; the User role keeps position 0. Needs `roles.manage`. Audited.
   */
  updateRole: Role;
  /**
   * Rename a set, change who sees it (`visibility`: owner or account editors with `SETS`), or
   * change its capacity. Owner or editor; capacity needs `sets.edit_any`.
   */
  updateSet: EmoteSet;
  /** Rename an emote in a set, or change its zero-width flag there. */
  updateSetEmote: SetEmote;
  /**
   * Replace every field of a shop item. Orders made already keep their price. Needs
   * `shop.manage`. Audited.
   */
  updateShopItem: ShopItem;
};


export type MutationAddAccountEditorArgs = {
  input: AddAccountEditorInput;
};


export type MutationAddSetEditorArgs = {
  accountId: Scalars['String']['input'];
  setId: Scalars['String']['input'];
};


export type MutationAddSetEmoteArgs = {
  emoteId: Scalars['String']['input'];
  input?: InputMaybe<AddSetEmoteInput>;
  setId: Scalars['String']['input'];
};


export type MutationApproveEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationAssignRoleArgs = {
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
};


export type MutationCloneSetArgs = {
  id: Scalars['String']['input'];
  name?: InputMaybe<Scalars['String']['input']>;
  ownerId?: InputMaybe<Scalars['String']['input']>;
  visibility?: InputMaybe<SetVisibility>;
};


export type MutationCreateBadgeArgs = {
  input: CreateBadgeInput;
};


export type MutationCreateCodeBatchArgs = {
  input: CodeBatchInput;
};


export type MutationCreatePaintArgs = {
  input: PaintInput;
};


export type MutationCreateRoleArgs = {
  input: CreateRoleInput;
};


export type MutationCreateSetArgs = {
  input: CreateSetInput;
};


export type MutationCreateShopItemArgs = {
  input: ShopItemInput;
};


export type MutationDeleteAccountArgs = {
  confirm: Scalars['String']['input'];
};


export type MutationDeleteBadgeArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteEmoteArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeletePaintArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteRoleArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteSetArgs = {
  id: Scalars['String']['input'];
};


export type MutationDeleteShopItemArgs = {
  id: Scalars['String']['input'];
};


export type MutationDisableCodeBatchArgs = {
  id: Scalars['String']['input'];
};


export type MutationGrantCosmeticArgs = {
  input: GrantCosmeticInput;
};


export type MutationLiftRestrictionArgs = {
  accountId: Scalars['String']['input'];
  kind: RestrictionKind;
  reason?: InputMaybe<Scalars['String']['input']>;
};


export type MutationMarkNotificationsReadArgs = {
  ids?: InputMaybe<Array<Scalars['String']['input']>>;
};


export type MutationPlaceRestrictionArgs = {
  accountId: Scalars['String']['input'];
  input: RestrictionInput;
};


export type MutationRedeemCodeArgs = {
  code: Scalars['String']['input'];
};


export type MutationRefundOrderArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationRejectEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationRemoveAccountEditorArgs = {
  accountId: Scalars['String']['input'];
  ownerId?: InputMaybe<Scalars['String']['input']>;
};


export type MutationRemoveSetEditorArgs = {
  accountId: Scalars['String']['input'];
  setId: Scalars['String']['input'];
};


export type MutationRemoveSetEmoteArgs = {
  emoteId: Scalars['String']['input'];
  setId: Scalars['String']['input'];
};


export type MutationReportEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationResolveReportArgs = {
  action: ReportAction;
  id: Scalars['String']['input'];
  note?: InputMaybe<Scalars['String']['input']>;
};


export type MutationRestrictAccountArgs = {
  accountId: Scalars['String']['input'];
  blocked?: InputMaybe<Scalars['Boolean']['input']>;
  uploadsBlocked?: InputMaybe<Scalars['Boolean']['input']>;
};


export type MutationRevokeCosmeticArgs = {
  accountId: Scalars['String']['input'];
  cosmeticId: Scalars['String']['input'];
  kind: CosmeticKind;
  reason?: InputMaybe<Scalars['String']['input']>;
};


export type MutationRevokeSessionArgs = {
  id: Scalars['String']['input'];
};


export type MutationSelectCosmeticsArgs = {
  badgeId?: InputMaybe<Scalars['String']['input']>;
  ownerId?: InputMaybe<Scalars['String']['input']>;
  paintId?: InputMaybe<Scalars['String']['input']>;
};


export type MutationStartCheckoutArgs = {
  itemId: Scalars['String']['input'];
  recipient?: InputMaybe<RecipientInput>;
};


export type MutationTakedownEmoteArgs = {
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
};


export type MutationUnassignRoleArgs = {
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
};


export type MutationUnlinkIdentityArgs = {
  platform: Platform;
};


export type MutationUpdateAccountEditorArgs = {
  accountId: Scalars['String']['input'];
  ownerId?: InputMaybe<Scalars['String']['input']>;
  permissions: Array<EditorPermission>;
};


export type MutationUpdateBadgeArgs = {
  id: Scalars['String']['input'];
  input: UpdateBadgeInput;
};


export type MutationUpdateChannelSettingsArgs = {
  id: Scalars['String']['input'];
  input: UpdateChannelSettingsInput;
  platform: Platform;
};


export type MutationUpdateEmoteArgs = {
  id: Scalars['String']['input'];
  input: UpdateEmoteInput;
};


export type MutationUpdatePaintArgs = {
  id: Scalars['String']['input'];
  input: PaintInput;
};


export type MutationUpdateRoleArgs = {
  id: Scalars['String']['input'];
  input: UpdateRoleInput;
};


export type MutationUpdateSetArgs = {
  id: Scalars['String']['input'];
  input: UpdateSetInput;
};


export type MutationUpdateSetEmoteArgs = {
  emoteId: Scalars['String']['input'];
  input: UpdateSetEmoteInput;
  setId: Scalars['String']['input'];
};


export type MutationUpdateShopItemArgs = {
  id: Scalars['String']['input'];
  input: ShopItemInput;
};

/** Something the platform tells you, such as a takedown of your emote. */
export type Notification = {
  __typename?: 'Notification';
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  /** For example `emote.taken_down`. */
  kind: Scalars['String']['output'];
  /** JSON, as text. */
  payload: Scalars['String']['output'];
  read: Scalars['Boolean']['output'];
};

export type Order = {
  __typename?: 'Order';
  /**
   * The item's price when the checkout started, in cents. 0 when a gift's recipient (not its
   * buyer) looks at it.
   */
  amountCents: Scalars['Int']['output'];
  /** Who paid. None once their account is gone. */
  buyer?: Maybe<AccountSummary>;
  createdAt: Scalars['String']['output'];
  currency: Scalars['String']['output'];
  failedAt?: Maybe<Scalars['String']['output']>;
  /** Bought for another account (decided at checkout; stays when either account is deleted). */
  gift: Scalars['Boolean']['output'];
  id: Scalars['String']['output'];
  itemId: Scalars['String']['output'];
  /** The item's name when it was bought. */
  itemName: Scalars['String']['output'];
  paidAt?: Maybe<Scalars['String']['output']>;
  paidCurrency?: Maybe<Scalars['String']['output']>;
  /**
   * What Paddle says was paid, tax included, in the lowest unit. Set once paid; null for a
   * gift's recipient, like `paidCurrency` and `providerTransactionId`.
   */
  paidTotalCents?: Maybe<Scalars['Int']['output']>;
  /** `paddle`. */
  provider: Scalars['String']['output'];
  /** Paddle's transaction ID (`txn_…`). */
  providerTransactionId?: Maybe<Scalars['String']['output']>;
  /** Who got the cosmetics. */
  recipient?: Maybe<AccountSummary>;
  refundReason?: Maybe<Scalars['String']['output']>;
  /**
   * When Paddle rejected this order's refund after the cosmetics were taken back (RFC 3339):
   * look at it by hand. Only for holders of `shop.orders`.
   */
  refundRejectedAt?: Maybe<Scalars['String']['output']>;
  refundedAt?: Maybe<Scalars['String']['output']>;
  status: OrderStatus;
};

export type OrderPage = {
  __typename?: 'OrderPage';
  items: Array<Order>;
  total: Scalars['Int']['output'];
};

export type OrderStatus =
  /**
   * Cancelled at Paddle, not started there, or not paid within a day (a late payment is still
   * taken).
   */
  | 'FAILED'
  /** Paid; the recipient owns the cosmetics. Also while a staff refund is being sent to Paddle. */
  | 'PAID'
  /** Checkout started, not paid yet. */
  | 'PENDING'
  /** Refunded or charged back; the cosmetics were taken back. */
  | 'REFUNDED';

/** One of your own roles, as `me.roles` lists it. */
export type OwnRole = {
  __typename?: 'OwnRole';
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  /** Shown on your public profile. When false, only you see it. */
  public: Scalars['Boolean']['output'];
};

/** An account's cosmetics: what it owns and what it shows. */
export type OwnedCosmetics = {
  __typename?: 'OwnedCosmetics';
  account: AccountSummary;
  /** The badge shown, one of `badges`. */
  badgeId?: Maybe<Scalars['String']['output']>;
  badges: Array<Badge>;
  /** The paint shown, one of `paints`. */
  paintId?: Maybe<Scalars['String']['output']>;
  paints: Array<Paint>;
};

/** A name paint: a gradient or a picture drawn into the chatter's name. */
export type Paint = {
  __typename?: 'Paint';
  /** Degrees, 0 to 360, CSS convention (0 points up, 90 to the right). Linear only. */
  angle: Scalars['Int']['output'];
  /** Shown where a client can't draw the paint. */
  color?: Maybe<Scalars['String']['output']>;
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  /** Image paints only. 64 px tall at 1x. */
  image?: Maybe<CosmeticImage>;
  kind: PaintKind;
  name: Scalars['String']['output'];
  /** `repeating-*-gradient`, or a tiled image. */
  repeat: Scalars['Boolean']['output'];
  /** At most 4. */
  shadows: Array<PaintShadow>;
  /** Radial only. */
  shape: PaintShape;
  /** 2 to 10 for gradients; none for image paints. */
  stops: Array<PaintStop>;
  updatedAt: Scalars['String']['output'];
};

/** A whole paint: `createPaint` makes one, `updatePaint` replaces every field. */
export type PaintInput = {
  /** 0 to 360, default 90. */
  angle?: InputMaybe<Scalars['Int']['input']>;
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color?: InputMaybe<Scalars['String']['input']>;
  /** A ready paint image; `IMAGE` paints only. */
  imageId?: InputMaybe<Scalars['String']['input']>;
  kind: PaintKind;
  /** 1 to 32 characters, unique. */
  name: Scalars['String']['input'];
  /** Default false. */
  repeat?: InputMaybe<Scalars['Boolean']['input']>;
  /** At most 4. */
  shadows?: Array<PaintShadowInput>;
  /** Default `CIRCLE`. */
  shape?: InputMaybe<PaintShape>;
  /** 2 to 10 for gradients, none for `IMAGE`. */
  stops?: Array<PaintStopInput>;
};

/** How a paint is drawn. */
export type PaintKind =
  /** The paint's image, drawn into the name's text. */
  | 'IMAGE'
  /** `linear-gradient(<angle>deg, <stops>)`. */
  | 'LINEAR_GRADIENT'
  /** `radial-gradient(<shape>, <stops>)`. */
  | 'RADIAL_GRADIENT';

/** A drop shadow under a painted name (CSS `drop-shadow(x y radius color)`, in px). */
export type PaintShadow = {
  __typename?: 'PaintShadow';
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['output'];
  /** Blur radius, 0 to 16. */
  radius: Scalars['Float']['output'];
  /** −16 to 16. */
  x: Scalars['Float']['output'];
  /** −16 to 16. */
  y: Scalars['Float']['output'];
};

/** A drop shadow under a painted name (CSS `drop-shadow(x y radius color)`, in px). */
export type PaintShadowInput = {
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['input'];
  /** Blur radius, 0 to 16. */
  radius: Scalars['Float']['input'];
  /** −16 to 16. */
  x: Scalars['Float']['input'];
  /** −16 to 16. */
  y: Scalars['Float']['input'];
};

/** The shape of a radial paint. */
export type PaintShape =
  | 'CIRCLE'
  | 'ELLIPSE';

/** One colour stop of a gradient paint. */
export type PaintStop = {
  __typename?: 'PaintStop';
  /** Where along the gradient, 0 to 1. Each stop is at or after the one before. */
  at: Scalars['Float']['output'];
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['output'];
};

/** One colour stop of a gradient paint. */
export type PaintStopInput = {
  /** Where along the gradient, 0 to 1. Each stop is at or after the one before. */
  at: Scalars['Float']['input'];
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: Scalars['String']['input'];
};

/** A deleted account that can still be restored (ADR 0019), for the page signing in to it opens. */
export type PendingDeletion = {
  __typename?: 'PendingDeletion';
  avatar?: Maybe<Scalars['String']['output']>;
  /** The name and avatar it showed, Twitch first. */
  displayName?: Maybe<Scalars['String']['output']>;
  /** When it will be erased for good, unless restored first. */
  eraseAfter: Scalars['String']['output'];
  /** When `deleteAccount` was called. */
  requestedAt: Scalars['String']['output'];
};

export type PersonalSetEntry = {
  __typename?: 'PersonalSetEntry';
  platformUserId: Scalars['String']['output'];
  set: EmoteSet;
};

/** A streaming platform. */
export type Platform =
  | 'KICK'
  | 'TWITCH';

/** A linked Twitch or Kick account as anyone may see it: no IDs. */
export type PublicIdentity = {
  __typename?: 'PublicIdentity';
  displayName: Scalars['String']['output'];
  login: Scalars['String']['output'];
  platform: Platform;
};

/** A role as anyone may see it on a profile. */
export type PublicRole = {
  __typename?: 'PublicRole';
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
};

export type Query = {
  __typename?: 'Query';
  /** An account's public summary. Null when the account is unknown, deleted or blocked. */
  account?: Maybe<AccountSummary>;
  /**
   * An account's public activity, newest first, like 7TV's "Activity" tab: sets and emotes the
   * owner or their editors changed, the active set, badge and paint picks, approved uploads.
   * Page back with `before` (an entry ID); `limit` is 1 to 100, default 30. Empty when the
   * account is unknown, deleted or blocked. Nothing private or from moderation shows. A page
   * can be shorter than `limit` and still not the last: use `accountActivityPage` to know.
   * A heavy read: 120 a minute per account, or per IP signed out (`rate_limited`).
   */
  accountActivity: Array<ActivityEntry>;
  /**
   * `accountActivity` with a cursor: `next` is the `before` for the following page, null when
   * there is none. A page can hold fewer than `limit` entries and still have a `next`, because
   * the feed skips what isn't public.
   */
  accountActivityPage: ActivityPage;
  /**
   * The badge and paint up to 100 accounts show, with the badges and paints themselves, by
   * cyex.tv account ID (for names on profiles, tiles and lists). Only accounts showing
   * something are listed; blocked and deleted ones never. `too_many` over 100.
   */
  accountCosmetics: Array<AccountCosmetics>;
  /**
   * An account's editors (ADR 0014): yours, or, with `accountId`, those of an account you edit
   * with the `EDITORS` permission.
   */
  accountEditors: Array<AccountEditor>;
  /**
   * An account's emotes in every state, newest first. Needs `users.view`, `emotes.review`
   * or `emotes.takedown`. `perPage` is 1 to 100, default 50.
   */
  accountEmotes: EmotePage;
  /**
   * An account's public profile: linked Twitch and Kick logins (no IDs), roles marked public,
   * and each channel's active set ("Active emotes"). Listed sets: `accountSets`. Null when the
   * account is unknown, deleted or blocked.
   */
  accountProfile?: Maybe<AccountProfile>;
  /**
   * Reports about an account's emotes and the ones it filed, newest first, at most 50 each.
   * Needs `emotes.review` or `emotes.takedown`.
   */
  accountReports: AccountReports;
  /**
   * Every restriction placed on an account, newest first (the latest 100), running or over,
   * with its reason and end. Needs `users.view`, `users.restrict` or a narrower restrict
   * permission.
   */
  accountRestrictions: Array<Restriction>;
  /** An account's roles. Yours, or anyone's with `roles.manage` or `users.view`. */
  accountRoles: Array<Role>;
  /**
   * An account's `LISTED` sets as profile cards, oldest first, at most 50. `preview` is how
   * many of each set's newest emotes to include, 0 to 50, default 12. Empty when the account
   * is unknown, deleted or blocked. Counts as a heavy read: 120 a minute per account, or per IP
   * signed out (`rate_limited`), and a high query cost.
   */
  accountSets: Array<SetSummary>;
  /**
   * What an account may do now: uploads, reports and sign-in, each with its end when timed.
   * Needs `users.view`, `users.restrict` or a narrower restrict permission.
   */
  accountStatus: AccountStatus;
  /**
   * An account as staff see it: identities, created time and restrictions. Needs
   * `users.view` or `users.restrict`.
   */
  adminAccount: AdminAccount;
  /**
   * Audited changes, newest first. Needs `audit.view`. Narrow by target, `actorId`, an exact
   * `action`, and `since` (inclusive) / `until` (exclusive) as RFC 3339 times. Page back with
   * `before` (an entry ID); `limit` is 1 to 100, default 50.
   */
  auditLog: Array<AuditEntry>;
  /** A channel by its platform ID. */
  channel?: Maybe<Channel>;
  /** A channel by its login or slug. Logins change; store the ID, not the login. */
  channelByLogin?: Maybe<Channel>;
  /**
   * Redeem code batches, newest first. Needs `codes.manage`. The codes themselves are shown
   * only once, by `createCodeBatch`.
   */
  codeBatches: CodeBatchPage;
  /**
   * Who owns which badges or paints, newest first; narrow it with `accountId` and/or
   * `cosmeticId`. `perPage` is 1 to 100, default 50. Needs `cosmetics.grant`.
   */
  cosmeticEntitlements: EntitlementPage;
  /**
   * A badge or paint picture, to see whether its files are made (`ready`). Needs
   * `cosmetics.manage`.
   */
  cosmeticImage?: Maybe<CosmeticImage>;
  /**
   * Every cyex.tv chat badge and name paint (ADR 0015). Public; clients cache it
   * (`GET /v1/cosmetics` has an ETag) and follow the `cosmetics` topic.
   */
  cosmetics: CosmeticCatalog;
  /** The channels of the accounts you edit. */
  editorChannels: Array<Channel>;
  /** The accounts you edit, with what you may do on each. */
  editorOf: Array<EditedAccount>;
  /** The sets of the accounts you edit. Each set names its owner. */
  editorSets: Array<EmoteSet>;
  /** An emote. Pending ones are visible to their owner and to moderators only. */
  emote?: Maybe<Emote>;
  /**
   * An emote's public activity, newest first: `EMOTE_UPLOADED` (when it was approved, shown
   * as by its owner) and `SET_EMOTE_ADDED`, `SET_EMOTE_REMOVED`, `SET_EMOTE_RENAMED` in listed
   * sets, by their owners and account editors. Page back with `before` (an entry ID); `limit`
   * is 1 to 100, default 30. Empty when the emote is unknown or not public. Nothing private or
   * from moderation shows. A page can be shorter than `limit` and still not the last: use
   * `emoteActivityPage` to know. A heavy read: 120 a minute per account, or per IP signed out
   * (`rate_limited`).
   */
  emoteActivity: Array<ActivityEntry>;
  /**
   * `emoteActivity` with a cursor: `next` is the `before` for the following page, null when
   * there is none. A page can hold fewer than `limit` entries and still have a `next`.
   */
  emoteActivityPage: ActivityPage;
  /**
   * Channels whose active set holds the emote, most recently added first. `limit` is 1 to 50,
   * default 8. Empty when the emote is unknown or not public.
   */
  emoteChannels: Array<Channel>;
  /** The cyex.tv global set, shown in every channel. */
  globalSet: EmoteSet;
  /** The signed-in account. */
  me: Me;
  /** Your channels: one per linked platform you have signed in with. */
  myChannels: Array<Channel>;
  /**
   * The cosmetics you own and show; with `accountId`, those of an account you edit with
   * `PROFILE`.
   */
  myCosmetics: OwnedCosmetics;
  /**
   * Channels you can manage: yours (`OWNER`), those of accounts you are an account editor of
   * (`ACCOUNT_EDITOR`), and those of accounts where you edit at least one set (`SET_EDITOR`).
   * Each channel once, under your highest role; sorted by role (`OWNER`, `ACCOUNT_EDITOR`,
   * `SET_EDITOR`), then login (ignoring case), then platform. Signed-in only.
   */
  myEditableChannels: Array<EditableChannel>;
  /** Your own emotes in every state, newest first. `perPage` is 1 to 100, default 50. */
  myEmotes: EmotePage;
  /**
   * Your notifications, newest first (the latest 100). `kind` is one of `emote.approved`,
   * `emote.rejected`, `emote.taken_down`, `editor.added`, `editor.changed`, `editor.removed`,
   * `cosmetic.granted` and `cosmetic.revoked`; `payload` is JSON with the IDs and names it is
   * about (see the API reference). Cosmetics from the shop and from codes carry `source`
   * (`purchase` or `code`), and from orders `order_id`.
   */
  myNotifications: Array<Notification>;
  /**
   * Orders you bought, and gifts you got once paid (without what the buyer paid), newest
   * first. `perPage` 1 to 100, default 50.
   */
  myOrders: OrderPage;
  /**
   * Where you are signed in: your live sessions, the most recently used first, with the one
   * this request came with marked `current`. Never shows tokens.
   */
  mySessions: Array<AccountSession>;
  /** The sets you own, then the ones you edit. Pending emotes included. */
  mySets: Array<EmoteSet>;
  /**
   * One order, for its buyer, its recipient (once paid, without what the buyer paid), or
   * `shop.orders`. Poll it after `startCheckout` until it is `PAID`.
   */
  order?: Maybe<Order>;
  /**
   * Every order, newest first; narrow by status, account (bought or received) and item.
   * Needs `shop.orders`.
   */
  orders: OrderPage;
  /**
   * The deletion you can still undo: set only for the cookie session that signing in to a
   * deleted account within its 30 days gave (`restoreAccount` undoes it). Null otherwise,
   * including for every normal session.
   */
  pendingDeletion?: Maybe<PendingDeletion>;
  /** Every permission name a role can hold. */
  permissions: Array<Scalars['String']['output']>;
  /** One user's personal set. */
  personalSet?: Maybe<EmoteSet>;
  /** Personal sets for up to 100 chatters at once. */
  personalSets: Array<PersonalSetEntry>;
  /**
   * The header search: the top 5 emotes for `query` by the name matcher (exact name, same
   * ignoring case, name prefix, word or initials prefix such as `point` or `pp` for
   * `PepePoint`, substring, then similar names for 3+ characters; most used first within a
   * tier), and the top 3 accounts (`searchAccounts` ranking, a leading `@` ignored, under 2
   * characters finds none). Approved, listed, non-NSFW emotes only. 240 searches a minute per
   * account, or per IP signed out (`rate_limited`).
   */
  quickSearch: QuickSearch;
  /** Open reports, oldest first. Needs `emotes.review` or `emotes.takedown`. */
  reportQueue: ReportQueue;
  /** Pending emotes, oldest first. Needs `emotes.review`. `perPage` is 1 to 100, default 50. */
  reviewQueue: ReviewQueue;
  /**
   * All roles, most senior first, each with whether you may edit or assign it. Needs
   * `roles.manage` or `roles.assign`. The top role is listed to the owners only.
   */
  roles: Array<Role>;
  /**
   * Accounts by Twitch or Kick login or display name (a leading `@` is ignored): exact login
   * first, then login and name prefixes, then similar names. Public fields only; deleted and
   * blocked accounts are never listed. `perPage` is 1 to 50, default 20. 60 searches a minute
   * per account, or per IP signed out (`rate_limited`). A `query` under 2 characters (after the
   * `@`) finds nobody. Matches are counted up to 1000: `totalCapped` says there are more, and
   * the pages end there.
   */
  searchAccounts: AccountResults;
  /**
   * Approved, listed emotes by name and tag, best match first, then the most used.
   * `perPage` is 1 to 100, default 50. NSFW emotes only with `nsfw: true`. `sort` is
   * `RELEVANCE` (default: best match, then most used), `TOP` (most used, then name) or `NEW`
   * (newest first). An empty `query` lists every approved, listed emote.
   */
  searchEmotes: SearchResults;
  /**
   * A set with its emotes. Null when there is none, or when it is `PRIVATE` and you are not
   * its owner, an editor or staff (a channel's active set is always public).
   */
  set?: Maybe<EmoteSet>;
  /** A set's editors. For its owner, its editors and `sets.edit_any`. */
  setEditors: Array<AccountSummary>;
  /** One shop item: on sale for everyone; any, deleted ones too, with `shop.manage`. */
  shopItem?: Maybe<ShopItem>;
  /**
   * Shop items on sale now, by sort order (ADR 0016). Public; signed in, `owned` says whether
   * you own everything in each. With `all: true`, every item that isn't deleted, hidden and
   * scheduled ones too (needs `shop.manage`).
   */
  shopItems: Array<ShopItem>;
  /** How many of your notifications are unread. */
  unreadNotificationCount: Scalars['Int']['output'];
  /**
   * The badge and paint up to 100 chatters show, as catalog IDs. Only chatters showing
   * something are listed.
   */
  userCosmetics: Array<UserCosmetics>;
};


export type QueryAccountArgs = {
  id: Scalars['String']['input'];
};


export type QueryAccountActivityArgs = {
  accountId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountActivityPageArgs = {
  accountId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountCosmeticsArgs = {
  accountIds: Array<Scalars['String']['input']>;
};


export type QueryAccountEditorsArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryAccountEmotesArgs = {
  accountId: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountProfileArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountReportsArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountRestrictionsArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountRolesArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAccountSetsArgs = {
  accountId: Scalars['String']['input'];
  preview?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryAccountStatusArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAdminAccountArgs = {
  accountId: Scalars['String']['input'];
};


export type QueryAuditLogArgs = {
  action?: InputMaybe<Scalars['String']['input']>;
  actorId?: InputMaybe<Scalars['String']['input']>;
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
  since?: InputMaybe<Scalars['String']['input']>;
  targetId?: InputMaybe<Scalars['String']['input']>;
  targetKind?: InputMaybe<Scalars['String']['input']>;
  until?: InputMaybe<Scalars['String']['input']>;
};


export type QueryChannelArgs = {
  id: Scalars['String']['input'];
  platform: Platform;
};


export type QueryChannelByLoginArgs = {
  login: Scalars['String']['input'];
  platform: Platform;
};


export type QueryCodeBatchesArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryCosmeticEntitlementsArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
  cosmeticId?: InputMaybe<Scalars['String']['input']>;
  kind: CosmeticKind;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryCosmeticImageArgs = {
  id: Scalars['String']['input'];
};


export type QueryEmoteArgs = {
  id: Scalars['String']['input'];
};


export type QueryEmoteActivityArgs = {
  before?: InputMaybe<Scalars['String']['input']>;
  emoteId: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryEmoteActivityPageArgs = {
  before?: InputMaybe<Scalars['String']['input']>;
  emoteId: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryEmoteChannelsArgs = {
  id: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryMyCosmeticsArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
};


export type QueryMyEmotesArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryMyNotificationsArgs = {
  unreadOnly?: InputMaybe<Scalars['Boolean']['input']>;
};


export type QueryMyOrdersArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryOrderArgs = {
  id: Scalars['String']['input'];
};


export type QueryOrdersArgs = {
  accountId?: InputMaybe<Scalars['String']['input']>;
  itemId?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  status?: InputMaybe<OrderStatus>;
};


export type QueryPersonalSetArgs = {
  id: Scalars['String']['input'];
  platform: Platform;
};


export type QueryPersonalSetsArgs = {
  platform: Platform;
  userIds: Array<Scalars['String']['input']>;
};


export type QueryQuickSearchArgs = {
  query: Scalars['String']['input'];
};


export type QueryReportQueueArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QueryReviewQueueArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
};


export type QuerySearchAccountsArgs = {
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  query: Scalars['String']['input'];
};


export type QuerySearchEmotesArgs = {
  animated?: InputMaybe<Scalars['Boolean']['input']>;
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  owner?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  query: Scalars['String']['input'];
  sort?: InputMaybe<EmoteSort>;
};


export type QuerySetArgs = {
  id: Scalars['String']['input'];
};


export type QuerySetEditorsArgs = {
  setId: Scalars['String']['input'];
};


export type QueryShopItemArgs = {
  id: Scalars['String']['input'];
};


export type QueryShopItemsArgs = {
  all?: InputMaybe<Scalars['Boolean']['input']>;
};


export type QueryUserCosmeticsArgs = {
  platform: Platform;
  userIds: Array<Scalars['String']['input']>;
};

/** The header search: the best emotes and accounts for one query. */
export type QuickSearch = {
  __typename?: 'QuickSearch';
  emotes: Array<Emote>;
  users: Array<AccountHit>;
};

/** Who gets a gift: their Twitch or Kick login. */
export type RecipientInput = {
  login: Scalars['String']['input'];
  platform: Platform;
};

/** What a redeemed code gave. */
export type RedeemResult = {
  __typename?: 'RedeemResult';
  /** Newly owned. */
  badges: Array<Badge>;
  /** The batch's name. */
  name: Scalars['String']['output'];
  paints: Array<Paint>;
};

export type Report = {
  __typename?: 'Report';
  createdAt: Scalars['String']['output'];
  /** The reported emote, as moderators see it. Empty if it was deleted since. */
  emote?: Maybe<Emote>;
  id: Scalars['String']['output'];
  reason: Scalars['String']['output'];
  reporterId?: Maybe<Scalars['String']['output']>;
  resolution?: Maybe<Scalars['String']['output']>;
  resolvedAt?: Maybe<Scalars['String']['output']>;
  state: ReportState;
};

/** What a moderator does with a report. */
export type ReportAction =
  | 'DISMISS'
  /** Take the emote down (needs `emotes.takedown`). */
  | 'TAKE_DOWN';

export type ReportQueue = {
  __typename?: 'ReportQueue';
  items: Array<Report>;
  total: Scalars['Int']['output'];
};

export type ReportState =
  /** The emote was taken down. */
  | 'ACTIONED'
  | 'DISMISSED'
  | 'OPEN';

/** One restriction placed on an account, running or over. */
export type Restriction = {
  __typename?: 'Restriction';
  /** Still holding now. */
  active: Scalars['Boolean']['output'];
  createdAt: Scalars['String']['output'];
  /** The staff account that placed it. */
  createdBy?: Maybe<Scalars['String']['output']>;
  /** When it ends; empty for good. */
  endsAt?: Maybe<Scalars['String']['output']>;
  id: Scalars['String']['output'];
  kind: RestrictionKind;
  /** When staff lifted it early, or a newer restriction of the same kind replaced it. */
  liftedAt?: Maybe<Scalars['String']['output']>;
  liftedBy?: Maybe<Scalars['String']['output']>;
  /** Why, as staff wrote it. Empty on restrictions made without one (`restrictAccount`). */
  reason?: Maybe<Scalars['String']['output']>;
};

/** A restriction to place. */
export type RestrictionInput = {
  kind: RestrictionKind;
  /** Why: 1 to 500 characters. Staff see it; the account doesn't. */
  reason: Scalars['String']['input'];
  /** When it ends (RFC 3339, in the future, at most ten years out); empty for good. */
  until?: InputMaybe<Scalars['String']['input']>;
};

/** What a restriction stops. */
export type RestrictionKind =
  /** The whole account: no sign-in, sessions ended (`users.block`). */
  | 'ACCOUNT'
  /** Filing reports and tickets (`users.restrict_tickets`). */
  | 'TICKETS'
  /** Uploading emotes (`users.restrict_uploads`). */
  | 'UPLOADS';

export type ReviewQueue = {
  __typename?: 'ReviewQueue';
  items: Array<Emote>;
  total: Scalars['Int']['output'];
};

export type Role = {
  __typename?: 'Role';
  /**
   * Whether you may give it to someone (and take it away): below your highest role, you hold
   * what it grants, and not the User or top role.
   */
  assignable: Scalars['Boolean']['output'];
  /**
   * Set on the built-in roles: `USER` (every account holds it), `ADMIN` and `ROOT` (the
   * owners' top role, shown only to its holders).
   */
  builtIn?: Maybe<BuiltInRole>;
  description?: Maybe<Scalars['String']['output']>;
  /** Whether you may change it: below your highest role, not one you hold, not the top role. */
  editable: Scalars['Boolean']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  permissions: Array<Scalars['String']['output']>;
  /** Higher means more senior. You can only manage roles below your own highest one. */
  position: Scalars['Int']['output'];
  /** Shown on its holders' public profiles (`accountProfile`). Off by default. */
  public: Scalars['Boolean']['output'];
};

export type SearchResults = {
  __typename?: 'SearchResults';
  items: Array<Emote>;
  page: Scalars['Int']['output'];
  perPage: Scalars['Int']['output'];
  total: Scalars['Int']['output'];
};

/** How a session signed in. */
export type SessionKind =
  /** A Bearer token for an app (cyex deck, the extension). */
  | 'APP'
  /** The cyex.tv website's cookie. */
  | 'SITE';

/** An emote as it sits in a set: the name it is typed as there, and who added it. */
export type SetEmote = {
  __typename?: 'SetEmote';
  addedAt: Scalars['String']['output'];
  addedBy?: Maybe<Scalars['String']['output']>;
  emote: Emote;
  /** The name in this set. Differs from `emote.name` when the set renamed it. */
  name: Scalars['String']['output'];
  zeroWidth: Scalars['Boolean']['output'];
};

export type SetKind =
  | 'CHANNEL'
  | 'GLOBAL'
  | 'PERSONAL';

/** A set as a card on a profile: its facts and the first few emotes (`accountSets`). */
export type SetSummary = {
  __typename?: 'SetSummary';
  /** A channel shows this set now. */
  active: Scalars['Boolean']['output'];
  capacity: Scalars['Int']['output'];
  createdAt: Scalars['String']['output'];
  id: Scalars['String']['output'];
  kind: SetKind;
  listed: Scalars['Boolean']['output'];
  name: Scalars['String']['output'];
  owner?: Maybe<AccountSummary>;
  /** The newest emotes you may see, at most the `preview` asked for. */
  preview: Array<SetEmote>;
  /** How many slots the set uses, as counted for you; see `EmoteSet.slotsUsed`. */
  slotsUsed: Scalars['Int']['output'];
  visibility: SetVisibility;
};

/** Who can find and open a set. */
export type SetVisibility =
  /** Anyone can open it, and it is listed on its owner's profile. New channel sets start here. */
  | 'LISTED'
  /**
   * Only its owner, its editors and staff can open it, except while a channel shows it:
   * a channel's active set is public. Channel sets only.
   */
  | 'PRIVATE'
  /** Anyone with its link (its ID) can open it; it isn't listed. Personal sets start here. */
  | 'UNLISTED';

/** Something staff sell: one or more badges and paints as a bundle. */
export type ShopItem = {
  __typename?: 'ShopItem';
  /** On sale from (RFC 3339); none: already. */
  activeFrom?: Maybe<Scalars['String']['output']>;
  /** On sale until (RFC 3339); none: no end. */
  activeTo?: Maybe<Scalars['String']['output']>;
  /** Can be bought now: visible, on sale, in stock, has cosmetics. */
  available: Scalars['Boolean']['output'];
  badges: Array<Badge>;
  createdAt: Scalars['String']['output'];
  /** ISO 4217, for example `EUR`. */
  currency: Scalars['String']['output'];
  /** Deleted: only staff see it, in lists with `all: true`. */
  deleted: Scalars['Boolean']['output'];
  description: Scalars['String']['output'];
  id: Scalars['String']['output'];
  name: Scalars['String']['output'];
  /** The caller owns every cosmetic in it (always false signed out). They can still gift it. */
  owned: Scalars['Boolean']['output'];
  /** The Paddle price (`pri_…`). Only for holders of `shop.manage`; null for everyone else. */
  paddlePriceId?: Maybe<Scalars['String']['output']>;
  paints: Array<Paint>;
  /** The price shown, in cents. Paddle's price (`paddlePriceId`) is what is charged. */
  priceCents: Scalars['Int']['output'];
  /** Lower first. */
  sortOrder: Scalars['Int']['output'];
  /** Copies left (pending checkouts of the last hour hold one each); none: no limit. */
  stockLeft?: Maybe<Scalars['Int']['output']>;
  /** Most copies sold; none: no limit. */
  stockLimit?: Maybe<Scalars['Int']['output']>;
  updatedAt: Scalars['String']['output'];
  /** Listed in the public shop (still only while on sale). */
  visible: Scalars['Boolean']['output'];
};

/** A whole item: `createShopItem` makes one, `updateShopItem` replaces every field. */
export type ShopItemInput = {
  /** RFC 3339. */
  activeFrom?: InputMaybe<Scalars['String']['input']>;
  /** RFC 3339, after `activeFrom`. */
  activeTo?: InputMaybe<Scalars['String']['input']>;
  /** 1 to 10 cosmetics in all. */
  badgeIds?: Array<Scalars['String']['input']>;
  /** ISO 4217, default `EUR`. */
  currency?: InputMaybe<Scalars['String']['input']>;
  /** Up to 500 characters; new lines allowed. */
  description?: InputMaybe<Scalars['String']['input']>;
  /** 1 to 64 characters. */
  name: Scalars['String']['input'];
  /** The Paddle price (`pri_…`). Without one, checkout says it isn't available. */
  paddlePriceId?: InputMaybe<Scalars['String']['input']>;
  paintIds?: Array<Scalars['String']['input']>;
  /** 0 to 100000 (1000.00). */
  priceCents: Scalars['Int']['input'];
  /** Default 0. */
  sortOrder?: InputMaybe<Scalars['Int']['input']>;
  /** 1 or more; none: no limit. */
  stockLimit?: InputMaybe<Scalars['Int']['input']>;
  /** Default false. */
  visible?: InputMaybe<Scalars['Boolean']['input']>;
};

export type Subscription = {
  __typename?: 'Subscription';
  /**
   * Live updates for up to 500 topics: `channel:<platform>:<id>`, `set:<id>`,
   * `user:<platform>:<id>` (ADR 0005), and `cosmetics` (ADR 0015). The same events as the event WebSocket.
   */
  events: Event;
};


export type SubscriptionEventsArgs = {
  topics: Array<Scalars['String']['input']>;
};

export type UpdateBadgeInput = {
  /** Another ready badge image. */
  imageId?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  tooltip?: InputMaybe<Scalars['String']['input']>;
};

export type UpdateChannelSettingsInput = {
  /** One of the owner's channel sets. An empty string clears it. */
  activeSetId?: InputMaybe<Scalars['String']['input']>;
  allowPersonalEmotes?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateEmoteInput = {
  name?: InputMaybe<Scalars['String']['input']>;
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  /**
   * `true` makes it private; `false` makes a private emote public (an unlisted one stays
   * unlisted). Prefer `visibility`.
   */
  private?: InputMaybe<Scalars['Boolean']['input']>;
  tags?: InputMaybe<Array<Scalars['String']['input']>>;
  /**
   * `PUBLIC`, `UNLISTED` or `PRIVATE`. With `private` too, the two must agree
   * (`bad_visibility`).
   */
  visibility?: InputMaybe<EmoteVisibility>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateRoleInput = {
  /** An empty string clears it. */
  description?: InputMaybe<Scalars['String']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  permissions?: InputMaybe<Array<Scalars['String']['input']>>;
  position?: InputMaybe<Scalars['Int']['input']>;
  /** Show it on its holders' public profiles. */
  public?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateSetEmoteInput = {
  name?: InputMaybe<Scalars['String']['input']>;
  zeroWidth?: InputMaybe<Scalars['Boolean']['input']>;
};

export type UpdateSetInput = {
  /** Needs `sets.edit_any` to raise above the default. */
  capacity?: InputMaybe<Scalars['Int']['input']>;
  name?: InputMaybe<Scalars['String']['input']>;
  /** Personal sets can't be `PRIVATE`; the global set is always listed. */
  visibility?: InputMaybe<SetVisibility>;
};

/** What one chatter shows, as IDs from the catalog. Only chatters showing something are listed. */
export type UserCosmetics = {
  __typename?: 'UserCosmetics';
  badgeId?: Maybe<Scalars['String']['output']>;
  paintId?: Maybe<Scalars['String']['output']>;
  platformUserId: Scalars['String']['output'];
};

export type StudioPaintFieldsFragment = { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null };

export type StudioPaintsQueryVariables = Exact<{ [key: string]: never; }>;


export type StudioPaintsQuery = { __typename?: 'Query', cosmetics: { __typename?: 'CosmeticCatalog', paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null }> } };

export type StudioCreatePaintMutationVariables = Exact<{
  input: PaintInput;
}>;


export type StudioCreatePaintMutation = { __typename?: 'Mutation', createPaint: { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null } };

export type StudioUpdatePaintMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: PaintInput;
}>;


export type StudioUpdatePaintMutation = { __typename?: 'Mutation', updatePaint: { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, updatedAt: string, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null } };

export type StudioCosmeticImageQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type StudioCosmeticImageQuery = { __typename?: 'Query', cosmeticImage?: { __typename?: 'CosmeticImage', id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null };

export type EmoteFieldsFragment = { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> };

export type SearchQueryVariables = Exact<{
  query: Scalars['String']['input'];
  animated?: InputMaybe<Scalars['Boolean']['input']>;
  nsfw?: InputMaybe<Scalars['Boolean']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  sort?: InputMaybe<EmoteSort>;
}>;


export type SearchQuery = { __typename?: 'Query', searchEmotes: { __typename?: 'SearchResults', total: number, page: number, perPage: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type FindPeopleQueryVariables = Exact<{
  query: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type FindPeopleQuery = { __typename?: 'Query', searchAccounts: { __typename?: 'AccountResults', total: number, page: number, perPage: number, items: Array<{ __typename?: 'AccountHit', account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> } };

export type EmoteQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type EmoteQuery = { __typename?: 'Query', emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null };

export type ChannelQueryVariables = Exact<{
  platform: Platform;
  login: Scalars['String']['input'];
}>;


export type ChannelQuery = { __typename?: 'Query', channelByLogin?: { __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, allowPersonalEmotes: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } | null };

export type SetQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type SetQuery = { __typename?: 'Query', set?: { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } | null };

export type PublicSetQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type PublicSetQuery = { __typename?: 'Query', set?: { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, slotsUsed: number, createdAt: string, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } | null };

export type CloneSetMutationVariables = Exact<{
  id: Scalars['String']['input'];
  name?: InputMaybe<Scalars['String']['input']>;
  visibility?: InputMaybe<SetVisibility>;
}>;


export type CloneSetMutation = { __typename?: 'Mutation', cloneSet: { __typename?: 'ClonedSet', copied: number, skipped: number, set: { __typename?: 'EmoteSet', id: string, name: string } } };

export type GlobalSetQueryVariables = Exact<{ [key: string]: never; }>;


export type GlobalSetQuery = { __typename?: 'Query', globalSet: { __typename?: 'EmoteSet', id: string, name: string, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } };

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { __typename?: 'Query', me: { __typename?: 'Me', id: string, createdAt: string, permissions: Array<string>, identities: Array<{ __typename?: 'Identity', platform: Platform, platformUserId: string, login: string, displayName: string, avatar?: string | null }> } };

export type MyEmotesQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type MyEmotesQuery = { __typename?: 'Query', myEmotes: { __typename?: 'EmotePage', total: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type UpdateEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateEmoteInput;
}>;


export type UpdateEmoteMutation = { __typename?: 'Mutation', updateEmote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } };

export type DeleteEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteEmoteMutation = { __typename?: 'Mutation', deleteEmote: boolean };

export type SetFieldsFragment = { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> };

export type MySetsQueryVariables = Exact<{ [key: string]: never; }>;


export type MySetsQuery = { __typename?: 'Query', mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }>, editorSets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string, displayName: string } }> };

export type EditSetQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type EditSetQuery = { __typename?: 'Query', set?: { __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } | null, setEditors: Array<{ __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string } }> };

export type CreateSetMutationVariables = Exact<{
  input: CreateSetInput;
}>;


export type CreateSetMutation = { __typename?: 'Mutation', createSet: { __typename?: 'EmoteSet', id: string } };

export type UpdateSetMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateSetInput;
}>;


export type UpdateSetMutation = { __typename?: 'Mutation', updateSet: { __typename?: 'EmoteSet', id: string, name: string } };

export type DeleteSetMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteSetMutation = { __typename?: 'Mutation', deleteSet: boolean };

export type AddSetEmoteMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  emoteId: Scalars['String']['input'];
  input?: InputMaybe<AddSetEmoteInput>;
}>;


export type AddSetEmoteMutation = { __typename?: 'Mutation', addSetEmote: { __typename?: 'SetEmote', name: string } };

export type UpdateSetEmoteMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  emoteId: Scalars['String']['input'];
  input: UpdateSetEmoteInput;
}>;


export type UpdateSetEmoteMutation = { __typename?: 'Mutation', updateSetEmote: { __typename?: 'SetEmote', name: string, zeroWidth: boolean } };

export type RemoveSetEmoteMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  emoteId: Scalars['String']['input'];
}>;


export type RemoveSetEmoteMutation = { __typename?: 'Mutation', removeSetEmote: boolean };

export type AddSetEditorMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  accountId: Scalars['String']['input'];
}>;


export type AddSetEditorMutation = { __typename?: 'Mutation', addSetEditor: boolean };

export type RemoveSetEditorMutationVariables = Exact<{
  setId: Scalars['String']['input'];
  accountId: Scalars['String']['input'];
}>;


export type RemoveSetEditorMutation = { __typename?: 'Mutation', removeSetEditor: boolean };

export type FindAccountQueryVariables = Exact<{
  platform: Platform;
  login: Scalars['String']['input'];
}>;


export type FindAccountQuery = { __typename?: 'Query', channelByLogin?: { __typename?: 'Channel', login: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } | null };

export type MyChannelsQueryVariables = Exact<{ [key: string]: never; }>;


export type MyChannelsQuery = { __typename?: 'Query', myChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, allowPersonalEmotes: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string } }>, mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, owner?: { __typename?: 'AccountSummary', id: string } | null }>, editorChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, allowPersonalEmotes: boolean, account: { __typename?: 'AccountSummary', id: string, displayName: string } }>, editorSets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, owner?: { __typename?: 'AccountSummary', id: string } | null }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string } }> };

export type UpdateChannelMutationVariables = Exact<{
  platform: Platform;
  id: Scalars['String']['input'];
  input: UpdateChannelSettingsInput;
}>;


export type UpdateChannelMutation = { __typename?: 'Mutation', updateChannelSettings: { __typename?: 'Channel', activeSetId?: string | null, allowPersonalEmotes: boolean } };

export type AccountEditorFieldsFragment = { __typename?: 'AccountEditor', permissions: Array<EditorPermission>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null };

export type EditorsQueryVariables = Exact<{ [key: string]: never; }>;


export type EditorsQuery = { __typename?: 'Query', accountEditors: Array<{ __typename?: 'AccountEditor', permissions: Array<EditorPermission>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null }>, editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } }> };

export type AddAccountEditorMutationVariables = Exact<{
  input: AddAccountEditorInput;
}>;


export type AddAccountEditorMutation = { __typename?: 'Mutation', addAccountEditor: { __typename?: 'AccountEditor', permissions: Array<EditorPermission>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } };

export type UpdateAccountEditorMutationVariables = Exact<{
  ownerId?: InputMaybe<Scalars['String']['input']>;
  accountId: Scalars['String']['input'];
  permissions: Array<EditorPermission> | EditorPermission;
}>;


export type UpdateAccountEditorMutation = { __typename?: 'Mutation', updateAccountEditor: { __typename?: 'AccountEditor', permissions: Array<EditorPermission>, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } };

export type RemoveAccountEditorMutationVariables = Exact<{
  ownerId?: InputMaybe<Scalars['String']['input']>;
  accountId: Scalars['String']['input'];
}>;


export type RemoveAccountEditorMutation = { __typename?: 'Mutation', removeAccountEditor: boolean };

export type UnlinkMutationVariables = Exact<{
  platform: Platform;
}>;


export type UnlinkMutation = { __typename?: 'Mutation', unlinkIdentity: { __typename?: 'Me', id: string } };

export type MyNotificationsQueryVariables = Exact<{ [key: string]: never; }>;


export type MyNotificationsQuery = { __typename?: 'Query', myNotifications: Array<{ __typename?: 'Notification', id: string, kind: string, payload: string, createdAt: string, read: boolean }> };

export type MarkReadMutationVariables = Exact<{
  ids?: InputMaybe<Array<Scalars['String']['input']> | Scalars['String']['input']>;
}>;


export type MarkReadMutation = { __typename?: 'Mutation', markNotificationsRead: number };

export type UnreadCountQueryVariables = Exact<{ [key: string]: never; }>;


export type UnreadCountQuery = { __typename?: 'Query', unreadNotificationCount: number };

export type MySessionsQueryVariables = Exact<{ [key: string]: never; }>;


export type MySessionsQuery = { __typename?: 'Query', mySessions: Array<{ __typename?: 'AccountSession', id: string, kind: SessionKind, device: string, createdAt: string, lastSeenAt: string, expiresAt: string, current: boolean }> };

export type RevokeSessionMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type RevokeSessionMutation = { __typename?: 'Mutation', revokeSession: boolean };

export type RevokeOtherSessionsMutationVariables = Exact<{ [key: string]: never; }>;


export type RevokeOtherSessionsMutation = { __typename?: 'Mutation', revokeOtherSessions: number };

export type ExportMyDataMutationVariables = Exact<{ [key: string]: never; }>;


export type ExportMyDataMutation = { __typename?: 'Mutation', exportMyData: string };

export type DeleteAccountMutationVariables = Exact<{
  confirm: Scalars['String']['input'];
}>;


export type DeleteAccountMutation = { __typename?: 'Mutation', deleteAccount: boolean };

export type PendingDeletionQueryVariables = Exact<{ [key: string]: never; }>;


export type PendingDeletionQuery = { __typename?: 'Query', pendingDeletion?: { __typename?: 'PendingDeletion', requestedAt: string, eraseAfter: string, displayName?: string | null, avatar?: string | null } | null };

export type RestoreAccountMutationVariables = Exact<{ [key: string]: never; }>;


export type RestoreAccountMutation = { __typename?: 'Mutation', restoreAccount: boolean };

export type MyRolesQueryVariables = Exact<{ [key: string]: never; }>;


export type MyRolesQuery = { __typename?: 'Query', me: { __typename?: 'Me', id: string, roles: Array<{ __typename?: 'OwnRole', id: string, name: string, public: boolean }> } };

export type AdminCountsQueryVariables = Exact<{
  review: Scalars['Boolean']['input'];
  reports: Scalars['Boolean']['input'];
}>;


export type AdminCountsQuery = { __typename?: 'Query', reviewQueue?: { __typename?: 'ReviewQueue', total: number }, reportQueue?: { __typename?: 'ReportQueue', total: number }, globalSet: { __typename?: 'EmoteSet', id: string } };

export type ReviewQueueQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type ReviewQueueQuery = { __typename?: 'Query', reviewQueue: { __typename?: 'ReviewQueue', total: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type ApproveMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type ApproveMutation = { __typename?: 'Mutation', approveEmote: { __typename?: 'Emote', id: string, state: EmoteState } };

export type RejectMutationVariables = Exact<{
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
}>;


export type RejectMutation = { __typename?: 'Mutation', rejectEmote: { __typename?: 'Emote', id: string, state: EmoteState } };

export type ReportFieldsFragment = { __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, resolution?: string | null, createdAt: string, resolvedAt?: string | null, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null };

export type ReportQueueQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type ReportQueueQuery = { __typename?: 'Query', reportQueue: { __typename?: 'ReportQueue', total: number, items: Array<{ __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, createdAt: string, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }> } };

export type ResolveReportMutationVariables = Exact<{
  id: Scalars['String']['input'];
  action: ReportAction;
  note?: InputMaybe<Scalars['String']['input']>;
}>;


export type ResolveReportMutation = { __typename?: 'Mutation', resolveReport: { __typename?: 'Report', id: string, state: ReportState } };

export type TakedownMutationVariables = Exact<{
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
}>;


export type TakedownMutation = { __typename?: 'Mutation', takedownEmote: { __typename?: 'Emote', id: string, state: EmoteState } };

export type RoleFieldsFragment = { __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, public: boolean, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean };

export type RolesQueryVariables = Exact<{ [key: string]: never; }>;


export type RolesQuery = { __typename?: 'Query', permissions: Array<string>, roles: Array<{ __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, public: boolean, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type CreateRoleMutationVariables = Exact<{
  input: CreateRoleInput;
}>;


export type CreateRoleMutation = { __typename?: 'Mutation', createRole: { __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, public: boolean, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean } };

export type UpdateRoleMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateRoleInput;
}>;


export type UpdateRoleMutation = { __typename?: 'Mutation', updateRole: { __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, public: boolean, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean } };

export type DeleteRoleMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteRoleMutation = { __typename?: 'Mutation', deleteRole: boolean };

export type UserQueryVariables = Exact<{
  platform: Platform;
  login: Scalars['String']['input'];
}>;


export type UserQuery = { __typename?: 'Query', channelByLogin?: { __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } | null };

export type UserDetailsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
  status: Scalars['Boolean']['input'];
  roles: Scalars['Boolean']['input'];
}>;


export type UserDetailsQuery = { __typename?: 'Query', accountStatus?: { __typename?: 'AccountStatus', uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null }, accountRoles?: Array<{ __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, public: boolean, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type AllRolesQueryVariables = Exact<{ [key: string]: never; }>;


export type AllRolesQuery = { __typename?: 'Query', roles: Array<{ __typename?: 'Role', id: string, name: string, description?: string | null, position: number, permissions: Array<string>, public: boolean, builtIn?: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type RestrictMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  uploadsBlocked?: InputMaybe<Scalars['Boolean']['input']>;
  blocked?: InputMaybe<Scalars['Boolean']['input']>;
}>;


export type RestrictMutation = { __typename?: 'Mutation', restrictAccount: { __typename?: 'AccountStatus', uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null } };

export type PlaceRestrictionMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  input: RestrictionInput;
}>;


export type PlaceRestrictionMutation = { __typename?: 'Mutation', placeRestriction: { __typename?: 'AccountStatus', uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null } };

export type LiftRestrictionMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  kind: RestrictionKind;
  reason?: InputMaybe<Scalars['String']['input']>;
}>;


export type LiftRestrictionMutation = { __typename?: 'Mutation', liftRestriction: { __typename?: 'AccountStatus', uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null } };

export type AccountRestrictionsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountRestrictionsQuery = { __typename?: 'Query', accountRestrictions: Array<{ __typename?: 'Restriction', id: string, kind: RestrictionKind, reason?: string | null, endsAt?: string | null, createdBy?: string | null, createdAt: string, liftedAt?: string | null, liftedBy?: string | null, active: boolean }> };

export type AssignRoleMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
}>;


export type AssignRoleMutation = { __typename?: 'Mutation', assignRole: boolean };

export type UnassignRoleMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  roleId: Scalars['String']['input'];
}>;


export type UnassignRoleMutation = { __typename?: 'Mutation', unassignRole: boolean };

export type AuditQueryVariables = Exact<{
  targetKind?: InputMaybe<Scalars['String']['input']>;
  targetId?: InputMaybe<Scalars['String']['input']>;
  actorId?: InputMaybe<Scalars['String']['input']>;
  action?: InputMaybe<Scalars['String']['input']>;
  since?: InputMaybe<Scalars['String']['input']>;
  until?: InputMaybe<Scalars['String']['input']>;
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type AuditQuery = { __typename?: 'Query', auditLog: Array<{ __typename?: 'AuditEntry', id: string, actorId?: string | null, action: string, targetKind: string, targetId: string, before?: string | null, after?: string | null, createdAt: string }> };

export type ReportEmoteMutationVariables = Exact<{
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
}>;


export type ReportEmoteMutation = { __typename?: 'Mutation', reportEmote: { __typename?: 'Report', id: string } };

export type MySetNamesQueryVariables = Exact<{ [key: string]: never; }>;


export type MySetNamesQuery = { __typename?: 'Query', mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, emotes: Array<{ __typename?: 'SetEmote', emote: { __typename?: 'Emote', id: string } }> }> };

export type SearchByOwnerQueryVariables = Exact<{
  owner: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
  sort?: InputMaybe<EmoteSort>;
}>;


export type SearchByOwnerQuery = { __typename?: 'Query', searchEmotes: { __typename?: 'SearchResults', total: number, page: number, perPage: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type AdminAccountQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AdminAccountQuery = { __typename?: 'Query', adminAccount: { __typename?: 'AdminAccount', id: string, displayName: string, avatar?: string | null, createdAt: string, uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil?: string | null, ticketsBlockedUntil?: string | null, blockedUntil?: string | null, identities: Array<{ __typename?: 'Identity', platform: Platform, platformUserId: string, login: string, displayName: string, avatar?: string | null }> } };

export type AccountEmotesQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type AccountEmotesQuery = { __typename?: 'Query', accountEmotes: { __typename?: 'EmotePage', total: number, items: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type AccountReportsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountReportsQuery = { __typename?: 'Query', accountReports: { __typename?: 'AccountReports', about: Array<{ __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, resolution?: string | null, createdAt: string, resolvedAt?: string | null, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }>, by: Array<{ __typename?: 'Report', id: string, reason: string, reporterId?: string | null, state: ReportState, resolution?: string | null, createdAt: string, resolvedAt?: string | null, emote?: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }> } };

export type PublicAccountQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type PublicAccountQuery = { __typename?: 'Query', account?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null };

export type EmoteChannelsQueryVariables = Exact<{
  id: Scalars['String']['input'];
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type EmoteChannelsQuery = { __typename?: 'Query', emoteChannels: Array<{ __typename?: 'Channel', platform: Platform, login: string, account: { __typename?: 'AccountSummary', displayName: string, avatar?: string | null } }> };

export type CosmeticImageFieldsFragment = { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> };

export type BadgeFieldsFragment = { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } };

export type PaintFieldsFragment = { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null };

export type CosmeticsQueryVariables = Exact<{ [key: string]: never; }>;


export type CosmeticsQuery = { __typename?: 'Query', cosmetics: { __typename?: 'CosmeticCatalog', badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type UserCosmeticsQueryVariables = Exact<{
  platform: Platform;
  userIds: Array<Scalars['String']['input']> | Scalars['String']['input'];
}>;


export type UserCosmeticsQuery = { __typename?: 'Query', userCosmetics: Array<{ __typename?: 'UserCosmetics', platformUserId: string, badgeId?: string | null, paintId?: string | null }> };

export type AccountCosmeticsQueryVariables = Exact<{
  accountIds: Array<Scalars['String']['input']> | Scalars['String']['input'];
}>;


export type AccountCosmeticsQuery = { __typename?: 'Query', accountCosmetics: Array<{ __typename?: 'AccountCosmetics', accountId: string, badge?: { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } } | null, paint?: { __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null } | null }> };

export type AccountProfileQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountProfileQuery = { __typename?: 'Query', accountProfile?: { __typename?: 'AccountProfile', createdAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }>, roles: Array<{ __typename?: 'PublicRole', id: string, name: string }>, activeSets: Array<{ __typename?: 'ActiveChannelSet', platform: Platform, login: string, setId: string, setName: string }> } | null };

export type MyCosmeticsQueryVariables = Exact<{
  accountId?: InputMaybe<Scalars['String']['input']>;
}>;


export type MyCosmeticsQuery = { __typename?: 'Query', myCosmetics: { __typename?: 'OwnedCosmetics', badgeId?: string | null, paintId?: string | null, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type SelectCosmeticsMutationVariables = Exact<{
  badgeId?: InputMaybe<Scalars['String']['input']>;
  paintId?: InputMaybe<Scalars['String']['input']>;
  ownerId?: InputMaybe<Scalars['String']['input']>;
}>;


export type SelectCosmeticsMutation = { __typename?: 'Mutation', selectCosmetics: { __typename?: 'OwnedCosmetics', badgeId?: string | null, paintId?: string | null } };

export type CosmeticImageQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type CosmeticImageQuery = { __typename?: 'Query', cosmeticImage?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null };

export type EntitlementsQueryVariables = Exact<{
  kind: CosmeticKind;
  accountId?: InputMaybe<Scalars['String']['input']>;
  cosmeticId?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type EntitlementsQuery = { __typename?: 'Query', cosmeticEntitlements: { __typename?: 'EntitlementPage', total: number, items: Array<{ __typename?: 'Entitlement', kind: CosmeticKind, cosmeticId: string, source: EntitlementSource, reason?: string | null, grantedAt: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, grantedBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null }> } };

export type CreateBadgeMutationVariables = Exact<{
  input: CreateBadgeInput;
}>;


export type CreateBadgeMutation = { __typename?: 'Mutation', createBadge: { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } } };

export type UpdateBadgeMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: UpdateBadgeInput;
}>;


export type UpdateBadgeMutation = { __typename?: 'Mutation', updateBadge: { __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } } };

export type DeleteBadgeMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteBadgeMutation = { __typename?: 'Mutation', deleteBadge: boolean };

export type DeletePaintMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeletePaintMutation = { __typename?: 'Mutation', deletePaint: boolean };

export type GrantCosmeticMutationVariables = Exact<{
  input: GrantCosmeticInput;
}>;


export type GrantCosmeticMutation = { __typename?: 'Mutation', grantCosmetic: { __typename?: 'Entitlement', cosmeticId: string } };

export type RevokeCosmeticMutationVariables = Exact<{
  accountId: Scalars['String']['input'];
  kind: CosmeticKind;
  cosmeticId: Scalars['String']['input'];
  reason?: InputMaybe<Scalars['String']['input']>;
}>;


export type RevokeCosmeticMutation = { __typename?: 'Mutation', revokeCosmetic: boolean };

export type AccountRolesQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountRolesQuery = { __typename?: 'Query', accountRoles: Array<{ __typename?: 'Role', id: string, name: string, position: number }> };

export type AccountEditorsQueryVariables = Exact<{
  accountId?: InputMaybe<Scalars['String']['input']>;
}>;


export type AccountEditorsQuery = { __typename?: 'Query', accountEditors: Array<{ __typename?: 'AccountEditor', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } }> };

export type EditorOfQueryVariables = Exact<{ [key: string]: never; }>;


export type EditorOfQuery = { __typename?: 'Query', editorOf: Array<{ __typename?: 'EditedAccount', permissions: Array<EditorPermission>, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } }> };

export type AccountSetsQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
}>;


export type AccountSetsQuery = { __typename?: 'Query', accountSets: Array<{ __typename?: 'SetSummary', id: string, name: string, kind: SetKind, capacity: number, slotsUsed: number, active: boolean, preview: Array<{ __typename?: 'SetEmote', emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ProfileMineQueryVariables = Exact<{ [key: string]: never; }>;


export type ProfileMineQuery = { __typename?: 'Query', myChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null }>, mySets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ProfileEditedQueryVariables = Exact<{ [key: string]: never; }>;


export type ProfileEditedQuery = { __typename?: 'Query', editorChannels: Array<{ __typename?: 'Channel', platform: Platform, platformChannelId: string, login: string, activeSetId?: string | null, account: { __typename?: 'AccountSummary', id: string } }>, editorSets: Array<{ __typename?: 'EmoteSet', id: string, name: string, kind: SetKind, capacity: number, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, emotes: Array<{ __typename?: 'SetEmote', name: string, zeroWidth: boolean, addedAt: string, emote: { __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type AccountActivityQueryVariables = Exact<{
  accountId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type AccountActivityQuery = { __typename?: 'Query', accountActivityPage: { __typename?: 'ActivityPage', next?: string | null, entries: Array<{ __typename?: 'ActivityEntry', id: string, kind: ActivityKind, fromName?: string | null, toName?: string | null, createdAt: string, actor: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, set?: { __typename?: 'ActivitySet', id: string, name: string } | null, emote?: { __typename?: 'ActivityEmote', id: string, name: string, host: string, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null, badge?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null, paint?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null }> } };

export type EmoteActivityQueryVariables = Exact<{
  emoteId: Scalars['String']['input'];
  before?: InputMaybe<Scalars['String']['input']>;
  limit?: InputMaybe<Scalars['Int']['input']>;
}>;


export type EmoteActivityQuery = { __typename?: 'Query', emoteActivityPage: { __typename?: 'ActivityPage', next?: string | null, entries: Array<{ __typename?: 'ActivityEntry', id: string, kind: ActivityKind, fromName?: string | null, toName?: string | null, createdAt: string, actor: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, set?: { __typename?: 'ActivitySet', id: string, name: string } | null, emote?: { __typename?: 'ActivityEmote', id: string, name: string, host: string, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null, badge?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null, paint?: { __typename?: 'ActivityCosmeticChange', fromId?: string | null, fromName?: string | null, toId?: string | null, toName?: string | null } | null }> } };

export type ShopItemFieldsFragment = { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> };

export type ShopItemsQueryVariables = Exact<{
  all?: InputMaybe<Scalars['Boolean']['input']>;
}>;


export type ShopItemsQuery = { __typename?: 'Query', shopItems: Array<{ __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> }> };

export type ShopItemQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type ShopItemQuery = { __typename?: 'Query', shopItem?: { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } | null };

export type OrderFieldsFragment = { __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null };

export type StartCheckoutMutationVariables = Exact<{
  itemId: Scalars['String']['input'];
  recipient?: InputMaybe<RecipientInput>;
}>;


export type StartCheckoutMutation = { __typename?: 'Mutation', startCheckout: { __typename?: 'Checkout', transactionId: string, checkoutUrl?: string | null, clientToken?: string | null, environment: string, order: { __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null } } };

export type OrderQueryVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type OrderQuery = { __typename?: 'Query', order?: { __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null } | null };

export type MyOrdersQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type MyOrdersQuery = { __typename?: 'Query', myOrders: { __typename?: 'OrderPage', total: number, items: Array<{ __typename?: 'Order', id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null }> } };

export type RedeemCodeMutationVariables = Exact<{
  code: Scalars['String']['input'];
}>;


export type RedeemCodeMutation = { __typename?: 'Mutation', redeemCode: { __typename?: 'RedeemResult', name: string, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type CreateShopItemMutationVariables = Exact<{
  input: ShopItemInput;
}>;


export type CreateShopItemMutation = { __typename?: 'Mutation', createShopItem: { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type UpdateShopItemMutationVariables = Exact<{
  id: Scalars['String']['input'];
  input: ShopItemInput;
}>;


export type UpdateShopItemMutation = { __typename?: 'Mutation', updateShopItem: { __typename?: 'ShopItem', id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId?: string | null, activeFrom?: string | null, activeTo?: string | null, stockLimit?: number | null, stockLeft?: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ __typename?: 'Badge', id: string, name: string, tooltip: string, image: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ __typename?: 'Paint', id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color?: string | null, stops: Array<{ __typename?: 'PaintStop', at: number, color: string }>, shadows: Array<{ __typename?: 'PaintShadow', x: number, y: number, radius: number, color: string }>, image?: { __typename?: 'CosmeticImage', id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ __typename?: 'EmoteFile', name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type DeleteShopItemMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DeleteShopItemMutation = { __typename?: 'Mutation', deleteShopItem: boolean };

export type AdminOrdersQueryVariables = Exact<{
  status?: InputMaybe<OrderStatus>;
  accountId?: InputMaybe<Scalars['String']['input']>;
  itemId?: InputMaybe<Scalars['String']['input']>;
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type AdminOrdersQuery = { __typename?: 'Query', orders: { __typename?: 'OrderPage', total: number, items: Array<{ __typename?: 'Order', providerTransactionId?: string | null, refundRejectedAt?: string | null, id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null }> } };

export type RefundOrderMutationVariables = Exact<{
  id: Scalars['String']['input'];
  reason: Scalars['String']['input'];
}>;


export type RefundOrderMutation = { __typename?: 'Mutation', refundOrder: { __typename?: 'Order', providerTransactionId?: string | null, refundRejectedAt?: string | null, id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents?: number | null, paidCurrency?: string | null, refundReason?: string | null, createdAt: string, paidAt?: string | null, refundedAt?: string | null, failedAt?: string | null, buyer?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, recipient?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null } };

export type CodeBatchFieldsFragment = { __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null };

export type CodeBatchesQueryVariables = Exact<{
  page?: InputMaybe<Scalars['Int']['input']>;
  perPage?: InputMaybe<Scalars['Int']['input']>;
}>;


export type CodeBatchesQuery = { __typename?: 'Query', codeBatches: { __typename?: 'CodeBatchPage', total: number, items: Array<{ __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null }> } };

export type CreateCodeBatchMutationVariables = Exact<{
  input: CodeBatchInput;
}>;


export type CreateCodeBatchMutation = { __typename?: 'Mutation', createCodeBatch: { __typename?: 'CreatedCodeBatch', codes: Array<string>, batch: { __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } } };

export type DisableCodeBatchMutationVariables = Exact<{
  id: Scalars['String']['input'];
}>;


export type DisableCodeBatchMutation = { __typename?: 'Mutation', disableCodeBatch: { __typename?: 'CodeBatch', id: string, name: string, prefix: string, itemId?: string | null, itemName?: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt?: string | null, disabledAt?: string | null, createdAt: string, createdBy?: { __typename?: 'AccountSummary', id: string, displayName: string } | null } };

export type ShopCountsQueryVariables = Exact<{
  items: Scalars['Boolean']['input'];
  orders: Scalars['Boolean']['input'];
  codes: Scalars['Boolean']['input'];
}>;


export type ShopCountsQuery = { __typename?: 'Query', shopItems?: Array<{ __typename?: 'ShopItem', id: string }>, orders?: { __typename?: 'OrderPage', total: number }, codeBatches?: { __typename?: 'CodeBatchPage', total: number } };

export type QuickSearchQueryVariables = Exact<{
  query: Scalars['String']['input'];
}>;


export type QuickSearchQuery = { __typename?: 'Query', quickSearch: { __typename?: 'QuickSearch', emotes: Array<{ __typename?: 'Emote', id: string, name: string, state: EmoteState, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { __typename?: 'EmoteFlags', zeroWidth: boolean, nsfw: boolean, private: boolean }, owner?: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } | null, files: Array<{ __typename?: 'EmoteFile', name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }>, users: Array<{ __typename?: 'AccountHit', account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null }, identities: Array<{ __typename?: 'PublicIdentity', platform: Platform, login: string, displayName: string }> }> } };

export type EditableChannelsQueryVariables = Exact<{ [key: string]: never; }>;


export type EditableChannelsQuery = { __typename?: 'Query', myEditableChannels: Array<{ __typename?: 'EditableChannel', role: ChannelRole, channel: { __typename?: 'Channel', platform: Platform, login: string, account: { __typename?: 'AccountSummary', id: string, displayName: string, avatar?: string | null } } }> };

export class TypedDocumentString<TResult, TVariables>
  extends String
  implements DocumentTypeDecoration<TResult, TVariables>
{
  __apiType?: NonNullable<DocumentTypeDecoration<TResult, TVariables>['__apiType']>;
  private value: string;
  public __meta__?: Record<string, any> | undefined;

  constructor(value: string, __meta__?: Record<string, any> | undefined) {
    super(value);
    this.value = value;
    this.__meta__ = __meta__;
  }

  override toString(): string & DocumentTypeDecoration<TResult, TVariables> {
    return this.value;
  }
}
export const StudioPaintFieldsFragmentDoc = new TypedDocumentString(`
    fragment StudioPaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}
    `, {"fragmentName":"StudioPaintFields"}) as unknown as TypedDocumentString<StudioPaintFieldsFragment, unknown>;
export const EmoteFieldsFragmentDoc = new TypedDocumentString(`
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}
    `, {"fragmentName":"EmoteFields"}) as unknown as TypedDocumentString<EmoteFieldsFragment, unknown>;
export const SetFieldsFragmentDoc = new TypedDocumentString(`
    fragment SetFields on EmoteSet {
  id
  name
  kind
  capacity
  owner {
    id
    displayName
    avatar
  }
  emotes {
    name
    zeroWidth
    addedAt
    emote {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`, {"fragmentName":"SetFields"}) as unknown as TypedDocumentString<SetFieldsFragment, unknown>;
export const AccountEditorFieldsFragmentDoc = new TypedDocumentString(`
    fragment AccountEditorFields on AccountEditor {
  account {
    id
    displayName
    avatar
  }
  permissions
  grantedBy {
    id
    displayName
  }
  grantedAt
}
    `, {"fragmentName":"AccountEditorFields"}) as unknown as TypedDocumentString<AccountEditorFieldsFragment, unknown>;
export const ReportFieldsFragmentDoc = new TypedDocumentString(`
    fragment ReportFields on Report {
  id
  reason
  reporterId
  state
  resolution
  createdAt
  resolvedAt
  emote {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`, {"fragmentName":"ReportFields"}) as unknown as TypedDocumentString<ReportFieldsFragment, unknown>;
export const RoleFieldsFragmentDoc = new TypedDocumentString(`
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  public
  builtIn
  editable
  assignable
}
    `, {"fragmentName":"RoleFields"}) as unknown as TypedDocumentString<RoleFieldsFragment, unknown>;
export const CosmeticImageFieldsFragmentDoc = new TypedDocumentString(`
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
    `, {"fragmentName":"CosmeticImageFields"}) as unknown as TypedDocumentString<CosmeticImageFieldsFragment, unknown>;
export const BadgeFieldsFragmentDoc = new TypedDocumentString(`
    fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}`, {"fragmentName":"BadgeFields"}) as unknown as TypedDocumentString<BadgeFieldsFragment, unknown>;
export const PaintFieldsFragmentDoc = new TypedDocumentString(`
    fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}`, {"fragmentName":"PaintFields"}) as unknown as TypedDocumentString<PaintFieldsFragment, unknown>;
export const ShopItemFieldsFragmentDoc = new TypedDocumentString(`
    fragment ShopItemFields on ShopItem {
  id
  name
  description
  priceCents
  currency
  paddlePriceId
  badges {
    ...BadgeFields
  }
  paints {
    ...PaintFields
  }
  activeFrom
  activeTo
  stockLimit
  stockLeft
  visible
  sortOrder
  available
  owned
  deleted
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}`, {"fragmentName":"ShopItemFields"}) as unknown as TypedDocumentString<ShopItemFieldsFragment, unknown>;
export const OrderFieldsFragmentDoc = new TypedDocumentString(`
    fragment OrderFields on Order {
  id
  buyer {
    id
    displayName
    avatar
  }
  recipient {
    id
    displayName
    avatar
  }
  gift
  itemId
  itemName
  status
  amountCents
  currency
  paidTotalCents
  paidCurrency
  refundReason
  createdAt
  paidAt
  refundedAt
  failedAt
}
    `, {"fragmentName":"OrderFields"}) as unknown as TypedDocumentString<OrderFieldsFragment, unknown>;
export const CodeBatchFieldsFragmentDoc = new TypedDocumentString(`
    fragment CodeBatchFields on CodeBatch {
  id
  name
  prefix
  itemId
  itemName
  badgeIds
  paintIds
  codeCount
  usesPerCode
  redemptions
  expiresAt
  disabledAt
  createdBy {
    id
    displayName
  }
  createdAt
}
    `, {"fragmentName":"CodeBatchFields"}) as unknown as TypedDocumentString<CodeBatchFieldsFragment, unknown>;
export const StudioPaintsDocument = new TypedDocumentString(`
    query StudioPaints {
  cosmetics {
    paints {
      ...StudioPaintFields
    }
  }
}
    fragment StudioPaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}`) as unknown as TypedDocumentString<StudioPaintsQuery, StudioPaintsQueryVariables>;
export const StudioCreatePaintDocument = new TypedDocumentString(`
    mutation StudioCreatePaint($input: PaintInput!) {
  createPaint(input: $input) {
    ...StudioPaintFields
  }
}
    fragment StudioPaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}`) as unknown as TypedDocumentString<StudioCreatePaintMutation, StudioCreatePaintMutationVariables>;
export const StudioUpdatePaintDocument = new TypedDocumentString(`
    mutation StudioUpdatePaint($id: String!, $input: PaintInput!) {
  updatePaint(id: $id, input: $input) {
    ...StudioPaintFields
  }
}
    fragment StudioPaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
  updatedAt
}`) as unknown as TypedDocumentString<StudioUpdatePaintMutation, StudioUpdatePaintMutationVariables>;
export const StudioCosmeticImageDocument = new TypedDocumentString(`
    query StudioCosmeticImage($id: String!) {
  cosmeticImage(id: $id) {
    id
    host
    ready
    width
    height
    animated
    files {
      name
      format
      scale
      width
      height
      isStatic
    }
  }
}
    `) as unknown as TypedDocumentString<StudioCosmeticImageQuery, StudioCosmeticImageQueryVariables>;
export const SearchDocument = new TypedDocumentString(`
    query Search($query: String!, $animated: Boolean, $nsfw: Boolean, $page: Int, $perPage: Int, $sort: EmoteSort) {
  searchEmotes(
    query: $query
    animated: $animated
    nsfw: $nsfw
    page: $page
    perPage: $perPage
    sort: $sort
  ) {
    total
    page
    perPage
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<SearchQuery, SearchQueryVariables>;
export const FindPeopleDocument = new TypedDocumentString(`
    query FindPeople($query: String!, $page: Int, $perPage: Int) {
  searchAccounts(query: $query, page: $page, perPage: $perPage) {
    total
    page
    perPage
    items {
      account {
        id
        displayName
        avatar
      }
      identities {
        platform
        login
        displayName
      }
    }
  }
}
    `) as unknown as TypedDocumentString<FindPeopleQuery, FindPeopleQueryVariables>;
export const EmoteDocument = new TypedDocumentString(`
    query Emote($id: String!) {
  emote(id: $id) {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<EmoteQuery, EmoteQueryVariables>;
export const ChannelDocument = new TypedDocumentString(`
    query Channel($platform: Platform!, $login: String!) {
  channelByLogin(platform: $platform, login: $login) {
    platform
    platformChannelId
    login
    activeSetId
    allowPersonalEmotes
    account {
      id
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<ChannelQuery, ChannelQueryVariables>;
export const SetDocument = new TypedDocumentString(`
    query Set($id: String!) {
  set(id: $id) {
    id
    name
    kind
    capacity
    owner {
      id
      displayName
    }
    emotes {
      name
      zeroWidth
      addedAt
      emote {
        ...EmoteFields
      }
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<SetQuery, SetQueryVariables>;
export const PublicSetDocument = new TypedDocumentString(`
    query PublicSet($id: String!) {
  set(id: $id) {
    id
    name
    kind
    visibility
    capacity
    slotsUsed
    createdAt
    owner {
      id
      displayName
      avatar
    }
    emotes {
      name
      zeroWidth
      addedAt
      emote {
        ...EmoteFields
      }
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<PublicSetQuery, PublicSetQueryVariables>;
export const CloneSetDocument = new TypedDocumentString(`
    mutation CloneSet($id: String!, $name: String, $visibility: SetVisibility) {
  cloneSet(id: $id, name: $name, visibility: $visibility) {
    copied
    skipped
    set {
      id
      name
    }
  }
}
    `) as unknown as TypedDocumentString<CloneSetMutation, CloneSetMutationVariables>;
export const GlobalSetDocument = new TypedDocumentString(`
    query GlobalSet {
  globalSet {
    id
    name
    emotes {
      name
      zeroWidth
      emote {
        ...EmoteFields
      }
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<GlobalSetQuery, GlobalSetQueryVariables>;
export const MeDocument = new TypedDocumentString(`
    query Me {
  me {
    id
    createdAt
    permissions
    identities {
      platform
      platformUserId
      login
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<MeQuery, MeQueryVariables>;
export const MyEmotesDocument = new TypedDocumentString(`
    query MyEmotes($page: Int, $perPage: Int) {
  myEmotes(page: $page, perPage: $perPage) {
    total
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<MyEmotesQuery, MyEmotesQueryVariables>;
export const UpdateEmoteDocument = new TypedDocumentString(`
    mutation UpdateEmote($id: String!, $input: UpdateEmoteInput!) {
  updateEmote(id: $id, input: $input) {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<UpdateEmoteMutation, UpdateEmoteMutationVariables>;
export const DeleteEmoteDocument = new TypedDocumentString(`
    mutation DeleteEmote($id: String!) {
  deleteEmote(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteEmoteMutation, DeleteEmoteMutationVariables>;
export const MySetsDocument = new TypedDocumentString(`
    query MySets {
  mySets {
    ...SetFields
  }
  editorSets {
    ...SetFields
  }
  editorOf {
    account {
      id
      displayName
    }
    permissions
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}
fragment SetFields on EmoteSet {
  id
  name
  kind
  capacity
  owner {
    id
    displayName
    avatar
  }
  emotes {
    name
    zeroWidth
    addedAt
    emote {
      ...EmoteFields
    }
  }
}`) as unknown as TypedDocumentString<MySetsQuery, MySetsQueryVariables>;
export const EditSetDocument = new TypedDocumentString(`
    query EditSet($id: String!) {
  set(id: $id) {
    ...SetFields
  }
  setEditors(setId: $id) {
    id
    displayName
    avatar
  }
  editorOf {
    account {
      id
    }
    permissions
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}
fragment SetFields on EmoteSet {
  id
  name
  kind
  capacity
  owner {
    id
    displayName
    avatar
  }
  emotes {
    name
    zeroWidth
    addedAt
    emote {
      ...EmoteFields
    }
  }
}`) as unknown as TypedDocumentString<EditSetQuery, EditSetQueryVariables>;
export const CreateSetDocument = new TypedDocumentString(`
    mutation CreateSet($input: CreateSetInput!) {
  createSet(input: $input) {
    id
  }
}
    `) as unknown as TypedDocumentString<CreateSetMutation, CreateSetMutationVariables>;
export const UpdateSetDocument = new TypedDocumentString(`
    mutation UpdateSet($id: String!, $input: UpdateSetInput!) {
  updateSet(id: $id, input: $input) {
    id
    name
  }
}
    `) as unknown as TypedDocumentString<UpdateSetMutation, UpdateSetMutationVariables>;
export const DeleteSetDocument = new TypedDocumentString(`
    mutation DeleteSet($id: String!) {
  deleteSet(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteSetMutation, DeleteSetMutationVariables>;
export const AddSetEmoteDocument = new TypedDocumentString(`
    mutation AddSetEmote($setId: String!, $emoteId: String!, $input: AddSetEmoteInput) {
  addSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {
    name
  }
}
    `) as unknown as TypedDocumentString<AddSetEmoteMutation, AddSetEmoteMutationVariables>;
export const UpdateSetEmoteDocument = new TypedDocumentString(`
    mutation UpdateSetEmote($setId: String!, $emoteId: String!, $input: UpdateSetEmoteInput!) {
  updateSetEmote(setId: $setId, emoteId: $emoteId, input: $input) {
    name
    zeroWidth
  }
}
    `) as unknown as TypedDocumentString<UpdateSetEmoteMutation, UpdateSetEmoteMutationVariables>;
export const RemoveSetEmoteDocument = new TypedDocumentString(`
    mutation RemoveSetEmote($setId: String!, $emoteId: String!) {
  removeSetEmote(setId: $setId, emoteId: $emoteId)
}
    `) as unknown as TypedDocumentString<RemoveSetEmoteMutation, RemoveSetEmoteMutationVariables>;
export const AddSetEditorDocument = new TypedDocumentString(`
    mutation AddSetEditor($setId: String!, $accountId: String!) {
  addSetEditor(setId: $setId, accountId: $accountId)
}
    `) as unknown as TypedDocumentString<AddSetEditorMutation, AddSetEditorMutationVariables>;
export const RemoveSetEditorDocument = new TypedDocumentString(`
    mutation RemoveSetEditor($setId: String!, $accountId: String!) {
  removeSetEditor(setId: $setId, accountId: $accountId)
}
    `) as unknown as TypedDocumentString<RemoveSetEditorMutation, RemoveSetEditorMutationVariables>;
export const FindAccountDocument = new TypedDocumentString(`
    query FindAccount($platform: Platform!, $login: String!) {
  channelByLogin(platform: $platform, login: $login) {
    login
    account {
      id
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<FindAccountQuery, FindAccountQueryVariables>;
export const MyChannelsDocument = new TypedDocumentString(`
    query MyChannels {
  myChannels {
    platform
    platformChannelId
    login
    activeSetId
    allowPersonalEmotes
    account {
      id
      displayName
    }
  }
  mySets {
    id
    name
    kind
    owner {
      id
    }
  }
  editorChannels {
    platform
    platformChannelId
    login
    activeSetId
    allowPersonalEmotes
    account {
      id
      displayName
    }
  }
  editorSets {
    id
    name
    kind
    owner {
      id
    }
  }
  editorOf {
    account {
      id
    }
    permissions
  }
}
    `) as unknown as TypedDocumentString<MyChannelsQuery, MyChannelsQueryVariables>;
export const UpdateChannelDocument = new TypedDocumentString(`
    mutation UpdateChannel($platform: Platform!, $id: String!, $input: UpdateChannelSettingsInput!) {
  updateChannelSettings(platform: $platform, id: $id, input: $input) {
    activeSetId
    allowPersonalEmotes
  }
}
    `) as unknown as TypedDocumentString<UpdateChannelMutation, UpdateChannelMutationVariables>;
export const EditorsDocument = new TypedDocumentString(`
    query Editors {
  accountEditors {
    ...AccountEditorFields
  }
  editorOf {
    account {
      id
      displayName
      avatar
    }
    permissions
    grantedAt
  }
}
    fragment AccountEditorFields on AccountEditor {
  account {
    id
    displayName
    avatar
  }
  permissions
  grantedBy {
    id
    displayName
  }
  grantedAt
}`) as unknown as TypedDocumentString<EditorsQuery, EditorsQueryVariables>;
export const AddAccountEditorDocument = new TypedDocumentString(`
    mutation AddAccountEditor($input: AddAccountEditorInput!) {
  addAccountEditor(input: $input) {
    ...AccountEditorFields
  }
}
    fragment AccountEditorFields on AccountEditor {
  account {
    id
    displayName
    avatar
  }
  permissions
  grantedBy {
    id
    displayName
  }
  grantedAt
}`) as unknown as TypedDocumentString<AddAccountEditorMutation, AddAccountEditorMutationVariables>;
export const UpdateAccountEditorDocument = new TypedDocumentString(`
    mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!) {
  updateAccountEditor(
    ownerId: $ownerId
    accountId: $accountId
    permissions: $permissions
  ) {
    ...AccountEditorFields
  }
}
    fragment AccountEditorFields on AccountEditor {
  account {
    id
    displayName
    avatar
  }
  permissions
  grantedBy {
    id
    displayName
  }
  grantedAt
}`) as unknown as TypedDocumentString<UpdateAccountEditorMutation, UpdateAccountEditorMutationVariables>;
export const RemoveAccountEditorDocument = new TypedDocumentString(`
    mutation RemoveAccountEditor($ownerId: String, $accountId: String!) {
  removeAccountEditor(ownerId: $ownerId, accountId: $accountId)
}
    `) as unknown as TypedDocumentString<RemoveAccountEditorMutation, RemoveAccountEditorMutationVariables>;
export const UnlinkDocument = new TypedDocumentString(`
    mutation Unlink($platform: Platform!) {
  unlinkIdentity(platform: $platform) {
    id
  }
}
    `) as unknown as TypedDocumentString<UnlinkMutation, UnlinkMutationVariables>;
export const MyNotificationsDocument = new TypedDocumentString(`
    query MyNotifications {
  myNotifications {
    id
    kind
    payload
    createdAt
    read
  }
}
    `) as unknown as TypedDocumentString<MyNotificationsQuery, MyNotificationsQueryVariables>;
export const MarkReadDocument = new TypedDocumentString(`
    mutation MarkRead($ids: [String!]) {
  markNotificationsRead(ids: $ids)
}
    `) as unknown as TypedDocumentString<MarkReadMutation, MarkReadMutationVariables>;
export const UnreadCountDocument = new TypedDocumentString(`
    query UnreadCount {
  unreadNotificationCount
}
    `) as unknown as TypedDocumentString<UnreadCountQuery, UnreadCountQueryVariables>;
export const MySessionsDocument = new TypedDocumentString(`
    query MySessions {
  mySessions {
    id
    kind
    device
    createdAt
    lastSeenAt
    expiresAt
    current
  }
}
    `) as unknown as TypedDocumentString<MySessionsQuery, MySessionsQueryVariables>;
export const RevokeSessionDocument = new TypedDocumentString(`
    mutation RevokeSession($id: String!) {
  revokeSession(id: $id)
}
    `) as unknown as TypedDocumentString<RevokeSessionMutation, RevokeSessionMutationVariables>;
export const RevokeOtherSessionsDocument = new TypedDocumentString(`
    mutation RevokeOtherSessions {
  revokeOtherSessions
}
    `) as unknown as TypedDocumentString<RevokeOtherSessionsMutation, RevokeOtherSessionsMutationVariables>;
export const ExportMyDataDocument = new TypedDocumentString(`
    mutation ExportMyData {
  exportMyData
}
    `) as unknown as TypedDocumentString<ExportMyDataMutation, ExportMyDataMutationVariables>;
export const DeleteAccountDocument = new TypedDocumentString(`
    mutation DeleteAccount($confirm: String!) {
  deleteAccount(confirm: $confirm)
}
    `) as unknown as TypedDocumentString<DeleteAccountMutation, DeleteAccountMutationVariables>;
export const PendingDeletionDocument = new TypedDocumentString(`
    query PendingDeletion {
  pendingDeletion {
    requestedAt
    eraseAfter
    displayName
    avatar
  }
}
    `) as unknown as TypedDocumentString<PendingDeletionQuery, PendingDeletionQueryVariables>;
export const RestoreAccountDocument = new TypedDocumentString(`
    mutation RestoreAccount {
  restoreAccount
}
    `) as unknown as TypedDocumentString<RestoreAccountMutation, RestoreAccountMutationVariables>;
export const MyRolesDocument = new TypedDocumentString(`
    query MyRoles {
  me {
    id
    roles {
      id
      name
      public
    }
  }
}
    `) as unknown as TypedDocumentString<MyRolesQuery, MyRolesQueryVariables>;
export const AdminCountsDocument = new TypedDocumentString(`
    query AdminCounts($review: Boolean!, $reports: Boolean!) {
  reviewQueue(page: 1, perPage: 1) @include(if: $review) {
    total
  }
  reportQueue(page: 1, perPage: 1) @include(if: $reports) {
    total
  }
  globalSet {
    id
  }
}
    `) as unknown as TypedDocumentString<AdminCountsQuery, AdminCountsQueryVariables>;
export const ReviewQueueDocument = new TypedDocumentString(`
    query ReviewQueue($page: Int, $perPage: Int) {
  reviewQueue(page: $page, perPage: $perPage) {
    total
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<ReviewQueueQuery, ReviewQueueQueryVariables>;
export const ApproveDocument = new TypedDocumentString(`
    mutation Approve($id: String!) {
  approveEmote(id: $id) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<ApproveMutation, ApproveMutationVariables>;
export const RejectDocument = new TypedDocumentString(`
    mutation Reject($id: String!, $reason: String!) {
  rejectEmote(id: $id, reason: $reason) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<RejectMutation, RejectMutationVariables>;
export const ReportQueueDocument = new TypedDocumentString(`
    query ReportQueue($page: Int, $perPage: Int) {
  reportQueue(page: $page, perPage: $perPage) {
    total
    items {
      id
      reason
      reporterId
      state
      createdAt
      emote {
        ...EmoteFields
      }
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<ReportQueueQuery, ReportQueueQueryVariables>;
export const ResolveReportDocument = new TypedDocumentString(`
    mutation ResolveReport($id: String!, $action: ReportAction!, $note: String) {
  resolveReport(id: $id, action: $action, note: $note) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<ResolveReportMutation, ResolveReportMutationVariables>;
export const TakedownDocument = new TypedDocumentString(`
    mutation Takedown($id: String!, $reason: String!) {
  takedownEmote(id: $id, reason: $reason) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<TakedownMutation, TakedownMutationVariables>;
export const RolesDocument = new TypedDocumentString(`
    query Roles {
  roles {
    ...RoleFields
  }
  permissions
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  public
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<RolesQuery, RolesQueryVariables>;
export const CreateRoleDocument = new TypedDocumentString(`
    mutation CreateRole($input: CreateRoleInput!) {
  createRole(input: $input) {
    ...RoleFields
  }
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  public
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<CreateRoleMutation, CreateRoleMutationVariables>;
export const UpdateRoleDocument = new TypedDocumentString(`
    mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {
  updateRole(id: $id, input: $input) {
    ...RoleFields
  }
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  public
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<UpdateRoleMutation, UpdateRoleMutationVariables>;
export const DeleteRoleDocument = new TypedDocumentString(`
    mutation DeleteRole($id: String!) {
  deleteRole(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteRoleMutation, DeleteRoleMutationVariables>;
export const UserDocument = new TypedDocumentString(`
    query User($platform: Platform!, $login: String!) {
  channelByLogin(platform: $platform, login: $login) {
    platform
    platformChannelId
    login
    account {
      id
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<UserQuery, UserQueryVariables>;
export const UserDetailsDocument = new TypedDocumentString(`
    query UserDetails($accountId: String!, $status: Boolean!, $roles: Boolean!) {
  accountStatus(accountId: $accountId) @include(if: $status) {
    uploadsBlocked
    ticketsBlocked
    blocked
    uploadsBlockedUntil
    ticketsBlockedUntil
    blockedUntil
  }
  accountRoles(accountId: $accountId) @include(if: $roles) {
    ...RoleFields
  }
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  public
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<UserDetailsQuery, UserDetailsQueryVariables>;
export const AllRolesDocument = new TypedDocumentString(`
    query AllRoles {
  roles {
    ...RoleFields
  }
}
    fragment RoleFields on Role {
  id
  name
  description
  position
  permissions
  public
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<AllRolesQuery, AllRolesQueryVariables>;
export const RestrictDocument = new TypedDocumentString(`
    mutation Restrict($accountId: String!, $uploadsBlocked: Boolean, $blocked: Boolean) {
  restrictAccount(
    accountId: $accountId
    uploadsBlocked: $uploadsBlocked
    blocked: $blocked
  ) {
    uploadsBlocked
    ticketsBlocked
    blocked
    uploadsBlockedUntil
    ticketsBlockedUntil
    blockedUntil
  }
}
    `) as unknown as TypedDocumentString<RestrictMutation, RestrictMutationVariables>;
export const PlaceRestrictionDocument = new TypedDocumentString(`
    mutation PlaceRestriction($accountId: String!, $input: RestrictionInput!) {
  placeRestriction(accountId: $accountId, input: $input) {
    uploadsBlocked
    ticketsBlocked
    blocked
    uploadsBlockedUntil
    ticketsBlockedUntil
    blockedUntil
  }
}
    `) as unknown as TypedDocumentString<PlaceRestrictionMutation, PlaceRestrictionMutationVariables>;
export const LiftRestrictionDocument = new TypedDocumentString(`
    mutation LiftRestriction($accountId: String!, $kind: RestrictionKind!, $reason: String) {
  liftRestriction(accountId: $accountId, kind: $kind, reason: $reason) {
    uploadsBlocked
    ticketsBlocked
    blocked
    uploadsBlockedUntil
    ticketsBlockedUntil
    blockedUntil
  }
}
    `) as unknown as TypedDocumentString<LiftRestrictionMutation, LiftRestrictionMutationVariables>;
export const AccountRestrictionsDocument = new TypedDocumentString(`
    query AccountRestrictions($accountId: String!) {
  accountRestrictions(accountId: $accountId) {
    id
    kind
    reason
    endsAt
    createdBy
    createdAt
    liftedAt
    liftedBy
    active
  }
}
    `) as unknown as TypedDocumentString<AccountRestrictionsQuery, AccountRestrictionsQueryVariables>;
export const AssignRoleDocument = new TypedDocumentString(`
    mutation AssignRole($accountId: String!, $roleId: String!) {
  assignRole(accountId: $accountId, roleId: $roleId)
}
    `) as unknown as TypedDocumentString<AssignRoleMutation, AssignRoleMutationVariables>;
export const UnassignRoleDocument = new TypedDocumentString(`
    mutation UnassignRole($accountId: String!, $roleId: String!) {
  unassignRole(accountId: $accountId, roleId: $roleId)
}
    `) as unknown as TypedDocumentString<UnassignRoleMutation, UnassignRoleMutationVariables>;
export const AuditDocument = new TypedDocumentString(`
    query Audit($targetKind: String, $targetId: String, $actorId: String, $action: String, $since: String, $until: String, $before: String, $limit: Int) {
  auditLog(
    targetKind: $targetKind
    targetId: $targetId
    actorId: $actorId
    action: $action
    since: $since
    until: $until
    before: $before
    limit: $limit
  ) {
    id
    actorId
    action
    targetKind
    targetId
    before
    after
    createdAt
  }
}
    `) as unknown as TypedDocumentString<AuditQuery, AuditQueryVariables>;
export const ReportEmoteDocument = new TypedDocumentString(`
    mutation ReportEmote($id: String!, $reason: String!) {
  reportEmote(id: $id, reason: $reason) {
    id
  }
}
    `) as unknown as TypedDocumentString<ReportEmoteMutation, ReportEmoteMutationVariables>;
export const MySetNamesDocument = new TypedDocumentString(`
    query MySetNames {
  mySets {
    id
    name
    kind
    emotes {
      emote {
        id
      }
    }
  }
}
    `) as unknown as TypedDocumentString<MySetNamesQuery, MySetNamesQueryVariables>;
export const SearchByOwnerDocument = new TypedDocumentString(`
    query SearchByOwner($owner: String!, $page: Int, $perPage: Int, $sort: EmoteSort) {
  searchEmotes(
    query: ""
    owner: $owner
    page: $page
    perPage: $perPage
    sort: $sort
  ) {
    total
    page
    perPage
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<SearchByOwnerQuery, SearchByOwnerQueryVariables>;
export const AdminAccountDocument = new TypedDocumentString(`
    query AdminAccount($accountId: String!) {
  adminAccount(accountId: $accountId) {
    id
    displayName
    avatar
    createdAt
    uploadsBlocked
    ticketsBlocked
    blocked
    uploadsBlockedUntil
    ticketsBlockedUntil
    blockedUntil
    identities {
      platform
      platformUserId
      login
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<AdminAccountQuery, AdminAccountQueryVariables>;
export const AccountEmotesDocument = new TypedDocumentString(`
    query AccountEmotes($accountId: String!, $page: Int, $perPage: Int) {
  accountEmotes(accountId: $accountId, page: $page, perPage: $perPage) {
    total
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<AccountEmotesQuery, AccountEmotesQueryVariables>;
export const AccountReportsDocument = new TypedDocumentString(`
    query AccountReports($accountId: String!) {
  accountReports(accountId: $accountId) {
    about {
      ...ReportFields
    }
    by {
      ...ReportFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}
fragment ReportFields on Report {
  id
  reason
  reporterId
  state
  resolution
  createdAt
  resolvedAt
  emote {
    ...EmoteFields
  }
}`) as unknown as TypedDocumentString<AccountReportsQuery, AccountReportsQueryVariables>;
export const PublicAccountDocument = new TypedDocumentString(`
    query PublicAccount($id: String!) {
  account(id: $id) {
    id
    displayName
    avatar
  }
}
    `) as unknown as TypedDocumentString<PublicAccountQuery, PublicAccountQueryVariables>;
export const EmoteChannelsDocument = new TypedDocumentString(`
    query EmoteChannels($id: String!, $limit: Int) {
  emoteChannels(id: $id, limit: $limit) {
    platform
    login
    account {
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<EmoteChannelsQuery, EmoteChannelsQueryVariables>;
export const CosmeticsDocument = new TypedDocumentString(`
    query Cosmetics {
  cosmetics {
    badges {
      ...BadgeFields
    }
    paints {
      ...PaintFields
    }
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<CosmeticsQuery, CosmeticsQueryVariables>;
export const UserCosmeticsDocument = new TypedDocumentString(`
    query UserCosmetics($platform: Platform!, $userIds: [String!]!) {
  userCosmetics(platform: $platform, userIds: $userIds) {
    platformUserId
    badgeId
    paintId
  }
}
    `) as unknown as TypedDocumentString<UserCosmeticsQuery, UserCosmeticsQueryVariables>;
export const AccountCosmeticsDocument = new TypedDocumentString(`
    query AccountCosmetics($accountIds: [String!]!) {
  accountCosmetics(accountIds: $accountIds) {
    accountId
    badge {
      ...BadgeFields
    }
    paint {
      ...PaintFields
    }
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<AccountCosmeticsQuery, AccountCosmeticsQueryVariables>;
export const AccountProfileDocument = new TypedDocumentString(`
    query AccountProfile($accountId: String!) {
  accountProfile(accountId: $accountId) {
    account {
      id
      displayName
      avatar
    }
    identities {
      platform
      login
      displayName
    }
    roles {
      id
      name
    }
    activeSets {
      platform
      login
      setId
      setName
    }
    createdAt
  }
}
    `) as unknown as TypedDocumentString<AccountProfileQuery, AccountProfileQueryVariables>;
export const MyCosmeticsDocument = new TypedDocumentString(`
    query MyCosmetics($accountId: String) {
  myCosmetics(accountId: $accountId) {
    account {
      id
      displayName
      avatar
    }
    badges {
      ...BadgeFields
    }
    paints {
      ...PaintFields
    }
    badgeId
    paintId
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<MyCosmeticsQuery, MyCosmeticsQueryVariables>;
export const SelectCosmeticsDocument = new TypedDocumentString(`
    mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {
  selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {
    badgeId
    paintId
  }
}
    `) as unknown as TypedDocumentString<SelectCosmeticsMutation, SelectCosmeticsMutationVariables>;
export const CosmeticImageDocument = new TypedDocumentString(`
    query CosmeticImage($id: String!) {
  cosmeticImage(id: $id) {
    ...CosmeticImageFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}`) as unknown as TypedDocumentString<CosmeticImageQuery, CosmeticImageQueryVariables>;
export const EntitlementsDocument = new TypedDocumentString(`
    query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {
  cosmeticEntitlements(
    kind: $kind
    accountId: $accountId
    cosmeticId: $cosmeticId
    page: $page
    perPage: $perPage
  ) {
    total
    items {
      account {
        id
        displayName
        avatar
      }
      kind
      cosmeticId
      source
      reason
      grantedBy {
        id
        displayName
      }
      grantedAt
    }
  }
}
    `) as unknown as TypedDocumentString<EntitlementsQuery, EntitlementsQueryVariables>;
export const CreateBadgeDocument = new TypedDocumentString(`
    mutation CreateBadge($input: CreateBadgeInput!) {
  createBadge(input: $input) {
    ...BadgeFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<CreateBadgeMutation, CreateBadgeMutationVariables>;
export const UpdateBadgeDocument = new TypedDocumentString(`
    mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {
  updateBadge(id: $id, input: $input) {
    ...BadgeFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<UpdateBadgeMutation, UpdateBadgeMutationVariables>;
export const DeleteBadgeDocument = new TypedDocumentString(`
    mutation DeleteBadge($id: String!) {
  deleteBadge(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteBadgeMutation, DeleteBadgeMutationVariables>;
export const DeletePaintDocument = new TypedDocumentString(`
    mutation DeletePaint($id: String!) {
  deletePaint(id: $id)
}
    `) as unknown as TypedDocumentString<DeletePaintMutation, DeletePaintMutationVariables>;
export const GrantCosmeticDocument = new TypedDocumentString(`
    mutation GrantCosmetic($input: GrantCosmeticInput!) {
  grantCosmetic(input: $input) {
    cosmeticId
  }
}
    `) as unknown as TypedDocumentString<GrantCosmeticMutation, GrantCosmeticMutationVariables>;
export const RevokeCosmeticDocument = new TypedDocumentString(`
    mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {
  revokeCosmetic(
    accountId: $accountId
    kind: $kind
    cosmeticId: $cosmeticId
    reason: $reason
  )
}
    `) as unknown as TypedDocumentString<RevokeCosmeticMutation, RevokeCosmeticMutationVariables>;
export const AccountRolesDocument = new TypedDocumentString(`
    query AccountRoles($accountId: String!) {
  accountRoles(accountId: $accountId) {
    id
    name
    position
  }
}
    `) as unknown as TypedDocumentString<AccountRolesQuery, AccountRolesQueryVariables>;
export const AccountEditorsDocument = new TypedDocumentString(`
    query AccountEditors($accountId: String) {
  accountEditors(accountId: $accountId) {
    account {
      id
      displayName
      avatar
    }
    permissions
  }
}
    `) as unknown as TypedDocumentString<AccountEditorsQuery, AccountEditorsQueryVariables>;
export const EditorOfDocument = new TypedDocumentString(`
    query EditorOf {
  editorOf {
    account {
      id
      displayName
      avatar
    }
    permissions
  }
}
    `) as unknown as TypedDocumentString<EditorOfQuery, EditorOfQueryVariables>;
export const AccountSetsDocument = new TypedDocumentString(`
    query AccountSets($accountId: String!) {
  accountSets(accountId: $accountId, preview: 12) {
    id
    name
    kind
    capacity
    slotsUsed
    active
    preview {
      emote {
        ...EmoteFields
      }
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<AccountSetsQuery, AccountSetsQueryVariables>;
export const ProfileMineDocument = new TypedDocumentString(`
    query ProfileMine {
  myChannels {
    platform
    platformChannelId
    login
    activeSetId
  }
  mySets {
    ...SetFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}
fragment SetFields on EmoteSet {
  id
  name
  kind
  capacity
  owner {
    id
    displayName
    avatar
  }
  emotes {
    name
    zeroWidth
    addedAt
    emote {
      ...EmoteFields
    }
  }
}`) as unknown as TypedDocumentString<ProfileMineQuery, ProfileMineQueryVariables>;
export const ProfileEditedDocument = new TypedDocumentString(`
    query ProfileEdited {
  editorChannels {
    platform
    platformChannelId
    login
    activeSetId
    account {
      id
    }
  }
  editorSets {
    ...SetFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}
fragment SetFields on EmoteSet {
  id
  name
  kind
  capacity
  owner {
    id
    displayName
    avatar
  }
  emotes {
    name
    zeroWidth
    addedAt
    emote {
      ...EmoteFields
    }
  }
}`) as unknown as TypedDocumentString<ProfileEditedQuery, ProfileEditedQueryVariables>;
export const AccountActivityDocument = new TypedDocumentString(`
    query AccountActivity($accountId: String!, $before: String, $limit: Int) {
  accountActivityPage(accountId: $accountId, before: $before, limit: $limit) {
    entries {
      id
      kind
      actor {
        id
        displayName
        avatar
      }
      set {
        id
        name
      }
      emote {
        id
        name
        host
        files {
          name
          width
          height
          isStatic
        }
      }
      fromName
      toName
      badge {
        fromId
        fromName
        toId
        toName
      }
      paint {
        fromId
        fromName
        toId
        toName
      }
      createdAt
    }
    next
  }
}
    `) as unknown as TypedDocumentString<AccountActivityQuery, AccountActivityQueryVariables>;
export const EmoteActivityDocument = new TypedDocumentString(`
    query EmoteActivity($emoteId: String!, $before: String, $limit: Int) {
  emoteActivityPage(emoteId: $emoteId, before: $before, limit: $limit) {
    entries {
      id
      kind
      actor {
        id
        displayName
        avatar
      }
      set {
        id
        name
      }
      emote {
        id
        name
        host
        files {
          name
          width
          height
          isStatic
        }
      }
      fromName
      toName
      badge {
        fromId
        fromName
        toId
        toName
      }
      paint {
        fromId
        fromName
        toId
        toName
      }
      createdAt
    }
    next
  }
}
    `) as unknown as TypedDocumentString<EmoteActivityQuery, EmoteActivityQueryVariables>;
export const ShopItemsDocument = new TypedDocumentString(`
    query ShopItems($all: Boolean) {
  shopItems(all: $all) {
    ...ShopItemFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}
fragment ShopItemFields on ShopItem {
  id
  name
  description
  priceCents
  currency
  paddlePriceId
  badges {
    ...BadgeFields
  }
  paints {
    ...PaintFields
  }
  activeFrom
  activeTo
  stockLimit
  stockLeft
  visible
  sortOrder
  available
  owned
  deleted
}`) as unknown as TypedDocumentString<ShopItemsQuery, ShopItemsQueryVariables>;
export const ShopItemDocument = new TypedDocumentString(`
    query ShopItem($id: String!) {
  shopItem(id: $id) {
    ...ShopItemFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}
fragment ShopItemFields on ShopItem {
  id
  name
  description
  priceCents
  currency
  paddlePriceId
  badges {
    ...BadgeFields
  }
  paints {
    ...PaintFields
  }
  activeFrom
  activeTo
  stockLimit
  stockLeft
  visible
  sortOrder
  available
  owned
  deleted
}`) as unknown as TypedDocumentString<ShopItemQuery, ShopItemQueryVariables>;
export const StartCheckoutDocument = new TypedDocumentString(`
    mutation StartCheckout($itemId: String!, $recipient: RecipientInput) {
  startCheckout(itemId: $itemId, recipient: $recipient) {
    order {
      ...OrderFields
    }
    transactionId
    checkoutUrl
    clientToken
    environment
  }
}
    fragment OrderFields on Order {
  id
  buyer {
    id
    displayName
    avatar
  }
  recipient {
    id
    displayName
    avatar
  }
  gift
  itemId
  itemName
  status
  amountCents
  currency
  paidTotalCents
  paidCurrency
  refundReason
  createdAt
  paidAt
  refundedAt
  failedAt
}`) as unknown as TypedDocumentString<StartCheckoutMutation, StartCheckoutMutationVariables>;
export const OrderDocument = new TypedDocumentString(`
    query Order($id: String!) {
  order(id: $id) {
    ...OrderFields
  }
}
    fragment OrderFields on Order {
  id
  buyer {
    id
    displayName
    avatar
  }
  recipient {
    id
    displayName
    avatar
  }
  gift
  itemId
  itemName
  status
  amountCents
  currency
  paidTotalCents
  paidCurrency
  refundReason
  createdAt
  paidAt
  refundedAt
  failedAt
}`) as unknown as TypedDocumentString<OrderQuery, OrderQueryVariables>;
export const MyOrdersDocument = new TypedDocumentString(`
    query MyOrders($page: Int, $perPage: Int) {
  myOrders(page: $page, perPage: $perPage) {
    total
    items {
      ...OrderFields
    }
  }
}
    fragment OrderFields on Order {
  id
  buyer {
    id
    displayName
    avatar
  }
  recipient {
    id
    displayName
    avatar
  }
  gift
  itemId
  itemName
  status
  amountCents
  currency
  paidTotalCents
  paidCurrency
  refundReason
  createdAt
  paidAt
  refundedAt
  failedAt
}`) as unknown as TypedDocumentString<MyOrdersQuery, MyOrdersQueryVariables>;
export const RedeemCodeDocument = new TypedDocumentString(`
    mutation RedeemCode($code: String!) {
  redeemCode(code: $code) {
    name
    badges {
      ...BadgeFields
    }
    paints {
      ...PaintFields
    }
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}`) as unknown as TypedDocumentString<RedeemCodeMutation, RedeemCodeMutationVariables>;
export const CreateShopItemDocument = new TypedDocumentString(`
    mutation CreateShopItem($input: ShopItemInput!) {
  createShopItem(input: $input) {
    ...ShopItemFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}
fragment ShopItemFields on ShopItem {
  id
  name
  description
  priceCents
  currency
  paddlePriceId
  badges {
    ...BadgeFields
  }
  paints {
    ...PaintFields
  }
  activeFrom
  activeTo
  stockLimit
  stockLeft
  visible
  sortOrder
  available
  owned
  deleted
}`) as unknown as TypedDocumentString<CreateShopItemMutation, CreateShopItemMutationVariables>;
export const UpdateShopItemDocument = new TypedDocumentString(`
    mutation UpdateShopItem($id: String!, $input: ShopItemInput!) {
  updateShopItem(id: $id, input: $input) {
    ...ShopItemFields
  }
}
    fragment CosmeticImageFields on CosmeticImage {
  id
  kind
  host
  animated
  ready
  width
  height
  files {
    name
    width
    height
    isStatic
  }
}
fragment BadgeFields on Badge {
  id
  name
  tooltip
  image {
    ...CosmeticImageFields
  }
}
fragment PaintFields on Paint {
  id
  name
  kind
  angle
  repeat
  shape
  color
  stops {
    at
    color
  }
  shadows {
    x
    y
    radius
    color
  }
  image {
    ...CosmeticImageFields
  }
}
fragment ShopItemFields on ShopItem {
  id
  name
  description
  priceCents
  currency
  paddlePriceId
  badges {
    ...BadgeFields
  }
  paints {
    ...PaintFields
  }
  activeFrom
  activeTo
  stockLimit
  stockLeft
  visible
  sortOrder
  available
  owned
  deleted
}`) as unknown as TypedDocumentString<UpdateShopItemMutation, UpdateShopItemMutationVariables>;
export const DeleteShopItemDocument = new TypedDocumentString(`
    mutation DeleteShopItem($id: String!) {
  deleteShopItem(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteShopItemMutation, DeleteShopItemMutationVariables>;
export const AdminOrdersDocument = new TypedDocumentString(`
    query AdminOrders($status: OrderStatus, $accountId: String, $itemId: String, $page: Int, $perPage: Int) {
  orders(
    status: $status
    accountId: $accountId
    itemId: $itemId
    page: $page
    perPage: $perPage
  ) {
    total
    items {
      ...OrderFields
      providerTransactionId
      refundRejectedAt
    }
  }
}
    fragment OrderFields on Order {
  id
  buyer {
    id
    displayName
    avatar
  }
  recipient {
    id
    displayName
    avatar
  }
  gift
  itemId
  itemName
  status
  amountCents
  currency
  paidTotalCents
  paidCurrency
  refundReason
  createdAt
  paidAt
  refundedAt
  failedAt
}`) as unknown as TypedDocumentString<AdminOrdersQuery, AdminOrdersQueryVariables>;
export const RefundOrderDocument = new TypedDocumentString(`
    mutation RefundOrder($id: String!, $reason: String!) {
  refundOrder(id: $id, reason: $reason) {
    ...OrderFields
    providerTransactionId
    refundRejectedAt
  }
}
    fragment OrderFields on Order {
  id
  buyer {
    id
    displayName
    avatar
  }
  recipient {
    id
    displayName
    avatar
  }
  gift
  itemId
  itemName
  status
  amountCents
  currency
  paidTotalCents
  paidCurrency
  refundReason
  createdAt
  paidAt
  refundedAt
  failedAt
}`) as unknown as TypedDocumentString<RefundOrderMutation, RefundOrderMutationVariables>;
export const CodeBatchesDocument = new TypedDocumentString(`
    query CodeBatches($page: Int, $perPage: Int) {
  codeBatches(page: $page, perPage: $perPage) {
    total
    items {
      ...CodeBatchFields
    }
  }
}
    fragment CodeBatchFields on CodeBatch {
  id
  name
  prefix
  itemId
  itemName
  badgeIds
  paintIds
  codeCount
  usesPerCode
  redemptions
  expiresAt
  disabledAt
  createdBy {
    id
    displayName
  }
  createdAt
}`) as unknown as TypedDocumentString<CodeBatchesQuery, CodeBatchesQueryVariables>;
export const CreateCodeBatchDocument = new TypedDocumentString(`
    mutation CreateCodeBatch($input: CodeBatchInput!) {
  createCodeBatch(input: $input) {
    batch {
      ...CodeBatchFields
    }
    codes
  }
}
    fragment CodeBatchFields on CodeBatch {
  id
  name
  prefix
  itemId
  itemName
  badgeIds
  paintIds
  codeCount
  usesPerCode
  redemptions
  expiresAt
  disabledAt
  createdBy {
    id
    displayName
  }
  createdAt
}`) as unknown as TypedDocumentString<CreateCodeBatchMutation, CreateCodeBatchMutationVariables>;
export const DisableCodeBatchDocument = new TypedDocumentString(`
    mutation DisableCodeBatch($id: String!) {
  disableCodeBatch(id: $id) {
    ...CodeBatchFields
  }
}
    fragment CodeBatchFields on CodeBatch {
  id
  name
  prefix
  itemId
  itemName
  badgeIds
  paintIds
  codeCount
  usesPerCode
  redemptions
  expiresAt
  disabledAt
  createdBy {
    id
    displayName
  }
  createdAt
}`) as unknown as TypedDocumentString<DisableCodeBatchMutation, DisableCodeBatchMutationVariables>;
export const ShopCountsDocument = new TypedDocumentString(`
    query ShopCounts($items: Boolean!, $orders: Boolean!, $codes: Boolean!) {
  shopItems(all: true) @include(if: $items) {
    id
  }
  orders(page: 1, perPage: 1) @include(if: $orders) {
    total
  }
  codeBatches(page: 1, perPage: 1) @include(if: $codes) {
    total
  }
}
    `) as unknown as TypedDocumentString<ShopCountsQuery, ShopCountsQueryVariables>;
export const QuickSearchDocument = new TypedDocumentString(`
    query QuickSearch($query: String!) {
  quickSearch(query: $query) {
    emotes {
      ...EmoteFields
    }
    users {
      account {
        id
        displayName
        avatar
      }
      identities {
        platform
        login
        displayName
      }
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    nsfw
    private
  }
  owner {
    id
    displayName
    avatar
  }
  files {
    name
    format
    scale
    width
    height
    size
    isStatic
  }
}`) as unknown as TypedDocumentString<QuickSearchQuery, QuickSearchQueryVariables>;
export const EditableChannelsDocument = new TypedDocumentString(`
    query EditableChannels {
  myEditableChannels {
    role
    channel {
      platform
      login
      account {
        id
        displayName
        avatar
      }
    }
  }
}
    `) as unknown as TypedDocumentString<EditableChannelsQuery, EditableChannelsQueryVariables>;