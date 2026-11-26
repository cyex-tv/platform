// GraphQL operations: roles. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const ROLE_FIELDS = graphql(`
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
`);

export const ROLES = graphql(`
  query Roles {
    roles {
      ...RoleFields
    }
    permissions
  }
`);

export const ROLE_LIMIT_FIELDS = graphql(`
  fragment RoleLimitFields on RoleLimit {
    key
    group
    default
    max
    windowSeconds
    value
  }
`);

/** The per-account limits with their defaults, for a role that doesn't exist yet. */
export const LIMIT_DEFAULTS = graphql(`
  query LimitDefaults {
    limitDefaults {
      ...RoleLimitFields
    }
  }
`);

export const ROLE_LIMITS = graphql(`
  query RoleLimits($id: String!) {
    roleLimits(id: $id) {
      ...RoleLimitFields
    }
  }
`);

export const SET_ROLE_LIMITS = graphql(`
  mutation SetRoleLimits($id: String!, $limits: [RoleLimitInput!]!) {
    setRoleLimits(id: $id, limits: $limits) {
      ...RoleLimitFields
    }
  }
`);

/** How many of the account's emotes wait for review, and how many may (the upload form). */
export const UPLOAD_QUOTA = graphql(`
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
      maxFps
      maxAspect
    }
  }
`);

export const MEDIA_LIMIT_FIELDS = graphql(`
  fragment MediaLimitFields on MediaLimits {
    maxBytes
    maxSide
    maxFrames
    maxFps
    maxTotalPixels
    maxDurationMs
    minSide
    maxAspect
    maxServedSide
  }
`);

/** Every upload and field limit, public; signed in, the caller's own per-account limits too. */
export const PUBLIC_LIMITS = graphql(`
  query PublicLimits {
    limits {
      emote { ...MediaLimitFields }
      badge { ...MediaLimitFields }
      paint { ...MediaLimitFields }
      names {
        emote { min max }
        set { min max }
        badge { min max }
        paint { min max }
        tooltip { min max }
      }
      tags { max maxLen }
      rates { key count windowSeconds }
      account { key default effective windowSeconds }
    }
  }
`);

export const CREATE_ROLE = graphql(`
  mutation CreateRole($input: CreateRoleInput!) {
    createRole(input: $input) {
      ...RoleFields
    }
  }
`);

export const UPDATE_ROLE = graphql(`
  mutation UpdateRole($id: String!, $input: UpdateRoleInput!) {
    updateRole(id: $id, input: $input) {
      ...RoleFields
    }
  }
`);

export const DELETE_ROLE = graphql(`
  mutation DeleteRole($id: String!) {
    deleteRole(id: $id)
  }
`);
