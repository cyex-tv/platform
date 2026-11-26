// GraphQL operations: cosmetics. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// Chat cosmetics (ADR 0015): badges and name paints. The dev mock answers only the cosmetics part
// of an operation, so these stay in operations of their own.

export const COSMETIC_IMAGE_FIELDS = graphql(`
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
`);

export const BADGE_FIELDS = graphql(`
  fragment BadgeFields on Badge {
    id
    name
    tooltip
    image {
      ...CosmeticImageFields
    }
  }
`);

export const PAINT_FIELDS = graphql(`
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
`);

export const COSMETICS = graphql(`
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
`);

export const USER_COSMETICS = graphql(`
  query UserCosmetics($platform: Platform!, $userIds: [String!]!) {
    userCosmetics(platform: $platform, userIds: $userIds) {
      platformUserId
      badgeId
      paintId
    }
  }
`);

// What accounts show (badge and paint, in full), by account id: public, up to 100 at once.
export const ACCOUNT_COSMETICS = graphql(`
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
`);

// An account's public profile: logins, public roles, each channel's active set, member since.
export const ACCOUNT_PROFILE = graphql(`
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
`);

export const MY_COSMETICS = graphql(`
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
`);

export const SELECT_COSMETICS = graphql(`
  mutation SelectCosmetics($badgeId: String, $paintId: String, $ownerId: String) {
    selectCosmetics(badgeId: $badgeId, paintId: $paintId, ownerId: $ownerId) {
      badgeId
      paintId
    }
  }
`);

export const COSMETIC_IMAGE = graphql(`
  query CosmeticImage($id: String!) {
    cosmeticImage(id: $id) {
      ...CosmeticImageFields
    }
  }
`);

export const ENTITLEMENTS = graphql(`
  query Entitlements($kind: CosmeticKind!, $accountId: String, $cosmeticId: String, $page: Int, $perPage: Int) {
    cosmeticEntitlements(kind: $kind, accountId: $accountId, cosmeticId: $cosmeticId, page: $page, perPage: $perPage) {
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
`);

export const CREATE_BADGE = graphql(`
  mutation CreateBadge($input: CreateBadgeInput!) {
    createBadge(input: $input) {
      ...BadgeFields
    }
  }
`);

export const UPDATE_BADGE = graphql(`
  mutation UpdateBadge($id: String!, $input: UpdateBadgeInput!) {
    updateBadge(id: $id, input: $input) {
      ...BadgeFields
    }
  }
`);

export const REMAKE_COSMETIC_IMAGE = graphql(`
  mutation RemakeCosmeticImage($id: String!) {
    remakeCosmeticImage(id: $id) {
      ...CosmeticImageFields
    }
  }
`);

export const DELETE_BADGE = graphql(`
  mutation DeleteBadge($id: String!) {
    deleteBadge(id: $id)
  }
`);

export const DELETE_PAINT = graphql(`
  mutation DeletePaint($id: String!) {
    deletePaint(id: $id)
  }
`);

export const GRANT_COSMETIC = graphql(`
  mutation GrantCosmetic($input: GrantCosmeticInput!) {
    grantCosmetic(input: $input) {
      cosmeticId
    }
  }
`);

export const REVOKE_COSMETIC = graphql(`
  mutation RevokeCosmetic($accountId: String!, $kind: CosmeticKind!, $cosmeticId: String!, $reason: String) {
    revokeCosmetic(accountId: $accountId, kind: $kind, cosmeticId: $cosmeticId, reason: $reason)
  }
`);
