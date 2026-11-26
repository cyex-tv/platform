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
