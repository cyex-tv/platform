# Takedowns on cyex.tv

How cyex.tv handles images that must not be there: someone else's copyrighted work, personal images, illegal content, or anything against the platform rules of Twitch or Kick.

## Reporting

- **cyex.tv users:** report an emote from its page (Report, with a reason). Reports go into a queue that moderators work through.
- **Everyone else (rights holders, people in an image, authorities):** email **abuse@cyex.tv** with:
  - the emote's URL (`https://cyex.tv/emotes/<id>`) or its CDN URL (`https://cdn.cyex.tv/emotes/<hash>/…`);
  - what the problem is, and for copyright, what work it copies and that you hold the rights or act for whoever does;
  - how to reach you.

We answer within a few days, and act sooner on illegal content.

## What a takedown does

Taking an emote down needs the `emotes.takedown` permission. It happens in one step, and the audit log records who did it and why.

1. **The image goes everywhere it is used.** Uploads of the same file share it, so every emote made from that image is taken down, not just the reported one.
2. **It leaves every set at once.** Connected clients (the deck, the extension, chat clients) are told live (`emote.updated`, then `set.emote_removed`) and drop it.
3. **It can't come back.** The file's hash and its perceptual hash (dHash) go on cyex.tv's blocklist, so re-uploads and near copies are refused.
4. **The files are deleted** from storage, and their CDN URLs are purged from Cloudflare's cache.
5. **The owners are told,** with the reason, in their cyex.tv notifications.
6. **Open reports on it are closed** as actioned.

A dismissed report leaves the emote as it is; that is audited too.

## Accounts

Holders of `users.restrict` can block an account's uploads, or the whole account (no sign-in; its sessions end at once). The owner's account can't be restricted. Every restriction is audited.

## For the operator

- The takedown runs from the admin pages (Phase 6) or GraphQL: `resolveReport(action: TAKE_DOWN)` or `takedownEmote(id, reason)`.
- CDN purging needs `CYEX_TV_CLOUDFLARE_ZONE_ID` and `CYEX_TV_CLOUDFLARE_PURGE_TOKEN` (an API token with Cache Purge on the zone) in the worker's env. Without them the files are still deleted, and cached copies expire on their own.
- Legal requests that need more than a takedown (data requests, court orders) go to the owner.
