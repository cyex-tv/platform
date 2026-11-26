// GraphQL operations: account settings. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// Which linked platform's name and picture you go by (Settings → Account).
export const SET_DISPLAY_PLATFORM = graphql(`
  mutation SetDisplayPlatform($platform: Platform!) {
    setDisplayPlatform(platform: $platform) {
      id
      displayPlatform
    }
  }
`);

// Settings → Account → Security: every session ends, this one too.
export const SIGN_OUT_EVERYWHERE = graphql(`
  mutation SignOutEverywhere {
    signOutEverywhere
  }
`);

export const UNLINK = graphql(`
  mutation Unlink($platform: Platform!) {
    unlinkIdentity(platform: $platform) {
      id
    }
  }
`);

export const MY_NOTIFICATIONS = graphql(`
  query MyNotifications {
    myNotifications {
      id
      kind
      payload
      createdAt
      read
    }
  }
`);

export const MARK_READ = graphql(`
  mutation MarkRead($ids: [String!]) {
    markNotificationsRead(ids: $ids)
  }
`);

export const UNREAD_COUNT = graphql(`
  query UnreadCount {
    unreadNotificationCount
  }
`);

export const MY_SESSIONS = graphql(`
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
`);

export const REVOKE_SESSION = graphql(`
  mutation RevokeSession($id: String!) {
    revokeSession(id: $id)
  }
`);

export const REVOKE_OTHER_SESSIONS = graphql(`
  mutation RevokeOtherSessions {
    revokeOtherSessions
  }
`);

export const EXPORT_MY_DATA = graphql(`
  mutation ExportMyData {
    exportMyData
  }
`);

export const DELETE_ACCOUNT = graphql(`
  mutation DeleteAccount($confirm: String!) {
    deleteAccount(confirm: $confirm)
  }
`);

// The 30-day undo window (ADR 0019): only the session that signing in to a deleted account gave.
export const PENDING_DELETION = graphql(`
  query PendingDeletion {
    pendingDeletion {
      requestedAt
      eraseAfter
      displayName
      avatar
    }
  }
`);

export const RESTORE_ACCOUNT = graphql(`
  mutation RestoreAccount {
    restoreAccount
  }
`);

// Your own roles, hidden ones too (only for your own profile).
export const MY_ROLES = graphql(`
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
`);
