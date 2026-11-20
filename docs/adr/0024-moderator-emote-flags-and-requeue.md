# 0024: Moderator emote flags, and undone approvals go to the end of the queue

- Status: accepted
- Date: 2026/11/20
- Amends: 0010 (a new permission), 0020 (who changes visibility)

## Context
The owner's answers of 2026/11/20: moderators and above with the right permission can change an emote's flags after review, for example take a public emote off the listings so it is unlisted again. And the review queue's 5 second undo stays, but an upload whose approval was taken back goes to the end of the queue, not back to its old place.

## Decision
**Emote flags**
- Permission `emotes.flags` ("Edit emote flags" in the role editor's Emotes group; its tooltip says it covers the hidden flags too). Migration `20261210006002_emote_flags_permission.sql` gives it to the built-in Admin role; no other role changes. `emotes.edit_any`, which already changes everything about an emote, also covers it.
- `setEmoteFlags(id, input: { visibility, nsfw, zeroWidth })`: any field left out stays. Works on an emote in any state, not only during review, and never changes its name, tags or review state.
- Audited as `emote.flags_changed` (before and after: visibility, nsfw, zero_width; `via: emotes.flags`). A call that changes nothing writes nothing.
- When the visibility changes, the owner gets `emote.visibility_changed` (`{ emote_id, name, visibility, was }`), which can't be turned off, as a takedown can't. Not sent when staff change their own emote.
- Hidden flags: today the emote's flags that only staff see are its listing (visibility) and nothing else. A moderation-only flag added later goes in the same input and the same audit row.
- Website: an "Emote flags" panel for holders on the emote page, the review detail and the admin user page's uploads.

**Requeue on undo**
- An approval waits 5 seconds on the reviewer's page before it is sent (the API has no way to take one back), so the server never saw it. On Undo the page puts the card at the end of what it shows and calls `requeueEmotes(ids)` (needs `emotes.review`, up to 100, audited as `emote.requeued`, not-pending ones listed in `failed`).
- Migration `20261210006001_emotes_requeued_at.sql` adds `emotes.requeued_at`. The queue's upload-time order reads `coalesce(requeued_at, created_at)`, so for every reviewer the upload sorts after everything that was waiting. A verdict clears it.

## Consequences
- Newest-first (`descending`) shows a requeued upload first, as it is the latest to (re)enter the queue.
- The other sorts (uploader, name, review time) are unchanged.
