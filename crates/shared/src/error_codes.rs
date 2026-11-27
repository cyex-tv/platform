//! The stable error codes of the HTTP and GraphQL API, in one place (ADR 0008).
//!
//! Every error answer is `{ "error": "<code>", "message": "<text>" }`, and GraphQL errors carry
//! the same code in `extensions.code`. Clients match on the code; the message is for people.
//! A code is never renamed or reused for another meaning, and new ones are only added.
//!
//! This list is the contract: the OpenAPI document (`components.schemas.ErrorCode`) and the
//! developer docs table are made from it, and `apps/api/tests/error_codes.rs` fails when the
//! source uses a code that is missing here. Adding a code means adding a row here.

/// One error code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorCode {
    /// What clients match on.
    pub code: &'static str,
    /// The area it belongs to, for grouping in the docs.
    pub group: &'static str,
    /// What it says.
    pub meaning: &'static str,
    /// When the API answers with it.
    pub when: &'static str,
}

const fn e(
    group: &'static str,
    code: &'static str,
    meaning: &'static str,
    when: &'static str,
) -> ErrorCode {
    ErrorCode {
        code,
        group,
        meaning,
        when,
    }
}

/// The groups, in the order the docs show them.
pub const GROUPS: [&str; 14] = [
    "General",
    "Auth",
    "Emotes",
    "Sets",
    "Editors",
    "Transfers",
    "Cosmetics",
    "Shop",
    "Codes",
    "Admin",
    "Site",
    "Reports",
    "Developers",
    "GraphQL",
];

