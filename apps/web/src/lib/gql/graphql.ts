/* eslint-disable */
/** Internal type. DO NOT USE DIRECTLY. */
type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
/** Internal type. DO NOT USE DIRECTLY. */
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
import type { DocumentTypeDecoration } from '@graphql-typed-document-node/core';
/**
 * Narrows `accountActivityPage`. Every field is optional; an empty filter is the whole feed
 * (without editor lines, as before filters existed).
 */
export type ActivityFilter = {
  /** Only lines by this account (the owner or one of its editors). */
  actorId?: string | null | undefined;
  /**
   * Only these groups. Editor lines (`EDITOR_ADDED`, `EDITOR_REMOVED`) show only when this
   * lists `EDITORS`.
   */
  groups?: Array<ActivityGroup> | null | undefined;
  /** Only lines at or after this time (RFC 3339). */
  since?: string | null | undefined;
  /** Only lines before this time (RFC 3339). */
  until?: string | null | undefined;
};

/** A group of activity lines, for filtering a feed. */
export type ActivityGroup =
  /** Badge and paint changes. */
  | 'COSMETICS'
  /** Account editors added and removed. */
  | 'EDITORS'
  /** Emotes added to, removed from or renamed in sets, and uploads that went public. */
  | 'EMOTES'
  /** Sets made or renamed, and active set changes. */
  | 'SETS';

/** What one line of an account's public activity feed says happened. */
export type ActivityKind =
  /** A channel's active set changed (`set` is the new one; `from_name`, `to_name`). */
  | 'ACTIVE_SET_CHANGED'
  /** The badge and/or paint shown changed (`badge`, `paint`). */
  | 'COSMETICS_SELECTED'
  /**
   * Someone became an account editor (`editor`). Never their permissions. Only listed when
   * the filter asks for `EDITORS`.
   */
  | 'EDITOR_ADDED'
  /**
   * An account editor was removed or left (`editor`). Only listed when the filter asks for
   * `EDITORS`.
   */
  | 'EDITOR_REMOVED'
  /** An emote of the account was approved and listed. */
  | 'EMOTE_UPLOADED'
  | 'SET_CREATED'
  | 'SET_EMOTE_ADDED'
  | 'SET_EMOTE_REMOVED'
  /** The emote's name in the set changed (`from_name`, `to_name`). */
  | 'SET_EMOTE_RENAMED'
  /** A set was renamed (`from_name`, `to_name`). */
  | 'SET_RENAMED';

/** Who to add as an editor: an account ID, or a Twitch or Kick login. */
export type AddAccountEditorInput = {
  /** The editor's cyex.tv account ID. Or give `platform` and `login`. */
  accountId?: string | null | undefined;
  login?: string | null | undefined;
  /** The account to add editors to. Empty: your own. */
  ownerId?: string | null | undefined;
  /** At least one. */
  permissions: Array<EditorPermission>;
  platform?: Platform | null | undefined;
  /** Limit them to these sets of the account. Empty or missing: every set, new ones included. */
  setIds?: Array<string> | null | undefined;
};

export type AddSetEmoteInput = {
  /** Rename it in this set. Defaults to the emote's own name. */
  name?: string | null | undefined;
  zeroWidth?: boolean | null | undefined;
};

/** What `createAnnouncement` makes and `updateAnnouncement` replaces it with. */
export type AnnouncementInput = {
  /** A badge's ID. */
  badgeId?: string | null | undefined;
  /** An approved emote's ID. */
  emoteId?: string | null | undefined;
  /** RFC 3339, in the future and after `startsAt`. */
  endsAt?: string | null | undefined;
  kind: AnnouncementKind;
  /** Up to 40 characters; only with `linkUrl`. */
  linkLabel?: string | null | undefined;
  /** `https://…` or a path on cyex.tv starting with `/`, up to 500 characters. */
  linkUrl?: string | null | undefined;
  /** RFC 3339. */
  startsAt?: string | null | undefined;
  /** 1 to 200 characters, one line. */
  text: string;
};

/** What an announcement is about: its icon and tone under the header. */
export type AnnouncementKind =
  /** An event or a sale, often with an emote or a badge. */
  | 'CAMPAIGN'
  /** News, a tip. */
  | 'INFO'
  /** Downtime or something not working. */
  | 'MAINTENANCE';

/** Which side of the audit log `auditTopPeople` counts. */
export type AuditPersonRole =
  /** Who made the changes. */
  | 'ACTOR'
  /** The accounts the changes were about (target kind `account`). */
  | 'TARGET';

/** The roles cyex.tv comes with (ADR 0010). */
export type BuiltInRole =
  /** "Admin": can't be deleted. */
  | 'ADMIN'
  /** The top role: every permission, held by the owners only, never granted or edited. */
  | 'ROOT'
  /** "User": every account holds it without an assignment. Starts with no permissions. */
  | 'USER';

/** Who you are on a channel you can manage, highest first. */
export type ChannelRole =
  /** You are an editor of the channel's account. */
  | 'ACCOUNT_EDITOR'
  /** The channel is yours. */
  | 'OWNER'
  /** You edit at least one set of the channel's account. */
  | 'SET_EDITOR';

export type CodeBatchInput = {
  badgeIds?: Array<string>;
  /** 1 to 1000 codes. */
  count: number;
  /** RFC 3339, in the future. */
  expiresAt?: string | null | undefined;
  /** A shop item whose cosmetics a code gives. */
  itemId?: string | null | undefined;
  /** For staff, 1 to 64 characters. */
  name: string;
  paintIds?: Array<string>;
  /** 1 to 12 letters or digits (upper-cased). */
  prefix: string;
  /** 1 to 10000, default 1. Each account can use a code once. */
  usesPerCode?: number | null | undefined;
};

/** A badge or a paint. */
export type CosmeticKind =
  | 'BADGE'
  | 'PAINT';

export type CreateBadgeInput = {
  /** A ready badge image from `POST /v1/cosmetics/images`. */
  imageId: string;
  /** 1 to 32 characters, unique. */
  name: string;
  /** 1 to 64 characters. */
  tooltip: string;
};

export type CreateRoleInput = {
  /** What it denies. Never also in `permissions` (`bad_permissions`). */
  denies?: Array<string> | null | undefined;
  description?: string | null | undefined;
  /** Never show it publicly, and leave it out wherever roles rank people. Default false. */
  invisible?: boolean | null | undefined;
  name: string;
  permissions: Array<string>;
  position: number;
  /** Show it on its holders' public profiles. Default false. */
  public?: boolean | null | undefined;
};

export type CreateSetInput = {
  /** `CHANNEL` or `PERSONAL`. The global set exists already. */
  kind: SetKind;
  name: string;
  /** Make the set for an account you edit with `sets` (ADR 0014). Empty: for yourself. */
  ownerId?: string | null | undefined;
  /**
   * Empty or `LISTED`: every set is public. `UNLISTED` and `PRIVATE` need `sets.edit_any`
   * (staff); personal sets can't be `PRIVATE`.
   */
  visibility?: SetVisibility | null | undefined;
};

/** What an account editor may do on the account they edit. The account's owner has all of them. */
export type EditorPermission =
  /** Manage the owner's payment methods. Stored; grants nothing yet. */
  | 'BILLING_PAYMENT'
  /** Manage the owner's subscription. Stored; grants nothing yet. */
  | 'BILLING_SUBSCRIPTION'
  /** Change the owner's channels' settings (and pick their active set, as `Sets` does). */
  | 'CHANNELS'
  /** Add, change and remove editors, never granting more than you hold. */
  | 'EDITORS'
  /** Add, remove, rename and alias emotes in the owner's sets, the active channel sets too. */
  | 'EMOTES'
  /** See, rename, retag and delete the owner's emotes. */
  | 'EMOTE_MANAGE'
  /**
   * Retired (ADR 0014, amendment 2026/11/22): only an emote's owner gives it away, and nobody
   * can ask for someone else's emote to be transferred. Kept so clients that send it still
   * parse; it is never granted, never listed and grants nothing (stored grants ignore it).
   */
  | 'EMOTE_TRANSFER'
  /** Upload emotes for the owner (`POST /v1/emotes` with `owner_id`). */
  | 'EMOTE_UPLOAD'
  /** Pick the badge and paint the owner shows (ADR 0015). Never grants. */
  | 'PROFILE'
  /** Create, rename and delete sets, change their capacity, and pick a channel's active set. */
  | 'SETS';

/** A flag `searchEmotes` can require. */
export type EmoteFlag =
  /** Retired: NSFW emotes aren't allowed, so this finds nothing. */
  | 'NSFW'
  /** About twice as wide as tall or more. */
  | 'WIDE'
  /** Overlays the emote before it. */
  | 'ZERO_WIDTH';

/** What `setEmoteFlags` changes (ADR 0024). Every field is optional: what is left out stays. */
export type EmoteFlagsInput = {
  /**
   * Retired: NSFW emotes aren't allowed. `false` clears an old flag (any change here clears
   * it too); `true` is `nsfw_not_allowed`.
   * @deprecated NSFW emotes aren't allowed on cyex.tv. true is refused (nsfw_not_allowed); saving flags clears an old NSFW flag.
   */
  nsfw?: boolean | null | undefined;
  /** `UNLISTED` takes a public emote off search, browse and its creator's page. */
  visibility?: EmoteVisibility | null | undefined;
  zeroWidth?: boolean | null | undefined;
};

/**
 * Whether an emote's image is made. Separate from [`EmoteState`] (review), so adding it
 * breaks nobody: a client that doesn't know it keeps showing `Processing` as before.
 */
export type EmoteProcessing =
  /** The transcode ran out of attempts: the emote never gets files. Delete it or upload again. */
  | 'FAILED'
  /** The transcode job is queued or running. */
  | 'PROCESSING'
  /** The files are made. */
  | 'READY';

/** How `searchEmotes` orders its results. */
export type EmoteSort =
  /** The newest first. */
  | 'NEW'
  /** Best match first, then the most used. The default. */
  | 'RELEVANCE'
  /** The most used first (channels showing the emote), then by name. */
  | 'TOP'
  /** Most channels added it in the `window` (net of removals), then the most used. */
  | 'TRENDING';

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
   * its creator's page leave it out for everyone but its owner. Set by staff only (a
   * moderator taking an emote off the listings); creators choose public or private.
   */
  | 'UNLISTED';

/**
 * How an account came to own a cosmetic: given by staff, bought in the shop (ADR 0016), from an
 * event, or from a redeem code.
 */
export type EntitlementSource =
  | 'CODE'
  | 'EVENT'
  | 'GRANT'
  | 'PURCHASE';

export type GrantCosmeticInput = {
  accountId: string;
  cosmeticId: string;
  kind: CosmeticKind;
  /** Why, for the audit log. 1 to 500 characters. */
  reason: string;
};

/** What an entitlement is of. */
export type GrantKind =
  | 'BADGE'
  | 'PAINT'
  | 'ROLE';

/** How an account came to hold something. */
export type GrantSource =
  /** A redeemed code (`codeBatchId`). */
  | 'CODE'
  /** The User role every account holds. */
  | 'DEFAULT'
  /** An event. */
  | 'EVENT'
  /** The owners' top role, from the owner list. */
  | 'OWNER'
  /** Bought in the shop (`orderId`). */
  | 'PURCHASE'
  /** Given by staff (a cosmetic grant or a role assignment). */
  | 'STAFF';

export type ImageFormat =
  | 'AVIF'
  | 'WEBP';

/** Which part of the site a limit covers, for grouping them on the role page. */
export type LimitGroup =
  /** Writes to your account: editors, cosmetics, sets, lists, settings, the shop, your data. */
  | 'ACCOUNT'
  /** How often an account may call the API and search. */
  | 'REQUESTS'
  /** Uploads an hour, and how many may wait for review. */
  | 'UPLOADS';

