// GraphQL operations: site. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// The site announcement under the header, and its admin page.
export const ANNOUNCEMENT_FIELDS = graphql(`
  fragment AnnouncementFields on Announcement {
    id
    kind
    text
    linkUrl
    linkLabel
    badgeId
    startsAt
    endsAt
    endedAt
    createdAt
    updatedAt
    live
    emote {
      id
      name
      host
      animated
      files {
        name
        width
        height
        isStatic
      }
    }
  }
`);

export const ACTIVE_ANNOUNCEMENT = graphql(`
  query ActiveAnnouncement {
    activeAnnouncement {
      ...AnnouncementFields
    }
  }
`);

export const ANNOUNCEMENTS = graphql(`
  query Announcements {
    announcements {
      ...AnnouncementFields
    }
  }
`);

export const CREATE_ANNOUNCEMENT = graphql(`
  mutation CreateAnnouncement($input: AnnouncementInput!) {
    createAnnouncement(input: $input) {
      ...AnnouncementFields
    }
  }
`);

export const UPDATE_ANNOUNCEMENT = graphql(`
  mutation UpdateAnnouncement($id: String!, $input: AnnouncementInput!) {
    updateAnnouncement(id: $id, input: $input) {
      ...AnnouncementFields
    }
  }
`);

export const END_ANNOUNCEMENT = graphql(`
  mutation EndAnnouncement($id: String!) {
    endAnnouncement(id: $id) {
      ...AnnouncementFields
    }
  }
`);

// Settings → Notifications.
export const NOTIFICATION_PREFS = graphql(`
  query NotificationPrefs {
    myNotificationPrefs {
      kind
      notify
      toast
      required
    }
  }
`);

export const SET_NOTIFICATION_PREFS = graphql(`
  mutation SetNotificationPrefs($prefs: [NotificationPrefInput!]!) {
    setNotificationPrefs(prefs: $prefs) {
      kind
      notify
      toast
      required
    }
  }
`);
