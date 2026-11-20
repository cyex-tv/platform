# 0022: Personal lists ("Hide for me" and favourites)

Status: accepted (2026-11-20)

## Context

People want to stop seeing an emote, or everything by one creator, without reporting it, and to
keep a short list of emotes they like. Both lists are about one viewer and must stay private.

## Decision

- Three tables (`hidden_emotes`, `hidden_creators`, `favourite_emotes`), keyed by account. Caps:
  1000 hidden emotes, 200 hidden creators, 1000 favourites, checked under a lock on the account row
  (`limit_reached`). Changes count against a per-account rate (`personal_lists`, 120 a minute).
- GraphQL, additive: `hiddenEmotes`, `hiddenCreators`, `myFavourites(query, page, perPage)`,
  `favouriteEmoteIds`; `hideEmote`/`unhideEmote`, `hideCreator`/`unhideCreator`,
  `favouriteEmote`/`unfavouriteEmote`. No field takes an account argument, so nobody else can read
  or change a list. Hiding or favouriting needs an emote the caller may see.
- **Filtering happens in the client.** The public reads (`searchEmotes`, trending, `quickSearch`,
  set reads) stay the same for everyone. A server-side "without my hidden" parameter would make
  every signed-in search answer personal: the read cache and the CDN could no longer share them,
  and each read would join a private table. The lists are small (at most 1200 IDs), so the site
  loads them once per account and drops matching emotes from what it shows. The cost: a page of
  50 can show fewer than 50; the next page fills in.
- Favourites first in the header search and the set editor's Add emotes: the site asks
  `myFavourites(query)` beside the normal search and puts the matches first.
- The lists are in the data export and are erased with the account.
- The browser extension has no cyex.tv sign-in yet, so it keeps its own device-only hidden list.
  Syncing waits for an extension sign-in path.