export type NotificationPrefInput = {
  kind: string;
  notify: boolean;
  toast: boolean;
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

/** A whole paint: `createPaint` makes one, `updatePaint` replaces every field. */
export type PaintInput = {
  /** 0 to 360, default 90. */
  angle?: number | null | undefined;
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color?: string | null | undefined;
  /** A ready paint image; `IMAGE` paints only. */
  imageId?: string | null | undefined;
  kind: PaintKind;
  /** 1 to 32 characters, unique. */
  name: string;
  /** Default false. */
  repeat?: boolean | null | undefined;
  /** At most 4. */
  shadows?: Array<PaintShadowInput>;
  /** Default `CIRCLE`. */
  shape?: PaintShape | null | undefined;
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
export type PaintShadowInput = {
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: string;
  /** Blur radius, 0 to 16. */
  radius: number;
  /** −16 to 16. */
  x: number;
  /** −16 to 16. */
  y: number;
};

/** The shape of a radial paint. */
export type PaintShape =
  | 'CIRCLE'
  | 'ELLIPSE';

/** One colour stop of a gradient paint. */
export type PaintStopInput = {
  /** Where along the gradient, 0 to 1. Each stop is at or after the one before. */
  at: number;
  /** `#RRGGBB` or `#RRGGBBAA`. */
  color: string;
};

/** What a role says about a permission (`PermissionDecision.effect`). */
export type PermissionEffect =
  | 'ALLOW'
  | 'DENY';

/** A streaming platform. */
export type Platform =
  | 'KICK'
  | 'TWITCH';

/** Who gets a gift: their Twitch or Kick login. */
export type RecipientInput = {
  login: string;
  platform: Platform;
};

/** What a moderator does with a report. */
export type ReportAction =
  | 'DISMISS'
  /** Take the emote down (needs `emotes.takedown`). */
  | 'TAKE_DOWN';

export type ReportState =
  /** The emote was taken down. */
  | 'ACTIONED'
  | 'DISMISSED'
  | 'OPEN';

/** A restriction to place. */
export type RestrictionInput = {
  kind: RestrictionKind;
  /** Why: 1 to 500 characters. Staff see it; the account doesn't. */
  reason: string;
  /** When it ends (RFC 3339, in the future, at most ten years out); empty for good. */
  until?: string | null | undefined;
};

/** What a restriction stops. */
export type RestrictionKind =
  /** The whole account: no sign-in, sessions ended (`users.block`). */
  | 'ACCOUNT'
  /** Filing reports and tickets (`users.restrict_tickets`). */
  | 'TICKETS'
  /** Uploading emotes (`users.restrict_uploads`). */
  | 'UPLOADS';

/** What `reviewQueue` lists. Every field is optional: none is every pending emote, oldest first. */
export type ReviewQueueFilter = {
  /** Newest (or Z) first. Default: oldest first, except `RESOLVED`, latest review first. */
  descending?: boolean | null | undefined;
  /** Part of the emote's name or of its uploader's login or display name, any case. */
  search?: string | null | undefined;
  sort?: ReviewSort | null | undefined;
  tab?: ReviewTab | null | undefined;
  /** Only this uploader's emotes (an account ID). */
  uploaderId?: string | null | undefined;
};

/** How the review queue is ordered. Ties go by upload time. */
export type ReviewSort =
  /** The emote's name. */
  | 'NAME'
  /** Review time. The default for `RESOLVED`. */
  | 'REVIEWED'
  /** Upload time. The default for the pending tabs. */
  | 'UPLOADED'
  /** The uploader's display name. */
  | 'UPLOADER';

/** A part of the review queue. */
export type ReviewTab =
  /** Pending and unlisted or private. */
  | 'LIMITED'
  /** Every pending emote. The default. */
  | 'PENDING'
  /** Pending and public: search shows it once approved. */
  | 'PUBLIC'
  /** Approved or rejected by a reviewer. */
  | 'RESOLVED';

/** What a batch review does to every emote in it. */
export type ReviewVerdict =
  | 'APPROVE'
  | 'REJECT';

/** One override for `setRoleLimits`. */
export type RoleLimitInput = {
  key: string;
  /** 1 to 1,000,000; null keeps the default. */
  value?: number | null | undefined;
};

/** How a session signed in. */
export type SessionKind =
  /** A Bearer token for an app (cyex deck, the extension). */
  | 'APP'
  /** A personal API token you made on cyex.tv (`createConsoleToken`, ADR 0028). */
  | 'CONSOLE'
  /** The cyex.tv website's cookie. */
  | 'SITE';

export type SetKind =
  | 'CHANNEL'
  | 'GLOBAL'
  | 'PERSONAL';

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

/** A whole item: `createShopItem` makes one, `updateShopItem` replaces every field. */
export type ShopItemInput = {
  /** RFC 3339. */
  activeFrom?: string | null | undefined;
  /** RFC 3339, after `activeFrom`. */
  activeTo?: string | null | undefined;
  /** 1 to 10 cosmetics in all. */
  badgeIds?: Array<string>;
  /** ISO 4217, default `EUR`. */
  currency?: string | null | undefined;
  /** Up to 500 characters; new lines allowed. */
  description?: string | null | undefined;
  /** 1 to 64 characters. */
  name: string;
  /** The Paddle price (`pri_…`). Without one, checkout says it isn't available. */
  paddlePriceId?: string | null | undefined;
  paintIds?: Array<string>;
  /** 0 to 100000 (1000.00). */
  priceCents: number;
  /** Default 0. */
  sortOrder?: number | null | undefined;
  /** 1 or more; none: no limit. */
  stockLimit?: number | null | undefined;
  /** Default false. */
  visible?: boolean | null | undefined;
};

/** Where a subscription is in time. */
export type SubscriptionState =
  | 'CURRENT'
  | 'FUTURE'
  | 'PAST';

/** How far back `TRENDING` counts channel adds. */
export type TrendingWindow =
  /** The last day. */
  | 'DAY'
  /** The last 30 days. */
  | 'MONTH'
  /** The last 7 days. The default. */
  | 'WEEK';

export type UpdateBadgeInput = {
  /** Another ready badge image. */
  imageId?: string | null | undefined;
  name?: string | null | undefined;
  tooltip?: string | null | undefined;
};

export type UpdateChannelSettingsInput = {
  /** One of the owner's channel sets. An empty string clears it. */
  activeSetId?: string | null | undefined;
  allowPersonalEmotes?: boolean | null | undefined;
};

export type UpdateEmoteInput = {
  name?: string | null | undefined;
  /**
   * Retired: NSFW emotes aren't allowed. `false` is ignored; `true` is `nsfw_not_allowed`.
   * @deprecated NSFW emotes aren't allowed on cyex.tv. false is ignored; true is refused (nsfw_not_allowed).
   */
  nsfw?: boolean | null | undefined;
  /**
   * `true` makes it private; `false` makes a private emote public (an unlisted one stays
   * unlisted). Prefer `visibility`.
   */
  private?: boolean | null | undefined;
  tags?: Array<string> | null | undefined;
  /**
   * `PUBLIC` or `PRIVATE`. With `private` too, the two must agree (`bad_visibility`).
   * `UNLISTED` (and changing an unlisted emote) is for staff with `emotes.flags` or
   * `emotes.edit_any`: a moderator takes an emote off the listings, not its owner.
   */
  visibility?: EmoteVisibility | null | undefined;
  zeroWidth?: boolean | null | undefined;
};

export type UpdateRoleInput = {
  /**
   * Replaces what it denies. A name in both lists is `bad_permissions`; one given in only one
   * of them leaves the other list (moving a name there from the other is fine).
   */
  denies?: Array<string> | null | undefined;
  /** An empty string clears it. */
  description?: string | null | undefined;
  /** Never show it publicly, and leave it out wherever roles rank people. */
  invisible?: boolean | null | undefined;
  name?: string | null | undefined;
  /** Replaces what it allows. */
  permissions?: Array<string> | null | undefined;
  position?: number | null | undefined;
  /** Show it on its holders' public profiles. */
  public?: boolean | null | undefined;
};

export type UpdateSetEmoteInput = {
  name?: string | null | undefined;
  zeroWidth?: boolean | null | undefined;
};

export type UpdateSetInput = {
  /**
   * 1 to the owner's `set_capacity_max` role limit (1.000 by default; 5 for a personal set),
   * never below the emotes the set holds; up to 1.000.000 with `sets.edit_any`.
   * `setCapacityRange` says what you may pick.
   */
  capacity?: number | null | undefined;
  name?: string | null | undefined;
  /**
   * Every set is public: changing it needs `sets.edit_any` (staff hiding a set). Personal sets
   * can't be `PRIVATE`; the global set is always listed.
   */
  visibility?: SetVisibility | null | undefined;
};

export type StudioPaintFieldsFragment = { id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, updatedAt: string, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null };

export type StudioPaintsQueryVariables = Exact<{ [key: string]: never; }>;


export type StudioPaintsQuery = { cosmetics: { paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, updatedAt: string, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null }> } };

export type StudioCreatePaintMutationVariables = Exact<{
  input: PaintInput;
}>;


export type StudioCreatePaintMutation = { createPaint: { id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, updatedAt: string, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null } };

export type StudioUpdatePaintMutationVariables = Exact<{
  id: string;
  input: PaintInput;
}>;


export type StudioUpdatePaintMutation = { updatePaint: { id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, updatedAt: string, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null } };

export type StudioCosmeticImageQueryVariables = Exact<{
  id: string;
}>;


export type StudioCosmeticImageQuery = { cosmeticImage: { id: string, host: string, ready: boolean, width: number, height: number, animated: boolean, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, isStatic: boolean }> } | null };

export type SetDisplayPlatformMutationVariables = Exact<{
  platform: Platform;
}>;


export type SetDisplayPlatformMutation = { setDisplayPlatform: { id: string, displayPlatform: Platform | null } };

export type SignOutEverywhereMutationVariables = Exact<{ [key: string]: never; }>;


export type SignOutEverywhereMutation = { signOutEverywhere: number };

export type UnlinkMutationVariables = Exact<{
  platform: Platform;
}>;


export type UnlinkMutation = { unlinkIdentity: { id: string } };

export type MyNotificationsQueryVariables = Exact<{ [key: string]: never; }>;


export type MyNotificationsQuery = { myNotifications: Array<{ id: string, kind: string, payload: string, createdAt: string, read: boolean }> };

export type MarkReadMutationVariables = Exact<{
  ids?: Array<string> | string | null | undefined;
}>;


export type MarkReadMutation = { markNotificationsRead: number };

export type UnreadCountQueryVariables = Exact<{ [key: string]: never; }>;


export type UnreadCountQuery = { unreadNotificationCount: number };

export type MySessionsQueryVariables = Exact<{ [key: string]: never; }>;


export type MySessionsQuery = { mySessions: Array<{ id: string, kind: SessionKind, device: string, createdAt: string, lastSeenAt: string, expiresAt: string, current: boolean }> };

export type MyConsoleTokenQueryVariables = Exact<{ [key: string]: never; }>;


export type MyConsoleTokenQuery = { myConsoleToken: { id: string, createdAt: string, expiresAt: string } | null };

export type RevokeSessionMutationVariables = Exact<{
  id: string;
}>;


export type RevokeSessionMutation = { revokeSession: boolean };

export type RevokeOtherSessionsMutationVariables = Exact<{ [key: string]: never; }>;


export type RevokeOtherSessionsMutation = { revokeOtherSessions: number };

export type ExportMyDataMutationVariables = Exact<{ [key: string]: never; }>;


export type ExportMyDataMutation = { exportMyData: string };

export type DeleteAccountMutationVariables = Exact<{
  confirm: string;
}>;


export type DeleteAccountMutation = { deleteAccount: boolean };

export type PendingDeletionQueryVariables = Exact<{ [key: string]: never; }>;


export type PendingDeletionQuery = { pendingDeletion: { requestedAt: string, eraseAfter: string, displayName: string | null, avatar: string | null } | null };

export type RestoreAccountMutationVariables = Exact<{ [key: string]: never; }>;


export type RestoreAccountMutation = { restoreAccount: boolean };

export type MyRolesQueryVariables = Exact<{ [key: string]: never; }>;


export type MyRolesQuery = { me: { id: string, roles: Array<{ id: string, name: string, public: boolean, invisible: boolean }> } };

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { me: { id: string, createdAt: string, permissions: Array<string>, displayPlatform: Platform | null, identities: Array<{ platform: Platform, platformUserId: string, login: string, displayName: string, avatar: string | null }> } };

export type AccountActivityQueryVariables = Exact<{
  accountId: string;
  before?: string | null | undefined;
  limit?: number | null | undefined;
  filter?: ActivityFilter | null | undefined;
}>;


