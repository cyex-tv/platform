// GraphQL operations: admin users. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

export const USER = graphql(`
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
`);

export const USER_DETAILS = graphql(`
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
`);

// Which role decided each permission for an account (the admin user page).
export const ACCOUNT_PERMISSIONS = graphql(`
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
`);

export const ALL_ROLES = graphql(`
  query AllRoles {
    roles {
      ...RoleFields
    }
  }
`);

export const PLACE_RESTRICTION = graphql(`
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
`);

export const LIFT_RESTRICTION = graphql(`
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
`);

export const ACCOUNT_RESTRICTIONS = graphql(`
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
`);

export const ASSIGN_ROLE = graphql(`
  mutation AssignRole($accountId: String!, $roleId: String!) {
    assignRole(accountId: $accountId, roleId: $roleId)
  }
`);

export const UNASSIGN_ROLE = graphql(`
  mutation UnassignRole($accountId: String!, $roleId: String!) {
    unassignRole(accountId: $accountId, roleId: $roleId)
  }
`);