/// Every code the API can answer with.
pub const ERROR_CODES: &[ErrorCode] = &[
    e(
        "General",
        "internal",
        "Something failed on our side.",
        "Unexpected server or database failure. Safe to retry later; report it if it keeps happening.",
    ),
    e(
        "General",
        "unavailable",
        "The service or a dependency is not available right now.",
        "Sign-in with a platform is not configured, uploads or storage are down, or the database cannot be reached (503).",
    ),
    e(
        "General",
        "not_implemented",
        "The endpoint is part of the contract but not built yet.",
        "You call a route that answers 501.",
    ),
    e(
        "General",
        "not_found",
        "Nothing with that ID, or you may not see it.",
        "Reading or changing something that does not exist. Private things answer the same way, so existence is not leaked.",
    ),
    e(
        "General",
        "forbidden",
        "Your roles do not allow this.",
        "The signed-in account lacks the permission for the action (403).",
    ),
    e(
        "General",
        "signed_out",
        "You must sign in first.",
        "A call that needs an account came without a valid session or token (401).",
    ),
    e(
        "General",
        "insufficient_scope",
        "This app token may not do that.",
        "A token made for an app (not the website) calls something the app scope does not cover.",
    ),
    e(
        "General",
        "website_only",
        "Only the website, signed in there, may do this.",
        "Account deletion and similar actions called with an app token or console token.",
    ),
    e(
        "General",
        "restricted",
        "The account is restricted.",
        "A restricted account tries an action its restrictions block, like uploading or deleting itself.",
    ),
    e(
        "General",
        "own_account",
        "You cannot do this to your own account.",
        "Staff change restrictions on their own account, or act on themselves where that makes no sense.",
    ),
    e(
        "General",
        "owner_protected",
        "The owner's account is protected.",
        "Someone tries to restrict, block or delete an owner account.",
    ),
    e(
        "General",
        "owner_only",
        "Only an owner may do this.",
        "Giving the User role a staff permission, or other owner-only changes.",
    ),
    e(
        "General",
        "rate_limited",
        "Too many requests or tries.",
        "Over a rate or per-account limit. Wait the seconds in the Retry-After header, then go on.",
    ),
    e(
        "General",
        "locked_out",
        "Too many wrong codes, locked out for a while.",
        "Several wrong redeem codes in a row. Retry-After holds the wait.",
    ),
    e(
        "General",
        "too_many",
        "More items in one request than allowed.",
        "A batch call (look up users, emotes, channels or sets, mark notifications, and so on) goes \n         over its cap, usually 100 (25 for sets).",
    ),
    e(
        "General",
        "too_many_fields",
        "The GraphQL query asks for too much at once.",
        "One request aliases or repeats fields past the cost cap. Split it.",
    ),
    e(
        "General",
        "bad_origin",
        "Requests with the session cookie must come from cyex.tv.",
        "A cookie-signed POST from another origin (403).",
    ),
    e(
        "General",
        "bad_idempotency_key",
        "The Idempotency-Key is not a UUID.",
        "A write sends the header in another format.",
    ),
    e(
        "General",
        "idempotency_key_reused",
        "The idempotency key was used for a different request.",
        "The same key arrives with a different body or operation.",
    ),
    e(
        "General",
        "idempotency_in_progress",
        "The first request with this key is still running.",
        "A retry with the same key arrives before the first finished (409). Try again shortly.",
    ),
    e(
        "General",
        "bad_form",
        "The upload form is malformed.",
        "A multipart upload misses its file, repeats a field or sends a field that is too long.",
    ),
    e(
        "General",
        "bad_name",
        "A name is empty, too long or has characters that are not allowed.",
        "Emote, set, role, badge, paint or tag names break the length or character rules.",
    ),
    e(
        "General",
        "bad_text",
        "Text is outside the allowed length.",
        "An announcement or similar text is empty or too long.",
    ),
    e(
        "General",
        "bad_description",
        "A description is too long.",
        "Descriptions are at most 500 characters.",
    ),
    e(
        "General",
        "bad_reason",
        "A reason is empty or too long.",
        "Reports, takedowns and moderator actions need a reason of 1 to 500 characters.",
    ),
    e(
        "General",
        "bad_note",
        "A note is empty or too long.",
        "Staff notes are 1 to 2000 characters.",
    ),
    e(
        "General",
        "bad_time",
        "A time is not in the expected format.",
        "A time field is not an RFC 3339 time like 2026-11-17T00:00:00Z.",
    ),
    e(
        "General",
        "bad_scope",
        "The scope arguments conflict.",
        "Activity queries get both or neither of accountId and emoteId.",
    ),
    e(
        "General",
        "bad_search",
        "The search text is too long.",
        "Searches are at most 100 characters.",
    ),
    e(
        "General",
        "bad_cursor",
        "A paging cursor is not one this list gave out.",
        "`after` was cut, edited or made up, comes from another list, or the list's filters or \
         order changed since. Start again from the first page.",
    ),
    e(
        "General",
        "bad_batch",
        "The batch size is out of range.",
        "A moderation batch has fewer than 1 or more than 100 items.",
    ),
    e(
        "General",
        "bad_kind",
        "The kind is not one of the allowed values.",
        "A cosmetics upload or list names a kind other than badge or paint.",
    ),
    e(
        "General",
        "unknown_kind",
        "That is not a kind of notification.",
        "Notification preferences name an unknown kind.",
    ),
    e(
        "Auth",
        "unknown_platform",
        "The platform must be twitch or kick.",
        "Sign-in or linking names another platform.",
    ),
    e(
        "Auth",
        "bad_return_to",
        "return_to must be a cyex.tv URL.",
        "Sign-in is started with a return address outside cyex.tv.",
    ),
    e(
        "Auth",
        "bad_state",
        "The sign-in expired or was already used.",
        "The OAuth state is missing, wrong or used twice. Start again.",
    ),
    e(
        "Auth",
        "declined",
        "Sign-in was cancelled on the platform.",
        "The person declined on Twitch or Kick.",
    ),
    e(
        "Auth",
        "platform_error",
        "The platform did not answer as expected.",
        "Twitch or Kick failed during sign-in (502). Try again.",
    ),
    e(
        "Auth",
        "identity_taken",
        "That platform account is linked to another cyex.tv account.",
        "Linking a Twitch or Kick account already used elsewhere.",
    ),
    e(
        "Auth",
        "platform_already_linked",
        "This platform is already linked to your account.",
        "Linking a platform you already linked.",
    ),
    e(
        "Auth",
        "last_identity",
        "The last linked account cannot be removed.",
        "Unlinking the only platform identity of an account.",
    ),
    e(
        "Auth",
        "account_deleted",
        "This account was deleted.",
        "Signing in to an account whose undo window ended.",
    ),
    e(
        "Auth",
        "account_blocked",
        "This account is blocked.",
        "Signing in or acting as a blocked account.",
    ),
    e(
        "Auth",
        "unknown_client",
        "Where the request came from is not known.",
        "Redeeming codes or app sign-in from an unregistered client.",
    ),
    e(
        "Auth",
        "confirm_mismatch",
        "The confirmation does not match.",
        "Deleting an account without typing one of your own logins.",
    ),
    e(
        "Auth",
        "nothing_to_restore",
        "There is no deleted account to restore.",
        "restoreAccount without an account in its undo window.",
    ),
    e(
        "Emotes",
        "bad_visibility",
        "That visibility is not allowed.",
        "Setting a set private, or unlisting without the emotes.flags permission.",
    ),
    e(
        "Emotes",
        "nsfw_not_allowed",
        "NSFW emotes are not allowed on cyex.tv.",
        "An upload asks for an NSFW flag. There is none.",
    ),
    e(
        "Emotes",
        "bad_tag",
        "A tag is not valid.",
        "Tags are 1 to 32 letters, digits, - or _.",
    ),
    e(
        "Emotes",
        "too_many_tags",
        "Too many tags.",
        "More than 10 tags on an emote.",
    ),
    e(
        "Emotes",
        "too_many_pending",
        "You already have as many emotes waiting for review as allowed.",
        "Uploading with the pending cap reached (409). See uploadQuota.",
    ),
    e(
        "Emotes",
        "unsupported_format",
        "The file is not a supported image type.",
        "Uploading something other than PNG, GIF, WebP, AVIF or APNG (415).",
    ),
    e(
        "Emotes",
        "invalid_image",
        "The file is not a valid image.",
        "The bytes are broken or the decoder refuses them (422).",
    ),
    e(
        "Emotes",
        "too_large",
        "The file or the picture is too big.",
        "An emote's bytes (8 MB), side (1000 px) or frame count (1000) go over the fixed limits (413); also a badge or paint over its limits.",
    ),
    e(
        "Emotes",
        "blocked",
        "This image cannot be uploaded.",
        "The image matches one that was taken down for good (422).",
    ),
    e(
        "Emotes",
        "bad_sha256",
        "A sha256 is not 64 hex digits.",
        "A review call names an image by a malformed hash.",
    ),
    e(
        "Emotes",
        "emote_unavailable",
        "The emote cannot be used.",
        "Adding an emote that is blocked, deleted or taken down.",
    ),
    e(
        "Emotes",
        "emote_private",
        "Only its owner can add a private emote to sets.",
        "Adding a private emote to someone else's set.",
    ),
    e(
        "Emotes",
        "not_approved",
        "The emote is not approved yet.",
        "Until approval an emote only goes in its owner's channel sets.",
    ),
    e(
        "Emotes",
        "personal_use_not_approved",
        "The emote is not approved for personal use.",
        "Adding an emote to a personal set before its personal use is approved, or after it was denied (409, ADR 0039).",
    ),
    e(
        "Emotes",
        "bad_approvals",
        "The review asks for nothing, or a denial has no reason.",
        "reviewApprovals without publicListing or personalUse, or with a DENY and no reason (ADR 0039).",
    ),
    e(
        "Emotes",
        "already_taken_down",
        "The emote is taken down already.",
        "Taking down an emote that is gone.",
    ),
    e(
        "Emotes",
        "not_pending",
        "Only pending items can do that.",
        "Requeueing an emote that is not pending, or answering a transfer that is not pending.",
    ),
    e(
        "Emotes",
        "bad_emote",
        "That emote does not exist.",
        "A site setting or list names an emote ID that does not exist.",
    ),
    e(
        "Emotes",
        "too_many_reasons",
        "Too many ready reasons.",
        "A review reason list has more than 12.",
    ),
    e(
        "Emotes",
        "bad_logins",
        "The login list is out of range.",
        "Give 1 to 100 logins at a time.",
    ),
    e(
        "Sets",
        "set_full",
        "The set is full.",
        "Adding an emote to a set with no free slot (409).",
    ),
    e(
        "Sets",
        "capacity_too_low",
        "The set already holds more emotes than that.",
        "Lowering capacity below the emotes in the set.",
    ),
    e(
        "Sets",
        "capacity_over_limit",
        "The capacity is above what you may have.",
        "Raising capacity over your cap (1000 normally).",
    ),
    e(
        "Sets",
        "bad_capacity",
        "The capacity is not a valid number.",
        "Capacity is a whole number from 1 to 1,000,000.",
    ),
    e(
        "Sets",
        "name_taken",
        "That name is already used.",
        "An emote name repeats inside a set, or a badge or paint name repeats.",
    ),
    e(
        "Sets",
        "already_in_set",
        "The emote is already in the set.",
        "Adding an emote twice.",
    ),
    e(
        "Sets",
        "set_in_use",
        "A channel shows this set.",
        "Deleting an active set. Pick another active set first.",
    ),
    e(
        "Sets",
        "invalid_set_kind",
        "Sets of this kind cannot be made or changed this way.",
        "Editing a global or personal set through the wrong call.",
    ),
    e(
        "Sets",
        "invalid_active_set",
        "The active set is not one of the channel owner's sets.",
        "Setting a channel's active set to someone else's set.",
    ),
    e(
        "Sets",
        "personal_set_exists",
        "You already have a personal set.",
        "Making a second personal set.",
    ),
    e(
        "Sets",
        "bad_channel",
        "The channel is not identified properly.",
        "Give the channel's ID or login (one of them, up to 64 characters).",
    ),
    e(
        "Sets",
        "bad_set",
        "The set choice is not valid.",
        "Editor permissions name sets that do not belong to the account.",
    ),
    e(
        "Sets",
        "limit_reached",
        "A personal list limit is reached.",
        "Hidden or favourite lists are full.",
    ),
    e(
        "Editors",
        "already_editor",
        "They are an editor already.",
        "Adding an editor twice. Change their permissions instead.",
    ),
    e(
        "Editors",
        "already_owner",
        "The owner can edit it already.",
        "Adding the set's owner as an editor.",
    ),
    e(
        "Editors",
        "too_many_editors",
        "An account has too many editors.",
        "More than 50 editors on one account.",
    ),
    e(
        "Editors",
        "too_many_sets",
        "Too many sets picked.",
        "An editor is limited to more than 200 sets.",
    ),
    e(
        "Editors",
        "no_permissions",
        "No permission was picked.",
        "Editors need at least one permission. To take all away, remove the editor.",
    ),
    e(
        "Editors",
        "retired_permission",
        "That permission is not available for editors any more.",
        "Giving editors the transfer permission.",
    ),
    e(
        "Editors",
        "bad_editor",
        "The editor is not identified properly.",
        "Give the editor's account ID, or a platform and a login.",
    ),
    e(
        "Transfers",
        "transfer_staff_only",
        "Only staff with emotes.transfer can change who owns an emote.",
        "transferEmote without the emotes.transfer permission, the emote's owner included (403, ADR 0039).",
    ),
    e(
        "Transfers",
        "transfer_requests_retired",
        "Transfer requests are gone: ownership can't be requested.",
        "acceptEmoteTransfer, declineEmoteTransfer or cancelEmoteTransfer (410, ADR 0039).",
    ),
    e(
        "Transfers",
        "same_owner",
        "That account owns the emote already.",
        "Transferring an emote to its owner.",
    ),
    e(
        "Transfers",
        "expired",
        "The request expired.",
        "Accepting a transfer request after its time ran out.",
    ),
    e(
        "Transfers",
        "recipient_not_found",
        "No account has that login.",
        "Sending a transfer or a gift to an unknown login.",
    ),
    e(
        "Cosmetics",
        "bad_color",
        "A colour is not valid.",
        "Use #RRGGBB or #RRGGBBAA.",
    ),
    e(
        "Cosmetics",
        "bad_angle",
        "The angle is not valid.",
        "Gradient angles are 0 to 360 degrees.",
    ),
    e(
        "Cosmetics",
        "bad_stops",
        "The gradient stops are not valid.",
        "Gradients take 2 to 10 stops.",
    ),
    e(
        "Cosmetics",
        "bad_shadows",
        "Too many shadows.",
        "At most 4 shadows on a paint.",
    ),
    e(
        "Cosmetics",
        "bad_tooltip",
        "The tooltip is not valid.",
        "Tooltips are 1 to 64 characters.",
    ),
    e(
        "Cosmetics",
        "bad_image",
        "The image does not fit this paint.",
        "Only image paints take an image.",
    ),
    e(
        "Cosmetics",
        "direct_upload_off",
        "Direct uploads aren't set up on this server.",
        "Asking for a direct upload URL where storage has no public upload address (501). Send the picture to POST /v1/cosmetics/images instead.",
    ),
    e(
        "Cosmetics",
        "too_many_uploads",
        "You have as many direct uploads open as allowed.",
        "Asking for another direct upload URL while that many are neither finalized nor expired (409, with details).",
    ),
    e(
        "Cosmetics",
        "upload_missing",
        "Nothing was uploaded to that direct upload yet.",
        "Finalizing before the PUT to the upload URL finished (409). Upload, then finalize again.",
    ),
    e(
        "Cosmetics",
        "upload_expired",
        "The direct upload has expired.",
        "Finalizing an upload past its expiry (410). Ask for a new upload URL.",
    ),
    e(
        "Cosmetics",
        "size_mismatch",
        "The uploaded file isn't the size that was declared.",
        "Finalizing an upload whose stored object differs from the declared size (422). The upload is discarded.",
    ),
    e(
        "Cosmetics",
        "already_finalized",
        "The direct upload was already finalized.",
        "Finalizing the same upload twice (409). The first answer's picture stands.",
    ),
    e(
        "Cosmetics",
        "bad_dimensions",
        "The picture's size does not fit.",
        "Badge or paint pictures outside the allowed size or aspect, and an emote wider than 4:1 (422).",
    ),
    e(
        "Cosmetics",
        "image_not_ready",
        "The picture's files are still being made.",
        "Using a picture before the worker finished it. Try again shortly.",
    ),
    e(
        "Cosmetics",
        "original_missing",
        "The picture's original is gone from storage.",
        "Remaking files for a picture whose original was lost. Upload it again.",
    ),
    e(
        "Cosmetics",
        "not_owned",
        "You do not own that cosmetic.",
        "Showing a badge or paint the account does not own.",
    ),
    e(
        "Cosmetics",
        "bad_badge",
        "That badge does not exist.",
        "A site setting names an unknown badge ID.",
    ),
    e(
        "Cosmetics",
        "bad_cosmetics",
        "A badge or paint does not exist.",
        "A grant or a shop item names an unknown cosmetic.",
    ),
    e(
        "Shop",
        "checkout_unavailable",
        "Buying is not available.",
        "Checkout is off or not set up (503).",
    ),
    e(
        "Shop",
        "checkout_failed",
        "The checkout could not be started.",
        "The payment provider refused to start it. Nothing was charged (503).",
    ),
    e(
        "Shop",
        "checkout_busy",
        "A checkout for this is starting already.",
        "A second checkout for the same item at the same moment.",
    ),
    e(
        "Shop",
        "payment_provider_error",
        "The payment provider answered wrongly.",
        "Paddle failed or sent something unexpected (502).",
    ),
    e(
        "Shop",
        "bad_payload",
        "Not a valid payment notification.",
        "The webhook body does not parse.",
    ),
    e(
        "Shop",
        "bad_item",
        "No shop item with that ID.",
        "Checkout or redeem names an unknown item.",
    ),
    e(
        "Shop",
        "item_unavailable",
        "That item is not on sale.",
        "Buying an item that is off sale.",
    ),
    e(
        "Shop",
        "sold_out",
        "The item is sold out.",
        "Buying an item with no stock left.",
    ),
    e(
        "Shop",
        "already_owned",
        "You own everything this gives.",
        "Buying or redeeming something you already own.",
    ),
    e(
        "Shop",
        "bad_price",
        "The price is not valid.",
        "Prices are 0 to 100000 cents.",
    ),
    e(
        "Shop",
        "bad_price_id",
        "The Paddle price ID is not valid.",
        "Price IDs look like pri_01h... in lower case.",
    ),
    e(
        "Shop",
        "bad_currency",
        "The currency is not valid.",
        "Currencies are three letters, like EUR.",
    ),
    e(
        "Shop",
        "bad_stock",
        "The stock is not valid.",
        "Stock is 1 to 1,000,000, or none for no limit.",
    ),
    e(
        "Shop",
        "order_not_ready",
        "The order is not paid yet.",
        "Adjusting an order before payment (503). Try again later.",
    ),
    e(
        "Shop",
        "not_refundable",
        "Only paid orders can be refunded.",
        "Refunding an order that is not paid.",
    ),
    e(
        "Shop",
        "already_refunded",
        "The payment was refunded already.",
        "Refunding twice.",
    ),
    e(
        "Shop",
        "refund_in_progress",
        "A refund of this order is under way.",
        "A second refund while one is pending.",
    ),
    e(
        "Shop",
        "refund_failed",
        "The payment provider did not take the refund.",
        "Paddle refused. Nothing was changed (502).",
    ),
    e(
        "Shop",
        "refund_unavailable",
        "Refunds are not available right now.",
        "Paddle is unreachable or off (503).",
    ),
    e(
        "Codes",
        "bad_code",
        "That code does not work.",
        "A redeem code is unknown or wrongly typed.",
    ),
    e(
        "Codes",
        "already_redeemed",
        "You used that code already.",
        "Redeeming the same code twice.",
    ),
    e(
        "Codes",
        "code_disabled",
        "That code was turned off.",
        "Redeeming a code staff disabled.",
    ),
    e(
        "Codes",
        "code_expired",
        "That code has expired.",
        "Redeeming after the end date.",
    ),
    e(
        "Codes",
        "code_used_up",
        "That code has been used up.",
        "Redeeming a code with no uses left.",
    ),
    e(
        "Codes",
        "bad_count",
        "The number of codes is out of range.",
        "A batch has 1 to 1000 codes.",
    ),
    e(
        "Codes",
        "bad_prefix",
        "The prefix is not valid.",
        "Prefixes are 1 to 12 letters or digits.",
    ),
    e(
        "Codes",
        "bad_uses",
        "The number of uses is out of range.",
        "A code can be used 1 to 10000 times.",
    ),
    e(
        "Admin",
        "bad_permissions",
        "The permission list is contradictory.",
        "A role allows and denies the same permission.",
    ),
    e(
        "Admin",
        "unknown_permission",
        "That permission does not exist.",
        "A role names an unknown permission.",
    ),
    e(
        "Admin",
        "role_name_taken",
        "A role with that name exists.",
        "Creating or renaming a role to a taken name.",
    ),
    e(
        "Admin",
        "built_in_role",
        "A built-in role cannot change this way.",
        "Editing the fixed parts of the User role.",
    ),
    e(
        "Admin",
        "implicit_role",
        "Nobody is assigned this role.",
        "Granting or removing a role every account holds.",
    ),
    e(
        "Admin",
        "bad_position",
        "The role position is not valid.",
        "Reordering roles to a position that does not exist.",
    ),
    e(
        "Admin",
        "bad_limit",
        "A limit value is not valid.",
        "Role limits are 1 to 1,000,000 (a file limit up to its ceiling) and listed once. Emote limits are fixed, not per role, and always refused.",
    ),
    e(
        "Admin",
        "unknown_limit",
        "That is not a limit.",
        "A role limit key does not exist or is not per role.",
    ),
    e(
        "Site",
        "bad_label",
        "A link label is too long.",
        "Link words are up to 40 characters.",
    ),
    e(
        "Site",
        "bad_link",
        "A link is not valid.",
        "Links start with https:// or are a path on cyex.tv, like /shop.",
    ),
    e(
        "Site",
        "ended",
        "The announcement has ended.",
        "Editing an announcement after its end. Make a new one.",
    ),
    e(
        "Site",
        "required_kind",
        "The notification kind is required.",
        "Turning off notices that are always sent.",
    ),
    e(
        "Reports",
        "already_reported",
        "You reported this emote already.",
        "Filing a second report on the same emote.",
    ),
    e(
        "Reports",
        "not_open",
        "The report was already resolved.",
        "Resolving a report twice.",
    ),
    e(
        "Reports",
        "tickets_blocked",
        "You cannot file reports.",
        "The account was barred from filing reports.",
    ),
    e(
        "Developers",
        "api_key_invalid",
        "That API key is not valid.",
        "A request sends `Authorization: Bearer cyex_live_...` with a key that was revoked, never existed, or whose account is deleted or blocked (401).",
    ),
    e(
        "Developers",
        "api_key_read_only",
        "API keys read public data only.",
        "A request with an API key tries a mutation, an upload, sign-in or anything else that is not a public read (403).",
    ),
    e(
        "Developers",
        "too_many_api_keys",
        "You have as many API keys as you may.",
        "Making a key past the limit per account (10). Revoke one first.",
    ),
    e(
        "Developers",
        "bad_webhook_url",
        "That webhook URL can't be used.",
        "The URL isn't https, has a user name or password, is too long, or points at a private, local or reserved address.",
    ),
    e(
        "Developers",
        "bad_webhook_events",
        "Those event types can't be used.",
        "No event type, or one a webhook can't subscribe to.",
    ),
    e(
        "Developers",
        "too_many_webhooks",
        "This channel has as many webhooks as it may.",
        "Adding a webhook past the limit per channel (5). Delete one first.",
    ),
    e(
        "GraphQL",
        "invalid_topic",
        "That is not a topic.",
        "A subscription topic is malformed.",
    ),
    e(
        "GraphQL",
        "too_many_topics",
        "Too many topics.",
        "A subscription lists more than 500 topics.",
    ),
    e(
        "GraphQL",
        "too_many_subscriptions",
        "Too many subscriptions on one socket.",
        "More than 8 subscriptions on a socket.",
    ),
    e(
        "GraphQL",
        "subscriptions_only",
        "The socket runs subscriptions only.",
        "A query or mutation sent over /v1/gql/ws. Use POST /v1/gql.",
    ),
    e(
        "GraphQL",
        "post_required",
        "Over GET only queries run.",
        "A mutation or subscription sent with GET /v1/gql (405, `Allow: POST`). Send it with          POST.",
    ),
    e(
        "GraphQL",
        "persisted_query_not_found",
        "The persisted query's hash isn't known here.",
        "`extensions.persistedQuery` without the query text, for a hash this server hasn't          stored (the message is `PersistedQueryNotFound`). Send the text with the hash once.",
    ),
    e(
        "GraphQL",
        "persisted_query_mismatch",
        "The hash isn't the query's SHA-256.",
        "`extensions.persistedQuery.sha256Hash` differs from the SHA-256 of the query text sent          with it.",
    ),
    e(
        "GraphQL",
        "bad_persisted_query",
        "The persisted query extension is malformed.",
        "`extensions.persistedQuery` needs `version: 1` and a `sha256Hash` of 64 lowercase hex          digits.",
    ),
];

/// Whether `code` is a listed error code.
pub fn is_known(code: &str) -> bool {
    ERROR_CODES.iter().any(|e| e.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for e in ERROR_CODES {
            assert!(seen.insert(e.code), "{} is listed twice", e.code);
            assert!(
                !e.code.is_empty()
                    && e.code
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{:?} is not snake_case",
                e.code
            );
            assert!(
                GROUPS.contains(&e.group),
                "{} has group {:?}",
                e.code,
                e.group
            );
            assert!(!e.meaning.is_empty() && !e.when.is_empty(), "{}", e.code);
        }
        assert!(is_known("rate_limited") && !is_known("nope"));
    }
}
