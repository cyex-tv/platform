// GraphQL operations: personal. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// Personal lists: "Hide for me" (by emote or creator) and favourites. Private to you; the site
// filters the public reads with them on its side.
export const PERSONAL_LISTS = graphql(`
  query PersonalLists {
    hiddenEmotes {
      emoteId
    }
    hiddenCreators {
      accountId
    }
    favouriteEmoteIds
  }
`);

export const HIDDEN_LISTS = graphql(`
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
`);

export const MY_FAVOURITES = graphql(`
  query MyFavourites($query: String, $page: Int, $perPage: Int) {
    myFavourites(query: $query, page: $page, perPage: $perPage) {
      total
      items {
        ...EmoteFields
      }
    }
  }
`);

export const HIDE_EMOTE = graphql(`
  mutation HideEmote($id: String!) {
    hideEmote(id: $id)
  }
`);

export const UNHIDE_EMOTE = graphql(`
  mutation UnhideEmote($id: String!) {
    unhideEmote(id: $id)
  }
`);

export const HIDE_CREATOR = graphql(`
  mutation HideCreator($accountId: String!) {
    hideCreator(accountId: $accountId)
  }
`);

export const UNHIDE_CREATOR = graphql(`
  mutation UnhideCreator($accountId: String!) {
    unhideCreator(accountId: $accountId)
  }
`);

export const FAVOURITE_EMOTE = graphql(`
  mutation FavouriteEmote($id: String!) {
    favouriteEmote(id: $id)
  }
`);

export const UNFAVOURITE_EMOTE = graphql(`
  mutation UnfavouriteEmote($id: String!) {
    unfavouriteEmote(id: $id)
  }
`);