export type AccountActivityQuery = { accountActivityPage: { next: string | null, entries: Array<{ id: string, kind: ActivityKind, fromName: string | null, toName: string | null, createdAt: string, actor: { id: string, displayName: string, avatar: string | null }, editor: { id: string, displayName: string, avatar: string | null } | null, set: { id: string, name: string } | null, emote: { id: string, name: string, host: string, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null, badge: { fromId: string | null, fromName: string | null, toId: string | null, toName: string | null } | null, paint: { fromId: string | null, fromName: string | null, toId: string | null, toName: string | null } | null }> } };

export type EmoteActivityQueryVariables = Exact<{
  emoteId: string;
  before?: string | null | undefined;
  limit?: number | null | undefined;
  actorId?: string | null | undefined;
}>;


export type EmoteActivityQuery = { emoteActivityPage: { next: string | null, entries: Array<{ id: string, kind: ActivityKind, fromName: string | null, toName: string | null, createdAt: string, actor: { id: string, displayName: string, avatar: string | null }, set: { id: string, name: string } | null, emote: { id: string, name: string, host: string, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null, badge: { fromId: string | null, fromName: string | null, toId: string | null, toName: string | null } | null, paint: { fromId: string | null, fromName: string | null, toId: string | null, toName: string | null } | null }> } };

export type UserQueryVariables = Exact<{
  platform: Platform;
  login: string;
}>;


export type UserQuery = { channelByLogin: { platform: Platform, platformChannelId: string, login: string, account: { id: string, displayName: string, avatar: string | null } } | null };

export type UserDetailsQueryVariables = Exact<{
  accountId: string;
  status: boolean;
  roles: boolean;
}>;


export type UserDetailsQuery = { accountStatus?: { uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil: string | null, ticketsBlockedUntil: string | null, blockedUntil: string | null }, accountRoles?: Array<{ id: string, name: string, description: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, builtIn: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type AccountPermissionsQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountPermissionsQuery = { accountPermissions: Array<{ permission: string, granted: boolean, effect: PermissionEffect | null, roleId: string | null, roleName: string | null, owner: boolean }> };

export type AllRolesQueryVariables = Exact<{ [key: string]: never; }>;


export type AllRolesQuery = { roles: Array<{ id: string, name: string, description: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, builtIn: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type PlaceRestrictionMutationVariables = Exact<{
  accountId: string;
  input: RestrictionInput;
}>;


export type PlaceRestrictionMutation = { placeRestriction: { uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil: string | null, ticketsBlockedUntil: string | null, blockedUntil: string | null } };

export type LiftRestrictionMutationVariables = Exact<{
  accountId: string;
  kind: RestrictionKind;
  reason?: string | null | undefined;
}>;


export type LiftRestrictionMutation = { liftRestriction: { uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil: string | null, ticketsBlockedUntil: string | null, blockedUntil: string | null } };

export type AccountRestrictionsQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountRestrictionsQuery = { accountRestrictions: Array<{ id: string, kind: RestrictionKind, reason: string | null, endsAt: string | null, createdBy: string | null, createdAt: string, liftedAt: string | null, liftedBy: string | null, active: boolean }> };

export type AssignRoleMutationVariables = Exact<{
  accountId: string;
  roleId: string;
}>;


export type AssignRoleMutation = { assignRole: boolean };

export type UnassignRoleMutationVariables = Exact<{
  accountId: string;
  roleId: string;
}>;


export type UnassignRoleMutation = { unassignRole: boolean };

export type AuditQueryVariables = Exact<{
  targetKind?: string | null | undefined;
  targetId?: string | null | undefined;
  actorId?: string | null | undefined;
  action?: string | null | undefined;
  since?: string | null | undefined;
  until?: string | null | undefined;
  before?: string | null | undefined;
  limit?: number | null | undefined;
}>;


export type AuditQuery = { auditLog: Array<{ id: string, actorId: string | null, action: string, targetKind: string, targetId: string, before: string | null, after: string | null, createdAt: string }> };

export type ChannelQueryVariables = Exact<{
  platform: Platform;
  login: string;
}>;


export type ChannelQuery = { channelByLogin: { platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, allowPersonalEmotes: boolean, account: { id: string, displayName: string, avatar: string | null } } | null };

export type CosmeticImageFieldsFragment = { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> };

export type BadgeFieldsFragment = { id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } };

export type PaintFieldsFragment = { id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null };

export type CosmeticsQueryVariables = Exact<{ [key: string]: never; }>;


export type CosmeticsQuery = { cosmetics: { badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type UserCosmeticsQueryVariables = Exact<{
  platform: Platform;
  userIds: Array<string> | string;
}>;


export type UserCosmeticsQuery = { userCosmetics: Array<{ platformUserId: string, badgeId: string | null, paintId: string | null }> };

export type AccountCosmeticsQueryVariables = Exact<{
  accountIds: Array<string> | string;
}>;


export type AccountCosmeticsQuery = { accountCosmetics: Array<{ accountId: string, badge: { id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } } | null, paint: { id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null } | null }> };

export type AccountProfileQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountProfileQuery = { accountProfile: { createdAt: string, account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }>, roles: Array<{ id: string, name: string }>, activeSets: Array<{ platform: Platform, login: string, setId: string, setName: string }>, editors: Array<{ id: string, displayName: string, avatar: string | null }> } | null };

export type MyCosmeticsQueryVariables = Exact<{
  accountId?: string | null | undefined;
}>;


export type MyCosmeticsQuery = { myCosmetics: { badgeId: string | null, paintId: string | null, account: { id: string, displayName: string, avatar: string | null }, badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type SelectCosmeticsMutationVariables = Exact<{
  badgeId?: string | null | undefined;
  paintId?: string | null | undefined;
  ownerId?: string | null | undefined;
}>;


export type SelectCosmeticsMutation = { selectCosmetics: { badgeId: string | null, paintId: string | null } };

export type CosmeticImageQueryVariables = Exact<{
  id: string;
}>;


export type CosmeticImageQuery = { cosmeticImage: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null };

export type EntitlementsQueryVariables = Exact<{
  kind: CosmeticKind;
  accountId?: string | null | undefined;
  cosmeticId?: string | null | undefined;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type EntitlementsQuery = { cosmeticEntitlements: { total: number, items: Array<{ kind: CosmeticKind, cosmeticId: string, source: EntitlementSource, reason: string | null, grantedAt: string, account: { id: string, displayName: string, avatar: string | null }, grantedBy: { id: string, displayName: string } | null }> } };

export type CreateBadgeMutationVariables = Exact<{
  input: CreateBadgeInput;
}>;


export type CreateBadgeMutation = { createBadge: { id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } } };

export type UpdateBadgeMutationVariables = Exact<{
  id: string;
  input: UpdateBadgeInput;
}>;


export type UpdateBadgeMutation = { updateBadge: { id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } } };

export type RemakeCosmeticImageMutationVariables = Exact<{
  id: string;
}>;


export type RemakeCosmeticImageMutation = { remakeCosmeticImage: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } };

export type DeleteBadgeMutationVariables = Exact<{
  id: string;
}>;


export type DeleteBadgeMutation = { deleteBadge: boolean };

export type DeletePaintMutationVariables = Exact<{
  id: string;
}>;


export type DeletePaintMutation = { deletePaint: boolean };

export type GrantCosmeticMutationVariables = Exact<{
  input: GrantCosmeticInput;
}>;


export type GrantCosmeticMutation = { grantCosmetic: { cosmeticId: string } };

export type RevokeCosmeticMutationVariables = Exact<{
  accountId: string;
  kind: CosmeticKind;
  cosmeticId: string;
  reason?: string | null | undefined;
}>;


export type RevokeCosmeticMutation = { revokeCosmetic: boolean };

export type AccountEditorFieldsFragment = { permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { id: string, displayName: string, avatar: string | null }, grantedBy: { id: string, displayName: string } | null };

export type EditorsQueryVariables = Exact<{ [key: string]: never; }>;


export type EditorsQuery = { accountEditors: Array<{ permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { id: string, displayName: string, avatar: string | null }, grantedBy: { id: string, displayName: string } | null }>, editorOf: Array<{ permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { id: string, displayName: string, avatar: string | null } }>, mySets: Array<{ id: string, name: string, kind: SetKind, owner: { id: string } | null }> };

export type AddAccountEditorMutationVariables = Exact<{
  input: AddAccountEditorInput;
}>;


export type AddAccountEditorMutation = { addAccountEditor: { permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { id: string, displayName: string, avatar: string | null }, grantedBy: { id: string, displayName: string } | null } };

export type UpdateAccountEditorMutationVariables = Exact<{
  ownerId?: string | null | undefined;
  accountId: string;
  permissions: Array<EditorPermission> | EditorPermission;
  setIds?: Array<string> | string | null | undefined;
}>;


export type UpdateAccountEditorMutation = { updateAccountEditor: { permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, grantedAt: string, account: { id: string, displayName: string, avatar: string | null }, grantedBy: { id: string, displayName: string } | null } };

export type RemoveAccountEditorMutationVariables = Exact<{
  ownerId?: string | null | undefined;
  accountId: string;
}>;


export type RemoveAccountEditorMutation = { removeAccountEditor: boolean };

export type ReportEmoteMutationVariables = Exact<{
  id: string;
  reason: string;
}>;


export type ReportEmoteMutation = { reportEmote: { id: string } };

export type MySetNamesQueryVariables = Exact<{ [key: string]: never; }>;


export type MySetNamesQuery = { mySets: Array<{ id: string, name: string, kind: SetKind, emotes: Array<{ emote: { id: string } }> }> };

export type SearchByOwnerQueryVariables = Exact<{
  owner: string;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
  sort?: EmoteSort | null | undefined;
}>;


export type SearchByOwnerQuery = { searchEmotes: { total: number, page: number, perPage: number, items: Array<{ id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type AdminAccountQueryVariables = Exact<{
  accountId: string;
}>;


export type AdminAccountQuery = { adminAccount: { id: string, displayName: string, avatar: string | null, createdAt: string, uploadsBlocked: boolean, ticketsBlocked: boolean, blocked: boolean, uploadsBlockedUntil: string | null, ticketsBlockedUntil: string | null, blockedUntil: string | null, identities: Array<{ platform: Platform, platformUserId: string, login: string, displayName: string, avatar: string | null }> } };

export type TransferEmoteMutationVariables = Exact<{
  id: string;
  toAccountId: string;
}>;


export type TransferEmoteMutation = { transferEmote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } };

export type AccountEmotesQueryVariables = Exact<{
  accountId: string;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type AccountEmotesQuery = { accountEmotes: { total: number, items: Array<{ id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type AccountReportsQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountReportsQuery = { accountReports: { about: Array<{ id: string, reason: string, reporterId: string | null, state: ReportState, resolution: string | null, createdAt: string, resolvedAt: string | null, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }>, by: Array<{ id: string, reason: string, reporterId: string | null, state: ReportState, resolution: string | null, createdAt: string, resolvedAt: string | null, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }> } };

export type PublicAccountQueryVariables = Exact<{
  id: string;
}>;


export type PublicAccountQuery = { account: { id: string, displayName: string, avatar: string | null } | null };

export type EmoteChannelsQueryVariables = Exact<{
  id: string;
  limit?: number | null | undefined;
  offset?: number | null | undefined;
}>;


export type EmoteChannelsQuery = { emoteChannels: Array<{ platform: Platform, login: string, account: { id: string, displayName: string, avatar: string | null } }> };

export type EmoteFieldsFragment = { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> };

export type SearchQueryVariables = Exact<{
  query: string;
  animated?: boolean | null | undefined;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
  sort?: EmoteSort | null | undefined;
  flags?: Array<EmoteFlag> | EmoteFlag | null | undefined;
  tags?: Array<string> | string | null | undefined;
  exact?: boolean | null | undefined;
  window?: TrendingWindow | null | undefined;
}>;


export type SearchQuery = { searchEmotes: { total: number, page: number, perPage: number, items: Array<{ id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type FindPeopleQueryVariables = Exact<{
  query: string;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type FindPeopleQuery = { searchAccounts: { total: number, page: number, perPage: number, items: Array<{ account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> }> } };

export type EmoteQueryVariables = Exact<{
  id: string;
}>;


export type EmoteQuery = { emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null };

export type EmoteTrendingQueryVariables = Exact<{
  id: string;
  window?: TrendingWindow | null | undefined;
}>;


export type EmoteTrendingQuery = { emoteTrending: { rank: number, channels: number, window: TrendingWindow } | null };

export type MyChannelsQueryVariables = Exact<{ [key: string]: never; }>;


export type MyChannelsQuery = { myChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, allowPersonalEmotes: boolean, account: { id: string, displayName: string } }>, mySets: Array<{ id: string, name: string, kind: SetKind, owner: { id: string } | null }>, editorChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, allowPersonalEmotes: boolean, account: { id: string, displayName: string } }>, editorSets: Array<{ id: string, name: string, kind: SetKind, owner: { id: string } | null }>, editorOf: Array<{ permissions: Array<EditorPermission>, account: { id: string } }> };

export type UpdateChannelMutationVariables = Exact<{
  platform: Platform;
  id: string;
  input: UpdateChannelSettingsInput;
}>;


export type UpdateChannelMutation = { updateChannelSettings: { activeSetId: string | null, allowPersonalEmotes: boolean } };

export type MyEmotesQueryVariables = Exact<{
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type MyEmotesQuery = { myEmotes: { total: number, items: Array<{ id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type UpdateEmoteMutationVariables = Exact<{
  id: string;
  input: UpdateEmoteInput;
}>;


export type UpdateEmoteMutation = { updateEmote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } };

export type DeleteEmoteMutationVariables = Exact<{
  id: string;
}>;


export type DeleteEmoteMutation = { deleteEmote: boolean };

export type PersonalListsQueryVariables = Exact<{ [key: string]: never; }>;


export type PersonalListsQuery = { favouriteEmoteIds: Array<string>, hiddenEmotes: Array<{ emoteId: string }>, hiddenCreators: Array<{ accountId: string }> };

export type HiddenListsQueryVariables = Exact<{ [key: string]: never; }>;


export type HiddenListsQuery = { hiddenEmotes: Array<{ emoteId: string, hiddenAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }>, hiddenCreators: Array<{ accountId: string, hiddenAt: string, account: { id: string, displayName: string, avatar: string | null } | null }> };

export type MyFavouritesQueryVariables = Exact<{
  query?: string | null | undefined;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type MyFavouritesQuery = { myFavourites: { total: number, items: Array<{ id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type HideEmoteMutationVariables = Exact<{
  id: string;
}>;


export type HideEmoteMutation = { hideEmote: boolean };

export type UnhideEmoteMutationVariables = Exact<{
  id: string;
}>;


export type UnhideEmoteMutation = { unhideEmote: boolean };

export type HideCreatorMutationVariables = Exact<{
  accountId: string;
}>;


export type HideCreatorMutation = { hideCreator: boolean };

export type UnhideCreatorMutationVariables = Exact<{
  accountId: string;
}>;


export type UnhideCreatorMutation = { unhideCreator: boolean };

export type FavouriteEmoteMutationVariables = Exact<{
  id: string;
}>;


export type FavouriteEmoteMutation = { favouriteEmote: boolean };

export type UnfavouriteEmoteMutationVariables = Exact<{
  id: string;
}>;


export type UnfavouriteEmoteMutation = { unfavouriteEmote: boolean };

export type AccountRolesQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountRolesQuery = { accountRoles: Array<{ id: string, name: string, position: number }> };

export type AccountEditorsQueryVariables = Exact<{
  accountId?: string | null | undefined;
}>;


export type AccountEditorsQuery = { accountEditors: Array<{ permissions: Array<EditorPermission>, account: { id: string, displayName: string, avatar: string | null } }> };

export type EditorOfQueryVariables = Exact<{ [key: string]: never; }>;


export type EditorOfQuery = { editorOf: Array<{ permissions: Array<EditorPermission>, allSets: boolean, account: { id: string, displayName: string, avatar: string | null } }> };

export type AccountSetsQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountSetsQuery = { accountSets: Array<{ id: string, name: string, kind: SetKind, capacity: number, slotsUsed: number, active: boolean, preview: Array<{ emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ProfileMineQueryVariables = Exact<{ [key: string]: never; }>;


export type ProfileMineQuery = { myChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null }>, mySets: Array<{ id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner: { id: string, displayName: string, avatar: string | null } | null, emotes: Array<{ name: string, zeroWidth: boolean, addedAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ProfileEditedQueryVariables = Exact<{ [key: string]: never; }>;


export type ProfileEditedQuery = { editorChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, account: { id: string } }>, editorSets: Array<{ id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner: { id: string, displayName: string, avatar: string | null } | null, emotes: Array<{ name: string, zeroWidth: boolean, addedAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }> };

export type ReportFieldsFragment = { id: string, reason: string, reporterId: string | null, state: ReportState, resolution: string | null, createdAt: string, resolvedAt: string | null, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null };

export type ReportQueueQueryVariables = Exact<{
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type ReportQueueQuery = { reportQueue: { total: number, items: Array<{ id: string, reason: string, reporterId: string | null, state: ReportState, createdAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } | null }> } };

export type ResolveReportMutationVariables = Exact<{
  id: string;
  action: ReportAction;
  note?: string | null | undefined;
}>;


export type ResolveReportMutation = { resolveReport: { id: string, state: ReportState } };

export type ReviewEmotesMutationVariables = Exact<{
  ids: Array<string> | string;
  verdict: ReviewVerdict;
  visibility?: EmoteVisibility | null | undefined;
  reason?: string | null | undefined;
}>;


export type ReviewEmotesMutation = { reviewEmotes: { done: Array<string>, failed: Array<{ id: string, code: string, message: string }> } };

export type RequeueEmotesMutationVariables = Exact<{
  ids: Array<string> | string;
}>;


export type RequeueEmotesMutation = { requeueEmotes: { done: Array<string>, failed: Array<{ id: string, code: string, message: string }> } };

export type SetEmoteFlagsMutationVariables = Exact<{
  id: string;
  input: EmoteFlagsInput;
}>;


export type SetEmoteFlagsMutation = { setEmoteFlags: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } };

export type DismissReportsMutationVariables = Exact<{
  ids: Array<string> | string;
  note?: string | null | undefined;
}>;


export type DismissReportsMutation = { dismissReports: { done: Array<string>, failed: Array<{ id: string, code: string, message: string }> } };

export type EmoteDuplicatesQueryVariables = Exact<{
  emoteId: string;
}>;


export type EmoteDuplicatesQuery = { emoteDuplicates: Array<{ sameFile: boolean, similarity: number, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> };

export type ReviewReasonsQueryVariables = Exact<{ [key: string]: never; }>;


export type ReviewReasonsQuery = { reviewReasons: Array<string> };

export type SetReviewReasonsMutationVariables = Exact<{
  reasons: Array<string> | string;
}>;


export type SetReviewReasonsMutation = { setReviewReasons: Array<string> };

export type AccountsByLoginQueryVariables = Exact<{
  logins: Array<string> | string;
}>;


export type AccountsByLoginQuery = { accountsByLogin: Array<{ login: string, problem: string | null, account: { id: string, displayName: string, avatar: string | null } | null }> };

export type EmotesWithSameFileQueryVariables = Exact<{
  sha256: string;
}>;


export type EmotesWithSameFileQuery = { emotesWithSameFile: Array<{ id: string, name: string, owner: { id: string, displayName: string } | null }> };

export type AdminCountsQueryVariables = Exact<{
  review: boolean;
  reports: boolean;
}>;


export type AdminCountsQuery = { reviewQueue?: { total: number }, reportQueue?: { total: number }, globalSet: { id: string } };

export type ReviewQueueQueryVariables = Exact<{
  page?: number | null | undefined;
  perPage?: number | null | undefined;
  filter?: ReviewQueueFilter | null | undefined;
}>;


export type ReviewQueueQuery = { reviewQueue: { total: number, counts: { public: number, limited: number }, items: Array<{ id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }> } };

export type ApproveMutationVariables = Exact<{
  id: string;
  visibility?: EmoteVisibility | null | undefined;
}>;


export type ApproveMutation = { approveEmote: { id: string, state: EmoteState, visibility: EmoteVisibility } };

export type EmoteHistoryQueryVariables = Exact<{
  emoteId: string;
}>;


export type EmoteHistoryQuery = { emoteHistory: Array<{ id: string, actorId: string | null, action: string, before: string | null, after: string | null, createdAt: string }> };

export type EmoteNotesQueryVariables = Exact<{
  emoteId: string;
}>;


export type EmoteNotesQuery = { emoteNotes: Array<{ id: string, authorId: string | null, body: string, createdAt: string }> };

export type AddEmoteNoteMutationVariables = Exact<{
  emoteId: string;
  body: string;
}>;


export type AddEmoteNoteMutation = { addEmoteNote: { id: string, authorId: string | null, body: string, createdAt: string } };

export type DeleteEmoteNoteMutationVariables = Exact<{
  id: string;
}>;


export type DeleteEmoteNoteMutation = { deleteEmoteNote: boolean };

export type RejectMutationVariables = Exact<{
  id: string;
  reason: string;
}>;


export type RejectMutation = { rejectEmote: { id: string, state: EmoteState } };

export type RoleFieldsFragment = { id: string, name: string, description: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, builtIn: BuiltInRole | null, editable: boolean, assignable: boolean };

export type RolesQueryVariables = Exact<{ [key: string]: never; }>;


export type RolesQuery = { permissions: Array<string>, roles: Array<{ id: string, name: string, description: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, builtIn: BuiltInRole | null, editable: boolean, assignable: boolean }> };

export type RoleLimitFieldsFragment = { key: string, group: LimitGroup, default: number, windowSeconds: number | null, value: number | null };

export type LimitDefaultsQueryVariables = Exact<{ [key: string]: never; }>;


export type LimitDefaultsQuery = { limitDefaults: Array<{ key: string, group: LimitGroup, default: number, windowSeconds: number | null, value: number | null }> };

export type RoleLimitsQueryVariables = Exact<{
  id: string;
}>;


export type RoleLimitsQuery = { roleLimits: Array<{ key: string, group: LimitGroup, default: number, windowSeconds: number | null, value: number | null }> };

export type SetRoleLimitsMutationVariables = Exact<{
  id: string;
  limits: Array<RoleLimitInput> | RoleLimitInput;
}>;


export type SetRoleLimitsMutation = { setRoleLimits: Array<{ key: string, group: LimitGroup, default: number, windowSeconds: number | null, value: number | null }> };

export type UploadQuotaQueryVariables = Exact<{
  ownerId?: string | null | undefined;
}>;


export type UploadQuotaQuery = { uploadQuota: { accountId: string, pending: number, pendingMax: number | null, uploadsThisHour: number, uploadsPerHour: number, maxBytes: number, maxSide: number, maxFrames: number } };

export type MediaLimitFieldsFragment = { maxBytes: number, maxSide: number, maxFrames: number, maxTotalPixels: number, maxDurationMs: number | null, minSide: number | null, maxAspect: number | null, maxServedSide: number };

export type PublicLimitsQueryVariables = Exact<{ [key: string]: never; }>;


export type PublicLimitsQuery = { limits: { emote: { maxBytes: number, maxSide: number, maxFrames: number, maxTotalPixels: number, maxDurationMs: number | null, minSide: number | null, maxAspect: number | null, maxServedSide: number }, badge: { maxBytes: number, maxSide: number, maxFrames: number, maxTotalPixels: number, maxDurationMs: number | null, minSide: number | null, maxAspect: number | null, maxServedSide: number }, paint: { maxBytes: number, maxSide: number, maxFrames: number, maxTotalPixels: number, maxDurationMs: number | null, minSide: number | null, maxAspect: number | null, maxServedSide: number }, names: { emote: { min: number, max: number }, set: { min: number, max: number }, badge: { min: number, max: number }, paint: { min: number, max: number }, tooltip: { min: number, max: number } }, tags: { max: number, maxLen: number }, rates: Array<{ key: string, count: number, windowSeconds: number }>, account: Array<{ key: string, default: number, effective: number, windowSeconds: number | null }> } };

export type CreateRoleMutationVariables = Exact<{
  input: CreateRoleInput;
}>;


export type CreateRoleMutation = { createRole: { id: string, name: string, description: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, builtIn: BuiltInRole | null, editable: boolean, assignable: boolean } };

export type UpdateRoleMutationVariables = Exact<{
  id: string;
  input: UpdateRoleInput;
}>;


export type UpdateRoleMutation = { updateRole: { id: string, name: string, description: string | null, position: number, permissions: Array<string>, denies: Array<string>, public: boolean, invisible: boolean, builtIn: BuiltInRole | null, editable: boolean, assignable: boolean } };

export type DeleteRoleMutationVariables = Exact<{
  id: string;
}>;


export type DeleteRoleMutation = { deleteRole: boolean };

export type QuickSearchQueryVariables = Exact<{
  query: string;
}>;


export type QuickSearchQuery = { quickSearch: { emotes: Array<{ id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> }>, users: Array<{ account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> }> } };

export type EditableChannelsQueryVariables = Exact<{ [key: string]: never; }>;


export type EditableChannelsQuery = { myEditableChannels: Array<{ role: ChannelRole, channel: { platform: Platform, login: string, account: { id: string, displayName: string, avatar: string | null } } }> };

export type SetFieldsFragment = { id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner: { id: string, displayName: string, avatar: string | null } | null, emotes: Array<{ name: string, zeroWidth: boolean, addedAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> };

export type MySetsQueryVariables = Exact<{ [key: string]: never; }>;


export type MySetsQuery = { mySets: Array<{ id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner: { id: string, displayName: string, avatar: string | null } | null, emotes: Array<{ name: string, zeroWidth: boolean, addedAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }>, editorSets: Array<{ id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, owner: { id: string, displayName: string, avatar: string | null } | null, emotes: Array<{ name: string, zeroWidth: boolean, addedAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> }>, editorOf: Array<{ permissions: Array<EditorPermission>, account: { id: string, displayName: string } }> };

export type EditSetQueryVariables = Exact<{
  id: string;
}>;


export type EditSetQuery = { set: { id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, slotsUsed: number, owner: { id: string, displayName: string, avatar: string | null } | null } | null, setEditors: Array<{ id: string, displayName: string, avatar: string | null }>, editorOf: Array<{ permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, account: { id: string } }>, myChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, account: { id: string } }>, editorChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, account: { id: string } }> };

export type SetEmotesQueryVariables = Exact<{
  id: string;
  query?: string | null | undefined;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type SetEmotesQuery = { setEmotes: { total: number, items: Array<{ name: string, zeroWidth: boolean, addedAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } };

export type SetMembersQueryVariables = Exact<{
  id: string;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type SetMembersQuery = { setEmotes: { total: number, items: Array<{ name: string, emote: { id: string } }> } };

export type CreateSetMutationVariables = Exact<{
  input: CreateSetInput;
}>;


export type CreateSetMutation = { createSet: { id: string } };

export type UpdateSetMutationVariables = Exact<{
  id: string;
  input: UpdateSetInput;
}>;


export type UpdateSetMutation = { updateSet: { id: string, name: string, visibility: SetVisibility, capacity: number } };

export type DeleteSetMutationVariables = Exact<{
  id: string;
}>;


export type DeleteSetMutation = { deleteSet: boolean };

export type SetCapacityRangeQueryVariables = Exact<{
  id: string;
}>;


export type SetCapacityRangeQuery = { setCapacityRange: { min: number, max: number, ownerMax: number, used: number, staff: boolean } | null };

export type SetMenuQueryVariables = Exact<{
  id: string;
}>;


export type SetMenuQuery = { setCapacityRange: { min: number, max: number, ownerMax: number, used: number, staff: boolean } | null, editorOf: Array<{ permissions: Array<EditorPermission>, allSets: boolean, setIds: Array<string>, account: { id: string } }>, myChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, account: { id: string } }>, editorChannels: Array<{ platform: Platform, platformChannelId: string, login: string, activeSetId: string | null, account: { id: string } }> };

export type AddSetEmoteMutationVariables = Exact<{
  setId: string;
  emoteId: string;
  input?: AddSetEmoteInput | null | undefined;
}>;


export type AddSetEmoteMutation = { addSetEmote: { name: string } };

export type UpdateSetEmoteMutationVariables = Exact<{
  setId: string;
  emoteId: string;
  input: UpdateSetEmoteInput;
}>;


export type UpdateSetEmoteMutation = { updateSetEmote: { name: string, zeroWidth: boolean } };

export type RemoveSetEmoteMutationVariables = Exact<{
  setId: string;
  emoteId: string;
}>;


export type RemoveSetEmoteMutation = { removeSetEmote: boolean };

export type RemoveSetEditorMutationVariables = Exact<{
  setId: string;
  accountId: string;
}>;


export type RemoveSetEditorMutation = { removeSetEditor: boolean };

export type PublicSetQueryVariables = Exact<{
  id: string;
}>;


export type PublicSetQuery = { set: { id: string, name: string, kind: SetKind, visibility: SetVisibility, capacity: number, slotsUsed: number, createdAt: string, owner: { id: string, displayName: string, avatar: string | null } | null, emotes: Array<{ name: string, zeroWidth: boolean, addedAt: string, emote: { id: string, name: string, state: EmoteState, processing: EmoteProcessing, animated: boolean, tags: Array<string>, host: string, createdAt: string, channelCount: number, visibility: EmoteVisibility, flags: { zeroWidth: boolean, private: boolean, wide: boolean }, owner: { id: string, displayName: string, avatar: string | null } | null, files: Array<{ name: string, format: ImageFormat, scale: number, width: number, height: number, size: number, isStatic: boolean }> } }> } | null };

export type SetEditorsQueryVariables = Exact<{
  id: string;
}>;


export type SetEditorsQuery = { setEditors: Array<{ id: string, displayName: string, avatar: string | null }> };

export type CloneSetMutationVariables = Exact<{
  id: string;
  name?: string | null | undefined;
  visibility?: SetVisibility | null | undefined;
}>;


export type CloneSetMutation = { cloneSet: { copied: number, skipped: number, set: { id: string, name: string } } };

export type ShopItemFieldsFragment = { id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId: string | null, activeFrom: string | null, activeTo: string | null, stockLimit: number | null, stockLeft: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> };

export type ShopItemsQueryVariables = Exact<{
  all?: boolean | null | undefined;
}>;


export type ShopItemsQuery = { shopItems: Array<{ id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId: string | null, activeFrom: string | null, activeTo: string | null, stockLimit: number | null, stockLeft: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> }> };

export type ShopItemQueryVariables = Exact<{
  id: string;
}>;


export type ShopItemQuery = { shopItem: { id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId: string | null, activeFrom: string | null, activeTo: string | null, stockLimit: number | null, stockLeft: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> } | null };

export type OrderFieldsFragment = { id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents: number | null, paidCurrency: string | null, refundReason: string | null, createdAt: string, paidAt: string | null, refundedAt: string | null, failedAt: string | null, buyer: { id: string, displayName: string, avatar: string | null } | null, recipient: { id: string, displayName: string, avatar: string | null } | null };

export type StartCheckoutMutationVariables = Exact<{
  itemId: string;
  recipient?: RecipientInput | null | undefined;
  idempotencyKey?: string | null | undefined;
}>;


export type StartCheckoutMutation = { startCheckout: { transactionId: string, checkoutUrl: string | null, clientToken: string | null, environment: string, order: { id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents: number | null, paidCurrency: string | null, refundReason: string | null, createdAt: string, paidAt: string | null, refundedAt: string | null, failedAt: string | null, buyer: { id: string, displayName: string, avatar: string | null } | null, recipient: { id: string, displayName: string, avatar: string | null } | null } } };

export type OrderQueryVariables = Exact<{
  id: string;
}>;


export type OrderQuery = { order: { id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents: number | null, paidCurrency: string | null, refundReason: string | null, createdAt: string, paidAt: string | null, refundedAt: string | null, failedAt: string | null, buyer: { id: string, displayName: string, avatar: string | null } | null, recipient: { id: string, displayName: string, avatar: string | null } | null } | null };

export type MyOrdersQueryVariables = Exact<{
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type MyOrdersQuery = { myOrders: { total: number, items: Array<{ id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents: number | null, paidCurrency: string | null, refundReason: string | null, createdAt: string, paidAt: string | null, refundedAt: string | null, failedAt: string | null, buyer: { id: string, displayName: string, avatar: string | null } | null, recipient: { id: string, displayName: string, avatar: string | null } | null }> } };

export type RedeemCodeMutationVariables = Exact<{
  code: string;
  idempotencyKey?: string | null | undefined;
}>;


export type RedeemCodeMutation = { redeemCode: { name: string, badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type CreateShopItemMutationVariables = Exact<{
  input: ShopItemInput;
}>;


export type CreateShopItemMutation = { createShopItem: { id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId: string | null, activeFrom: string | null, activeTo: string | null, stockLimit: number | null, stockLeft: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type UpdateShopItemMutationVariables = Exact<{
  id: string;
  input: ShopItemInput;
}>;


export type UpdateShopItemMutation = { updateShopItem: { id: string, name: string, description: string, priceCents: number, currency: string, paddlePriceId: string | null, activeFrom: string | null, activeTo: string | null, stockLimit: number | null, stockLeft: number | null, visible: boolean, sortOrder: number, available: boolean, owned: boolean, deleted: boolean, badges: Array<{ id: string, name: string, tooltip: string, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } }>, paints: Array<{ id: string, name: string, kind: PaintKind, angle: number, repeat: boolean, shape: PaintShape, color: string | null, stops: Array<{ at: number, color: string }>, shadows: Array<{ x: number, y: number, radius: number, color: string }>, image: { id: string, kind: CosmeticKind, host: string, animated: boolean, ready: boolean, width: number, height: number, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> } };

export type DeleteShopItemMutationVariables = Exact<{
  id: string;
}>;


export type DeleteShopItemMutation = { deleteShopItem: boolean };

export type AdminOrdersQueryVariables = Exact<{
  status?: OrderStatus | null | undefined;
  accountId?: string | null | undefined;
  itemId?: string | null | undefined;
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type AdminOrdersQuery = { orders: { total: number, items: Array<{ providerTransactionId: string | null, refundRejectedAt: string | null, id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents: number | null, paidCurrency: string | null, refundReason: string | null, createdAt: string, paidAt: string | null, refundedAt: string | null, failedAt: string | null, buyer: { id: string, displayName: string, avatar: string | null } | null, recipient: { id: string, displayName: string, avatar: string | null } | null }> } };

export type RefundOrderMutationVariables = Exact<{
  id: string;
  reason: string;
}>;


export type RefundOrderMutation = { refundOrder: { providerTransactionId: string | null, refundRejectedAt: string | null, id: string, gift: boolean, itemId: string, itemName: string, status: OrderStatus, amountCents: number, currency: string, paidTotalCents: number | null, paidCurrency: string | null, refundReason: string | null, createdAt: string, paidAt: string | null, refundedAt: string | null, failedAt: string | null, buyer: { id: string, displayName: string, avatar: string | null } | null, recipient: { id: string, displayName: string, avatar: string | null } | null } };

export type CodeBatchFieldsFragment = { id: string, name: string, prefix: string, itemId: string | null, itemName: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt: string | null, disabledAt: string | null, createdAt: string, createdBy: { id: string, displayName: string } | null };

export type CodeBatchesQueryVariables = Exact<{
  page?: number | null | undefined;
  perPage?: number | null | undefined;
}>;


export type CodeBatchesQuery = { codeBatches: { total: number, items: Array<{ id: string, name: string, prefix: string, itemId: string | null, itemName: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt: string | null, disabledAt: string | null, createdAt: string, createdBy: { id: string, displayName: string } | null }> } };

export type CreateCodeBatchMutationVariables = Exact<{
  input: CodeBatchInput;
}>;


export type CreateCodeBatchMutation = { createCodeBatch: { codes: Array<string>, batch: { id: string, name: string, prefix: string, itemId: string | null, itemName: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt: string | null, disabledAt: string | null, createdAt: string, createdBy: { id: string, displayName: string } | null } } };

export type DisableCodeBatchMutationVariables = Exact<{
  id: string;
}>;


export type DisableCodeBatchMutation = { disableCodeBatch: { id: string, name: string, prefix: string, itemId: string | null, itemName: string | null, badgeIds: Array<string>, paintIds: Array<string>, codeCount: number, usesPerCode: number, redemptions: number, expiresAt: string | null, disabledAt: string | null, createdAt: string, createdBy: { id: string, displayName: string } | null } };

export type ShopCountsQueryVariables = Exact<{
  items: boolean;
  orders: boolean;
  codes: boolean;
}>;


export type ShopCountsQuery = { shopItems?: Array<{ id: string }>, orders?: { total: number }, codeBatches?: { total: number } };

export type AnnouncementFieldsFragment = { id: string, kind: AnnouncementKind, text: string, linkUrl: string | null, linkLabel: string | null, badgeId: string | null, startsAt: string | null, endsAt: string | null, endedAt: string | null, createdAt: string, updatedAt: string, live: boolean, emote: { id: string, name: string, host: string, animated: boolean, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null };

export type ActiveAnnouncementQueryVariables = Exact<{ [key: string]: never; }>;


export type ActiveAnnouncementQuery = { activeAnnouncement: { id: string, kind: AnnouncementKind, text: string, linkUrl: string | null, linkLabel: string | null, badgeId: string | null, startsAt: string | null, endsAt: string | null, endedAt: string | null, createdAt: string, updatedAt: string, live: boolean, emote: { id: string, name: string, host: string, animated: boolean, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null } | null };

export type AnnouncementsQueryVariables = Exact<{ [key: string]: never; }>;


export type AnnouncementsQuery = { announcements: Array<{ id: string, kind: AnnouncementKind, text: string, linkUrl: string | null, linkLabel: string | null, badgeId: string | null, startsAt: string | null, endsAt: string | null, endedAt: string | null, createdAt: string, updatedAt: string, live: boolean, emote: { id: string, name: string, host: string, animated: boolean, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null }> };

export type CreateAnnouncementMutationVariables = Exact<{
  input: AnnouncementInput;
}>;


export type CreateAnnouncementMutation = { createAnnouncement: { id: string, kind: AnnouncementKind, text: string, linkUrl: string | null, linkLabel: string | null, badgeId: string | null, startsAt: string | null, endsAt: string | null, endedAt: string | null, createdAt: string, updatedAt: string, live: boolean, emote: { id: string, name: string, host: string, animated: boolean, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null } };

export type UpdateAnnouncementMutationVariables = Exact<{
  id: string;
  input: AnnouncementInput;
}>;


export type UpdateAnnouncementMutation = { updateAnnouncement: { id: string, kind: AnnouncementKind, text: string, linkUrl: string | null, linkLabel: string | null, badgeId: string | null, startsAt: string | null, endsAt: string | null, endedAt: string | null, createdAt: string, updatedAt: string, live: boolean, emote: { id: string, name: string, host: string, animated: boolean, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null } };

export type EndAnnouncementMutationVariables = Exact<{
  id: string;
}>;


export type EndAnnouncementMutation = { endAnnouncement: { id: string, kind: AnnouncementKind, text: string, linkUrl: string | null, linkLabel: string | null, badgeId: string | null, startsAt: string | null, endsAt: string | null, endedAt: string | null, createdAt: string, updatedAt: string, live: boolean, emote: { id: string, name: string, host: string, animated: boolean, files: Array<{ name: string, width: number, height: number, isStatic: boolean }> } | null } };

export type NotificationPrefsQueryVariables = Exact<{ [key: string]: never; }>;


export type NotificationPrefsQuery = { myNotificationPrefs: Array<{ kind: string, notify: boolean, toast: boolean, required: boolean }> };

export type SetNotificationPrefsMutationVariables = Exact<{
  prefs: Array<NotificationPrefInput> | NotificationPrefInput;
}>;


export type SetNotificationPrefsMutation = { setNotificationPrefs: Array<{ kind: string, notify: boolean, toast: boolean, required: boolean }> };

export type StaffNotesQueryVariables = Exact<{
  accountId: string;
}>;


export type StaffNotesQuery = { staffNotes: Array<{ id: string, authorId: string | null, body: string, createdAt: string }> };

export type AddStaffNoteMutationVariables = Exact<{
  accountId: string;
  body: string;
}>;


export type AddStaffNoteMutation = { addStaffNote: { id: string, authorId: string | null, body: string, createdAt: string } };

export type DeleteStaffNoteMutationVariables = Exact<{
  id: string;
}>;


export type DeleteStaffNoteMutation = { deleteStaffNote: boolean };

export type AccountSessionsQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountSessionsQuery = { accountSessions: Array<{ id: string, kind: SessionKind, device: string, createdAt: string, lastSeenAt: string, expiresAt: string }> };

export type AccountSubscriptionsQueryVariables = Exact<{
  accountId: string;
}>;


export type AccountSubscriptionsQuery = { accountSubscriptions: Array<{ id: string, plan: string, state: SubscriptionState, startsAt: string, endsAt: string | null, source: string }> };

export type GrantFieldsFragment = { kind: GrantKind, itemId: string, itemName: string, accountId: string, source: GrantSource, reason: string | null, grantedBy: string | null, grantedAt: string | null, orderId: string | null, orderName: string | null, orderStatus: string | null, codeBatchId: string | null, codeBatchName: string | null, revokedAt: string | null, revokedBy: string | null, revokeReason: string | null, active: boolean };

export type EntitlementGraphQueryVariables = Exact<{
  accountId: string;
}>;


export type EntitlementGraphQuery = { entitlementGraph: Array<{ kind: GrantKind, itemId: string, itemName: string, accountId: string, source: GrantSource, reason: string | null, grantedBy: string | null, grantedAt: string | null, orderId: string | null, orderName: string | null, orderStatus: string | null, codeBatchId: string | null, codeBatchName: string | null, revokedAt: string | null, revokedBy: string | null, revokeReason: string | null, active: boolean }> };

export type EntitlementHoldersQueryVariables = Exact<{
  kind: GrantKind;
  id: string;
}>;


export type EntitlementHoldersQuery = { entitlementHolders: Array<{ kind: GrantKind, itemId: string, itemName: string, accountId: string, source: GrantSource, reason: string | null, grantedBy: string | null, grantedAt: string | null, orderId: string | null, orderName: string | null, orderStatus: string | null, codeBatchId: string | null, codeBatchName: string | null, revokedAt: string | null, revokedBy: string | null, revokeReason: string | null, active: boolean }> };

export type PersonCountFieldsFragment = { count: number, account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> };

export type ActivityTopActorsQueryVariables = Exact<{
  accountId?: string | null | undefined;
  emoteId?: string | null | undefined;
  filter?: ActivityFilter | null | undefined;
  limit?: number | null | undefined;
}>;


export type ActivityTopActorsQuery = { activityTopActors: Array<{ count: number, account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> }> };

export type AuditTopPeopleQueryVariables = Exact<{
  by?: AuditPersonRole | null | undefined;
  targetKind?: string | null | undefined;
  targetId?: string | null | undefined;
  actorId?: string | null | undefined;
  action?: string | null | undefined;
  since?: string | null | undefined;
  until?: string | null | undefined;
  limit?: number | null | undefined;
}>;


export type AuditTopPeopleQuery = { auditTopPeople: Array<{ count: number, account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> }> };

export type OrderTopBuyersQueryVariables = Exact<{
  status?: OrderStatus | null | undefined;
  itemId?: string | null | undefined;
  limit?: number | null | undefined;
}>;


export type OrderTopBuyersQuery = { orderTopBuyers: Array<{ count: number, account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> }> };

export type ReviewTopUploadersQueryVariables = Exact<{
  filter?: ReviewQueueFilter | null | undefined;
  limit?: number | null | undefined;
}>;


export type ReviewTopUploadersQuery = { reviewTopUploaders: Array<{ count: number, account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> }> };

export type EntitlementTopHoldersQueryVariables = Exact<{
  limit?: number | null | undefined;
}>;


export type EntitlementTopHoldersQuery = { entitlementTopHolders: Array<{ count: number, account: { id: string, displayName: string, avatar: string | null }, identities: Array<{ platform: Platform, login: string, displayName: string }> }> };

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
export const AccountEditorFieldsFragmentDoc = new TypedDocumentString(`
    fragment AccountEditorFields on AccountEditor {
  account {
    id
    displayName
    avatar
  }
  permissions
  allSets
  setIds
  grantedBy {
    id
    displayName
  }
  grantedAt
}
    `, {"fragmentName":"AccountEditorFields"}) as unknown as TypedDocumentString<AccountEditorFieldsFragment, unknown>;
export const EmoteFieldsFragmentDoc = new TypedDocumentString(`
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  denies
  public
  invisible
  builtIn
  editable
  assignable
}
    `, {"fragmentName":"RoleFields"}) as unknown as TypedDocumentString<RoleFieldsFragment, unknown>;
export const RoleLimitFieldsFragmentDoc = new TypedDocumentString(`
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  windowSeconds
  value
}
    `, {"fragmentName":"RoleLimitFields"}) as unknown as TypedDocumentString<RoleLimitFieldsFragment, unknown>;
export const MediaLimitFieldsFragmentDoc = new TypedDocumentString(`
    fragment MediaLimitFields on MediaLimits {
  maxBytes
  maxSide
  maxFrames
  maxTotalPixels
  maxDurationMs
  minSide
  maxAspect
  maxServedSide
}
    `, {"fragmentName":"MediaLimitFields"}) as unknown as TypedDocumentString<MediaLimitFieldsFragment, unknown>;
export const SetFieldsFragmentDoc = new TypedDocumentString(`
    fragment SetFields on EmoteSet {
  id
  name
  kind
  visibility
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
export const AnnouncementFieldsFragmentDoc = new TypedDocumentString(`
    fragment AnnouncementFields on Announcement {
  id
  kind
  text
  linkUrl
  linkLabel
  badgeId
  startsAt
  endsAt
  endedAt
  createdAt
  updatedAt
  live
  emote {
    id
    name
    host
    animated
    files {
      name
      width
      height
      isStatic
    }
  }
}
    `, {"fragmentName":"AnnouncementFields"}) as unknown as TypedDocumentString<AnnouncementFieldsFragment, unknown>;
export const GrantFieldsFragmentDoc = new TypedDocumentString(`
    fragment GrantFields on EntitlementGrant {
  kind
  itemId
  itemName
  accountId
  source
  reason
  grantedBy
  grantedAt
  orderId
  orderName
  orderStatus
  codeBatchId
  codeBatchName
  revokedAt
  revokedBy
  revokeReason
  active
}
    `, {"fragmentName":"GrantFields"}) as unknown as TypedDocumentString<GrantFieldsFragment, unknown>;
export const PersonCountFieldsFragmentDoc = new TypedDocumentString(`
    fragment PersonCountFields on PersonCount {
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
  count
}
    `, {"fragmentName":"PersonCountFields"}) as unknown as TypedDocumentString<PersonCountFieldsFragment, unknown>;
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
export const SetDisplayPlatformDocument = new TypedDocumentString(`
    mutation SetDisplayPlatform($platform: Platform!) {
  setDisplayPlatform(platform: $platform) {
    id
    displayPlatform
  }
}
    `) as unknown as TypedDocumentString<SetDisplayPlatformMutation, SetDisplayPlatformMutationVariables>;
export const SignOutEverywhereDocument = new TypedDocumentString(`
    mutation SignOutEverywhere {
  signOutEverywhere
}
    `) as unknown as TypedDocumentString<SignOutEverywhereMutation, SignOutEverywhereMutationVariables>;
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
export const MyConsoleTokenDocument = new TypedDocumentString(`
    query MyConsoleToken {
  myConsoleToken {
    id
    createdAt
    expiresAt
  }
}
    `) as unknown as TypedDocumentString<MyConsoleTokenQuery, MyConsoleTokenQueryVariables>;
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
      invisible
    }
  }
}
    `) as unknown as TypedDocumentString<MyRolesQuery, MyRolesQueryVariables>;
export const MeDocument = new TypedDocumentString(`
    query Me {
  me {
    id
    createdAt
    permissions
    displayPlatform
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
export const AccountActivityDocument = new TypedDocumentString(`
    query AccountActivity($accountId: String!, $before: String, $limit: Int, $filter: ActivityFilter) {
  accountActivityPage(
    accountId: $accountId
    before: $before
    limit: $limit
    filter: $filter
  ) {
    entries {
      id
      kind
      actor {
        id
        displayName
        avatar
      }
      editor {
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
    query EmoteActivity($emoteId: String!, $before: String, $limit: Int, $actorId: String) {
  emoteActivityPage(
    emoteId: $emoteId
    before: $before
    limit: $limit
    actorId: $actorId
  ) {
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
  denies
  public
  invisible
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<UserDetailsQuery, UserDetailsQueryVariables>;
export const AccountPermissionsDocument = new TypedDocumentString(`
    query AccountPermissions($accountId: String!) {
  accountPermissions(accountId: $accountId) {
    permission
    granted
    effect
    roleId
    roleName
    owner
  }
}
    `) as unknown as TypedDocumentString<AccountPermissionsQuery, AccountPermissionsQueryVariables>;
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
  denies
  public
  invisible
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<AllRolesQuery, AllRolesQueryVariables>;
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
    editors {
      id
      displayName
      avatar
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
export const RemakeCosmeticImageDocument = new TypedDocumentString(`
    mutation RemakeCosmeticImage($id: String!) {
  remakeCosmeticImage(id: $id) {
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
}`) as unknown as TypedDocumentString<RemakeCosmeticImageMutation, RemakeCosmeticImageMutationVariables>;
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
    allSets
    setIds
    grantedAt
  }
  mySets {
    id
    name
    kind
    owner {
      id
    }
  }
}
    fragment AccountEditorFields on AccountEditor {
  account {
    id
    displayName
    avatar
  }
  permissions
  allSets
  setIds
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
  allSets
  setIds
  grantedBy {
    id
    displayName
  }
  grantedAt
}`) as unknown as TypedDocumentString<AddAccountEditorMutation, AddAccountEditorMutationVariables>;
export const UpdateAccountEditorDocument = new TypedDocumentString(`
    mutation UpdateAccountEditor($ownerId: String, $accountId: String!, $permissions: [EditorPermission!]!, $setIds: [String!]) {
  updateAccountEditor(
    ownerId: $ownerId
    accountId: $accountId
    permissions: $permissions
    setIds: $setIds
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
  allSets
  setIds
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
export const TransferEmoteDocument = new TypedDocumentString(`
    mutation TransferEmote($id: String!, $toAccountId: String!) {
  transferEmote(id: $id, toAccountId: $toAccountId) {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
}`) as unknown as TypedDocumentString<TransferEmoteMutation, TransferEmoteMutationVariables>;
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
    query EmoteChannels($id: String!, $limit: Int, $offset: Int) {
  emoteChannels(id: $id, limit: $limit, offset: $offset) {
    platform
    login
    account {
      id
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<EmoteChannelsQuery, EmoteChannelsQueryVariables>;
export const SearchDocument = new TypedDocumentString(`
    query Search($query: String!, $animated: Boolean, $page: Int, $perPage: Int, $sort: EmoteSort, $flags: [EmoteFlag!], $tags: [String!], $exact: Boolean, $window: TrendingWindow) {
  searchEmotes(
    query: $query
    animated: $animated
    page: $page
    perPage: $perPage
    sort: $sort
    flags: $flags
    tags: $tags
    exact: $exact
    window: $window
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
export const EmoteTrendingDocument = new TypedDocumentString(`
    query EmoteTrending($id: String!, $window: TrendingWindow) {
  emoteTrending(id: $id, window: $window) {
    rank
    channels
    window
  }
}
    `) as unknown as TypedDocumentString<EmoteTrendingQuery, EmoteTrendingQueryVariables>;
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
export const PersonalListsDocument = new TypedDocumentString(`
    query PersonalLists {
  hiddenEmotes {
    emoteId
  }
  hiddenCreators {
    accountId
  }
  favouriteEmoteIds
}
    `) as unknown as TypedDocumentString<PersonalListsQuery, PersonalListsQueryVariables>;
export const HiddenListsDocument = new TypedDocumentString(`
    query HiddenLists {
  hiddenEmotes {
    emoteId
    hiddenAt
    emote {
      ...EmoteFields
    }
  }
  hiddenCreators {
    accountId
    hiddenAt
    account {
      id
      displayName
      avatar
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
}`) as unknown as TypedDocumentString<HiddenListsQuery, HiddenListsQueryVariables>;
export const MyFavouritesDocument = new TypedDocumentString(`
    query MyFavourites($query: String, $page: Int, $perPage: Int) {
  myFavourites(query: $query, page: $page, perPage: $perPage) {
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
}`) as unknown as TypedDocumentString<MyFavouritesQuery, MyFavouritesQueryVariables>;
export const HideEmoteDocument = new TypedDocumentString(`
    mutation HideEmote($id: String!) {
  hideEmote(id: $id)
}
    `) as unknown as TypedDocumentString<HideEmoteMutation, HideEmoteMutationVariables>;
export const UnhideEmoteDocument = new TypedDocumentString(`
    mutation UnhideEmote($id: String!) {
  unhideEmote(id: $id)
}
    `) as unknown as TypedDocumentString<UnhideEmoteMutation, UnhideEmoteMutationVariables>;
export const HideCreatorDocument = new TypedDocumentString(`
    mutation HideCreator($accountId: String!) {
  hideCreator(accountId: $accountId)
}
    `) as unknown as TypedDocumentString<HideCreatorMutation, HideCreatorMutationVariables>;
export const UnhideCreatorDocument = new TypedDocumentString(`
    mutation UnhideCreator($accountId: String!) {
  unhideCreator(accountId: $accountId)
}
    `) as unknown as TypedDocumentString<UnhideCreatorMutation, UnhideCreatorMutationVariables>;
export const FavouriteEmoteDocument = new TypedDocumentString(`
    mutation FavouriteEmote($id: String!) {
  favouriteEmote(id: $id)
}
    `) as unknown as TypedDocumentString<FavouriteEmoteMutation, FavouriteEmoteMutationVariables>;
export const UnfavouriteEmoteDocument = new TypedDocumentString(`
    mutation UnfavouriteEmote($id: String!) {
  unfavouriteEmote(id: $id)
}
    `) as unknown as TypedDocumentString<UnfavouriteEmoteMutation, UnfavouriteEmoteMutationVariables>;
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
    allSets
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  visibility
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  visibility
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
export const ReviewEmotesDocument = new TypedDocumentString(`
    mutation ReviewEmotes($ids: [String!]!, $verdict: ReviewVerdict!, $visibility: EmoteVisibility, $reason: String) {
  reviewEmotes(
    ids: $ids
    verdict: $verdict
    visibility: $visibility
    reason: $reason
  ) {
    done
    failed {
      id
      code
      message
    }
  }
}
    `) as unknown as TypedDocumentString<ReviewEmotesMutation, ReviewEmotesMutationVariables>;
export const RequeueEmotesDocument = new TypedDocumentString(`
    mutation RequeueEmotes($ids: [String!]!) {
  requeueEmotes(ids: $ids) {
    done
    failed {
      id
      code
      message
    }
  }
}
    `) as unknown as TypedDocumentString<RequeueEmotesMutation, RequeueEmotesMutationVariables>;
export const SetEmoteFlagsDocument = new TypedDocumentString(`
    mutation SetEmoteFlags($id: String!, $input: EmoteFlagsInput!) {
  setEmoteFlags(id: $id, input: $input) {
    ...EmoteFields
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
}`) as unknown as TypedDocumentString<SetEmoteFlagsMutation, SetEmoteFlagsMutationVariables>;
export const DismissReportsDocument = new TypedDocumentString(`
    mutation DismissReports($ids: [String!]!, $note: String) {
  dismissReports(ids: $ids, note: $note) {
    done
    failed {
      id
      code
      message
    }
  }
}
    `) as unknown as TypedDocumentString<DismissReportsMutation, DismissReportsMutationVariables>;
export const EmoteDuplicatesDocument = new TypedDocumentString(`
    query EmoteDuplicates($emoteId: String!) {
  emoteDuplicates(emoteId: $emoteId) {
    sameFile
    similarity
    emote {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
}`) as unknown as TypedDocumentString<EmoteDuplicatesQuery, EmoteDuplicatesQueryVariables>;
export const ReviewReasonsDocument = new TypedDocumentString(`
    query ReviewReasons {
  reviewReasons
}
    `) as unknown as TypedDocumentString<ReviewReasonsQuery, ReviewReasonsQueryVariables>;
export const SetReviewReasonsDocument = new TypedDocumentString(`
    mutation SetReviewReasons($reasons: [String!]!) {
  setReviewReasons(reasons: $reasons)
}
    `) as unknown as TypedDocumentString<SetReviewReasonsMutation, SetReviewReasonsMutationVariables>;
export const AccountsByLoginDocument = new TypedDocumentString(`
    query AccountsByLogin($logins: [String!]!) {
  accountsByLogin(logins: $logins) {
    login
    problem
    account {
      id
      displayName
      avatar
    }
  }
}
    `) as unknown as TypedDocumentString<AccountsByLoginQuery, AccountsByLoginQueryVariables>;
export const EmotesWithSameFileDocument = new TypedDocumentString(`
    query EmotesWithSameFile($sha256: String!) {
  emotesWithSameFile(sha256: $sha256) {
    id
    name
    owner {
      id
      displayName
    }
  }
}
    `) as unknown as TypedDocumentString<EmotesWithSameFileQuery, EmotesWithSameFileQueryVariables>;
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
    query ReviewQueue($page: Int, $perPage: Int, $filter: ReviewQueueFilter) {
  reviewQueue(page: $page, perPage: $perPage, filter: $filter) {
    total
    counts {
      public
      limited
    }
    items {
      ...EmoteFields
    }
  }
}
    fragment EmoteFields on Emote {
  id
  name
  state
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
    mutation Approve($id: String!, $visibility: EmoteVisibility) {
  approveEmote(id: $id, visibility: $visibility) {
    id
    state
    visibility
  }
}
    `) as unknown as TypedDocumentString<ApproveMutation, ApproveMutationVariables>;
export const EmoteHistoryDocument = new TypedDocumentString(`
    query EmoteHistory($emoteId: String!) {
  emoteHistory(emoteId: $emoteId) {
    id
    actorId
    action
    before
    after
    createdAt
  }
}
    `) as unknown as TypedDocumentString<EmoteHistoryQuery, EmoteHistoryQueryVariables>;
export const EmoteNotesDocument = new TypedDocumentString(`
    query EmoteNotes($emoteId: String!) {
  emoteNotes(emoteId: $emoteId) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<EmoteNotesQuery, EmoteNotesQueryVariables>;
export const AddEmoteNoteDocument = new TypedDocumentString(`
    mutation AddEmoteNote($emoteId: String!, $body: String!) {
  addEmoteNote(emoteId: $emoteId, body: $body) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<AddEmoteNoteMutation, AddEmoteNoteMutationVariables>;
export const DeleteEmoteNoteDocument = new TypedDocumentString(`
    mutation DeleteEmoteNote($id: String!) {
  deleteEmoteNote(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteEmoteNoteMutation, DeleteEmoteNoteMutationVariables>;
export const RejectDocument = new TypedDocumentString(`
    mutation Reject($id: String!, $reason: String!) {
  rejectEmote(id: $id, reason: $reason) {
    id
    state
  }
}
    `) as unknown as TypedDocumentString<RejectMutation, RejectMutationVariables>;
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
  denies
  public
  invisible
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<RolesQuery, RolesQueryVariables>;
export const LimitDefaultsDocument = new TypedDocumentString(`
    query LimitDefaults {
  limitDefaults {
    ...RoleLimitFields
  }
}
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  windowSeconds
  value
}`) as unknown as TypedDocumentString<LimitDefaultsQuery, LimitDefaultsQueryVariables>;
export const RoleLimitsDocument = new TypedDocumentString(`
    query RoleLimits($id: String!) {
  roleLimits(id: $id) {
    ...RoleLimitFields
  }
}
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  windowSeconds
  value
}`) as unknown as TypedDocumentString<RoleLimitsQuery, RoleLimitsQueryVariables>;
export const SetRoleLimitsDocument = new TypedDocumentString(`
    mutation SetRoleLimits($id: String!, $limits: [RoleLimitInput!]!) {
  setRoleLimits(id: $id, limits: $limits) {
    ...RoleLimitFields
  }
}
    fragment RoleLimitFields on RoleLimit {
  key
  group
  default
  windowSeconds
  value
}`) as unknown as TypedDocumentString<SetRoleLimitsMutation, SetRoleLimitsMutationVariables>;
export const UploadQuotaDocument = new TypedDocumentString(`
    query UploadQuota($ownerId: String) {
  uploadQuota(ownerId: $ownerId) {
    accountId
    pending
    pendingMax
    uploadsThisHour
    uploadsPerHour
    maxBytes
    maxSide
    maxFrames
  }
}
    `) as unknown as TypedDocumentString<UploadQuotaQuery, UploadQuotaQueryVariables>;
export const PublicLimitsDocument = new TypedDocumentString(`
    query PublicLimits {
  limits {
    emote {
      ...MediaLimitFields
    }
    badge {
      ...MediaLimitFields
    }
    paint {
      ...MediaLimitFields
    }
    names {
      emote {
        min
        max
      }
      set {
        min
        max
      }
      badge {
        min
        max
      }
      paint {
        min
        max
      }
      tooltip {
        min
        max
      }
    }
    tags {
      max
      maxLen
    }
    rates {
      key
      count
      windowSeconds
    }
    account {
      key
      default
      effective
      windowSeconds
    }
  }
}
    fragment MediaLimitFields on MediaLimits {
  maxBytes
  maxSide
  maxFrames
  maxTotalPixels
  maxDurationMs
  minSide
  maxAspect
  maxServedSide
}`) as unknown as TypedDocumentString<PublicLimitsQuery, PublicLimitsQueryVariables>;
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
  denies
  public
  invisible
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
  denies
  public
  invisible
  builtIn
  editable
  assignable
}`) as unknown as TypedDocumentString<UpdateRoleMutation, UpdateRoleMutationVariables>;
export const DeleteRoleDocument = new TypedDocumentString(`
    mutation DeleteRole($id: String!) {
  deleteRole(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteRoleMutation, DeleteRoleMutationVariables>;
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
  visibility
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
    id
    name
    kind
    visibility
    capacity
    slotsUsed
    owner {
      id
      displayName
      avatar
    }
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
    allSets
    setIds
  }
  myChannels {
    platform
    platformChannelId
    login
    activeSetId
    account {
      id
    }
  }
  editorChannels {
    platform
    platformChannelId
    login
    activeSetId
    account {
      id
    }
  }
}
    `) as unknown as TypedDocumentString<EditSetQuery, EditSetQueryVariables>;
export const SetEmotesDocument = new TypedDocumentString(`
    query SetEmotes($id: String!, $query: String, $page: Int, $perPage: Int) {
  setEmotes(id: $id, query: $query, page: $page, perPage: $perPage) {
    total
    items {
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
}`) as unknown as TypedDocumentString<SetEmotesQuery, SetEmotesQueryVariables>;
export const SetMembersDocument = new TypedDocumentString(`
    query SetMembers($id: String!, $page: Int, $perPage: Int) {
  setEmotes(id: $id, page: $page, perPage: $perPage) {
    total
    items {
      name
      emote {
        id
      }
    }
  }
}
    `) as unknown as TypedDocumentString<SetMembersQuery, SetMembersQueryVariables>;
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
    visibility
    capacity
  }
}
    `) as unknown as TypedDocumentString<UpdateSetMutation, UpdateSetMutationVariables>;
export const DeleteSetDocument = new TypedDocumentString(`
    mutation DeleteSet($id: String!) {
  deleteSet(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteSetMutation, DeleteSetMutationVariables>;
export const SetCapacityRangeDocument = new TypedDocumentString(`
    query SetCapacityRange($id: String!) {
  setCapacityRange(id: $id) {
    min
    max
    ownerMax
    used
    staff
  }
}
    `) as unknown as TypedDocumentString<SetCapacityRangeQuery, SetCapacityRangeQueryVariables>;
export const SetMenuDocument = new TypedDocumentString(`
    query SetMenu($id: String!) {
  setCapacityRange(id: $id) {
    min
    max
    ownerMax
    used
    staff
  }
  editorOf {
    account {
      id
    }
    permissions
    allSets
    setIds
  }
  myChannels {
    platform
    platformChannelId
    login
    activeSetId
    account {
      id
    }
  }
  editorChannels {
    platform
    platformChannelId
    login
    activeSetId
    account {
      id
    }
  }
}
    `) as unknown as TypedDocumentString<SetMenuQuery, SetMenuQueryVariables>;
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
export const RemoveSetEditorDocument = new TypedDocumentString(`
    mutation RemoveSetEditor($setId: String!, $accountId: String!) {
  removeSetEditor(setId: $setId, accountId: $accountId)
}
    `) as unknown as TypedDocumentString<RemoveSetEditorMutation, RemoveSetEditorMutationVariables>;
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
  processing
  animated
  tags
  host
  createdAt
  channelCount
  visibility
  flags {
    zeroWidth
    private
    wide
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
export const SetEditorsDocument = new TypedDocumentString(`
    query SetEditors($id: String!) {
  setEditors(setId: $id) {
    id
    displayName
    avatar
  }
}
    `) as unknown as TypedDocumentString<SetEditorsQuery, SetEditorsQueryVariables>;
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
    mutation StartCheckout($itemId: String!, $recipient: RecipientInput, $idempotencyKey: String) {
  startCheckout(
    itemId: $itemId
    recipient: $recipient
    idempotencyKey: $idempotencyKey
  ) {
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
    mutation RedeemCode($code: String!, $idempotencyKey: String) {
  redeemCode(code: $code, idempotencyKey: $idempotencyKey) {
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
export const ActiveAnnouncementDocument = new TypedDocumentString(`
    query ActiveAnnouncement {
  activeAnnouncement {
    ...AnnouncementFields
  }
}
    fragment AnnouncementFields on Announcement {
  id
  kind
  text
  linkUrl
  linkLabel
  badgeId
  startsAt
  endsAt
  endedAt
  createdAt
  updatedAt
  live
  emote {
    id
    name
    host
    animated
    files {
      name
      width
      height
      isStatic
    }
  }
}`) as unknown as TypedDocumentString<ActiveAnnouncementQuery, ActiveAnnouncementQueryVariables>;
export const AnnouncementsDocument = new TypedDocumentString(`
    query Announcements {
  announcements {
    ...AnnouncementFields
  }
}
    fragment AnnouncementFields on Announcement {
  id
  kind
  text
  linkUrl
  linkLabel
  badgeId
  startsAt
  endsAt
  endedAt
  createdAt
  updatedAt
  live
  emote {
    id
    name
    host
    animated
    files {
      name
      width
      height
      isStatic
    }
  }
}`) as unknown as TypedDocumentString<AnnouncementsQuery, AnnouncementsQueryVariables>;
export const CreateAnnouncementDocument = new TypedDocumentString(`
    mutation CreateAnnouncement($input: AnnouncementInput!) {
  createAnnouncement(input: $input) {
    ...AnnouncementFields
  }
}
    fragment AnnouncementFields on Announcement {
  id
  kind
  text
  linkUrl
  linkLabel
  badgeId
  startsAt
  endsAt
  endedAt
  createdAt
  updatedAt
  live
  emote {
    id
    name
    host
    animated
    files {
      name
      width
      height
      isStatic
    }
  }
}`) as unknown as TypedDocumentString<CreateAnnouncementMutation, CreateAnnouncementMutationVariables>;
export const UpdateAnnouncementDocument = new TypedDocumentString(`
    mutation UpdateAnnouncement($id: String!, $input: AnnouncementInput!) {
  updateAnnouncement(id: $id, input: $input) {
    ...AnnouncementFields
  }
}
    fragment AnnouncementFields on Announcement {
  id
  kind
  text
  linkUrl
  linkLabel
  badgeId
  startsAt
  endsAt
  endedAt
  createdAt
  updatedAt
  live
  emote {
    id
    name
    host
    animated
    files {
      name
      width
      height
      isStatic
    }
  }
}`) as unknown as TypedDocumentString<UpdateAnnouncementMutation, UpdateAnnouncementMutationVariables>;
export const EndAnnouncementDocument = new TypedDocumentString(`
    mutation EndAnnouncement($id: String!) {
  endAnnouncement(id: $id) {
    ...AnnouncementFields
  }
}
    fragment AnnouncementFields on Announcement {
  id
  kind
  text
  linkUrl
  linkLabel
  badgeId
  startsAt
  endsAt
  endedAt
  createdAt
  updatedAt
  live
  emote {
    id
    name
    host
    animated
    files {
      name
      width
      height
      isStatic
    }
  }
}`) as unknown as TypedDocumentString<EndAnnouncementMutation, EndAnnouncementMutationVariables>;
export const NotificationPrefsDocument = new TypedDocumentString(`
    query NotificationPrefs {
  myNotificationPrefs {
    kind
    notify
    toast
    required
  }
}
    `) as unknown as TypedDocumentString<NotificationPrefsQuery, NotificationPrefsQueryVariables>;
export const SetNotificationPrefsDocument = new TypedDocumentString(`
    mutation SetNotificationPrefs($prefs: [NotificationPrefInput!]!) {
  setNotificationPrefs(prefs: $prefs) {
    kind
    notify
    toast
    required
  }
}
    `) as unknown as TypedDocumentString<SetNotificationPrefsMutation, SetNotificationPrefsMutationVariables>;
export const StaffNotesDocument = new TypedDocumentString(`
    query StaffNotes($accountId: String!) {
  staffNotes(accountId: $accountId) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<StaffNotesQuery, StaffNotesQueryVariables>;
export const AddStaffNoteDocument = new TypedDocumentString(`
    mutation AddStaffNote($accountId: String!, $body: String!) {
  addStaffNote(accountId: $accountId, body: $body) {
    id
    authorId
    body
    createdAt
  }
}
    `) as unknown as TypedDocumentString<AddStaffNoteMutation, AddStaffNoteMutationVariables>;
export const DeleteStaffNoteDocument = new TypedDocumentString(`
    mutation DeleteStaffNote($id: String!) {
  deleteStaffNote(id: $id)
}
    `) as unknown as TypedDocumentString<DeleteStaffNoteMutation, DeleteStaffNoteMutationVariables>;
export const AccountSessionsDocument = new TypedDocumentString(`
    query AccountSessions($accountId: String!) {
  accountSessions(accountId: $accountId) {
    id
    kind
    device
    createdAt
    lastSeenAt
    expiresAt
  }
}
    `) as unknown as TypedDocumentString<AccountSessionsQuery, AccountSessionsQueryVariables>;
export const AccountSubscriptionsDocument = new TypedDocumentString(`
    query AccountSubscriptions($accountId: String!) {
  accountSubscriptions(accountId: $accountId) {
    id
    plan
    state
    startsAt
    endsAt
    source
  }
}
    `) as unknown as TypedDocumentString<AccountSubscriptionsQuery, AccountSubscriptionsQueryVariables>;
export const EntitlementGraphDocument = new TypedDocumentString(`
    query EntitlementGraph($accountId: String!) {
  entitlementGraph(accountId: $accountId) {
    ...GrantFields
  }
}
    fragment GrantFields on EntitlementGrant {
  kind
  itemId
  itemName
  accountId
  source
  reason
  grantedBy
  grantedAt
  orderId
  orderName
  orderStatus
  codeBatchId
  codeBatchName
  revokedAt
  revokedBy
  revokeReason
  active
}`) as unknown as TypedDocumentString<EntitlementGraphQuery, EntitlementGraphQueryVariables>;
export const EntitlementHoldersDocument = new TypedDocumentString(`
    query EntitlementHolders($kind: GrantKind!, $id: String!) {
  entitlementHolders(kind: $kind, id: $id) {
    ...GrantFields
  }
}
    fragment GrantFields on EntitlementGrant {
  kind
  itemId
  itemName
  accountId
  source
  reason
  grantedBy
  grantedAt
  orderId
  orderName
  orderStatus
  codeBatchId
  codeBatchName
  revokedAt
  revokedBy
  revokeReason
  active
}`) as unknown as TypedDocumentString<EntitlementHoldersQuery, EntitlementHoldersQueryVariables>;
export const ActivityTopActorsDocument = new TypedDocumentString(`
    query ActivityTopActors($accountId: String, $emoteId: String, $filter: ActivityFilter, $limit: Int) {
  activityTopActors(
    accountId: $accountId
    emoteId: $emoteId
    filter: $filter
    limit: $limit
  ) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<ActivityTopActorsQuery, ActivityTopActorsQueryVariables>;
export const AuditTopPeopleDocument = new TypedDocumentString(`
    query AuditTopPeople($by: AuditPersonRole, $targetKind: String, $targetId: String, $actorId: String, $action: String, $since: String, $until: String, $limit: Int) {
  auditTopPeople(
    by: $by
    targetKind: $targetKind
    targetId: $targetId
    actorId: $actorId
    action: $action
    since: $since
    until: $until
    limit: $limit
  ) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<AuditTopPeopleQuery, AuditTopPeopleQueryVariables>;
export const OrderTopBuyersDocument = new TypedDocumentString(`
    query OrderTopBuyers($status: OrderStatus, $itemId: String, $limit: Int) {
  orderTopBuyers(status: $status, itemId: $itemId, limit: $limit) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<OrderTopBuyersQuery, OrderTopBuyersQueryVariables>;
export const ReviewTopUploadersDocument = new TypedDocumentString(`
    query ReviewTopUploaders($filter: ReviewQueueFilter, $limit: Int) {
  reviewTopUploaders(filter: $filter, limit: $limit) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<ReviewTopUploadersQuery, ReviewTopUploadersQueryVariables>;
export const EntitlementTopHoldersDocument = new TypedDocumentString(`
    query EntitlementTopHolders($limit: Int) {
  entitlementTopHolders(limit: $limit) {
    ...PersonCountFields
  }
}
    fragment PersonCountFields on PersonCount {
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
  count
}`) as unknown as TypedDocumentString<EntitlementTopHoldersQuery, EntitlementTopHoldersQueryVariables>;