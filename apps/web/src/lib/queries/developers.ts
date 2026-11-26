// The developer platform: your API keys (ADR 0033) and a channel's webhooks (ADR 0034). A key and
// a webhook secret come back only from the mutation that makes them; nothing here lists them.
import { graphql } from '../gql';

export const MY_API_KEYS = graphql(`
  query MyApiKeys {
    myApiKeys {
      id
      name
      prefix
      createdAt
      lastUsedAt
      usage {
        day
        requests
      }
    }
  }
`);

export const CREATE_API_KEY = graphql(`
  mutation CreateApiKey($name: String!) {
    createApiKey(name: $name) {
      key
      apiKey {
        id
        name
        prefix
        createdAt
        lastUsedAt
        usage {
          day
          requests
        }
      }
    }
  }
`);

export const REVOKE_API_KEY = graphql(`
  mutation RevokeApiKey($id: String!) {
    revokeApiKey(id: $id)
  }
`);

export const WEBHOOK_FIELDS = graphql(`
  fragment WebhookFields on Webhook {
    id
    url
    events
    enabled
    disabledReason
    failedDeliveries
    createdAt
  }
`);

export const CHANNEL_WEBHOOKS = graphql(`
  query ChannelWebhooks($platform: Platform!, $id: String!) {
    channelWebhooks(platform: $platform, id: $id) {
      ...WebhookFields
    }
  }
`);

export const CREATE_WEBHOOK = graphql(`
  mutation CreateWebhook($platform: Platform!, $id: String!, $input: CreateWebhookInput!) {
    createWebhook(platform: $platform, id: $id, input: $input) {
      secret
      webhook {
        ...WebhookFields
      }
    }
  }
`);

export const UPDATE_WEBHOOK = graphql(`
  mutation UpdateWebhook($id: String!, $input: UpdateWebhookInput!) {
    updateWebhook(id: $id, input: $input) {
      ...WebhookFields
    }
  }
`);

export const DELETE_WEBHOOK = graphql(`
  mutation DeleteWebhook($id: String!) {
    deleteWebhook(id: $id)
  }
`);

export const ROTATE_WEBHOOK_SECRET = graphql(`
  mutation RotateWebhookSecret($id: String!) {
    rotateWebhookSecret(id: $id) {
      secret
      webhook {
        ...WebhookFields
      }
    }
  }
`);

export const TEST_WEBHOOK = graphql(`
  mutation TestWebhook($id: String!) {
    testWebhook(id: $id) {
      id
      state
    }
  }
`);

export const WEBHOOK_DELIVERIES = graphql(`
  query WebhookDeliveries($id: String!) {
    webhookDeliveries(id: $id) {
      id
      eventType
      eventId
      state
      attempts
      responseStatus
      error
      durationMs
      createdAt
      finishedAt
    }
  }
`);
