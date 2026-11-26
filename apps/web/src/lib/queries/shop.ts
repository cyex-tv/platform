// GraphQL operations: shop. Types come from `npm run codegen` (src/lib/gql).
import { graphql } from '../gql';

// The shop, redeem codes and orders (ADR 0016).

export const SHOP_ITEM_FIELDS = graphql(`
  fragment ShopItemFields on ShopItem {
    id
    name
    description
    priceCents
    currency
    paddlePriceId
    badges {
      ...BadgeFields
    }
    paints {
      ...PaintFields
    }
    activeFrom
    activeTo
    stockLimit
    stockLeft
    visible
    sortOrder
    available
    owned
    deleted
  }
`);

export const SHOP_ITEMS = graphql(`
  query ShopItems($all: Boolean) {
    shopItems(all: $all) {
      ...ShopItemFields
    }
  }
`);

export const SHOP_ITEM = graphql(`
  query ShopItem($id: String!) {
    shopItem(id: $id) {
      ...ShopItemFields
    }
  }
`);

export const ORDER_FIELDS = graphql(`
  fragment OrderFields on Order {
    id
    buyer {
      id
      displayName
      avatar
    }
    recipient {
      id
      displayName
      avatar
    }
    gift
    itemId
    itemName
    status
    amountCents
    currency
    paidTotalCents
    paidCurrency
    refundReason
    createdAt
    paidAt
    refundedAt
    failedAt
  }
`);

export const START_CHECKOUT = graphql(`
  mutation StartCheckout($itemId: String!, $recipient: RecipientInput, $idempotencyKey: String) {
    startCheckout(itemId: $itemId, recipient: $recipient, idempotencyKey: $idempotencyKey) {
      order {
        ...OrderFields
      }
      transactionId
      checkoutUrl
      clientToken
      environment
    }
  }
`);

export const ORDER = graphql(`
  query Order($id: String!) {
    order(id: $id) {
      ...OrderFields
    }
  }
`);

export const MY_ORDERS = graphql(`
  query MyOrders($page: Int, $perPage: Int) {
    myOrders(page: $page, perPage: $perPage) {
      total
      items {
        ...OrderFields
      }
    }
  }
`);

export const REDEEM_CODE = graphql(`
  mutation RedeemCode($code: String!, $idempotencyKey: String) {
    redeemCode(code: $code, idempotencyKey: $idempotencyKey) {
      name
      badges {
        ...BadgeFields
      }
      paints {
        ...PaintFields
      }
    }
  }
`);

export const CREATE_SHOP_ITEM = graphql(`
  mutation CreateShopItem($input: ShopItemInput!) {
    createShopItem(input: $input) {
      ...ShopItemFields
    }
  }
`);

export const UPDATE_SHOP_ITEM = graphql(`
  mutation UpdateShopItem($id: String!, $input: ShopItemInput!) {
    updateShopItem(id: $id, input: $input) {
      ...ShopItemFields
    }
  }
`);

export const DELETE_SHOP_ITEM = graphql(`
  mutation DeleteShopItem($id: String!) {
    deleteShopItem(id: $id)
  }
`);

export const ADMIN_ORDERS = graphql(`
  query AdminOrders($status: OrderStatus, $accountId: String, $itemId: String, $page: Int, $perPage: Int) {
    orders(status: $status, accountId: $accountId, itemId: $itemId, page: $page, perPage: $perPage) {
      total
      items {
        ...OrderFields
        providerTransactionId
        refundRejectedAt
      }
    }
  }
`);

export const REFUND_ORDER = graphql(`
  mutation RefundOrder($id: String!, $reason: String!) {
    refundOrder(id: $id, reason: $reason) {
      ...OrderFields
      providerTransactionId
      refundRejectedAt
    }
  }
`);

export const CODE_BATCH_FIELDS = graphql(`
  fragment CodeBatchFields on CodeBatch {
    id
    name
    prefix
    itemId
    itemName
    badgeIds
    paintIds
    codeCount
    usesPerCode
    redemptions
    expiresAt
    disabledAt
    createdBy {
      id
      displayName
    }
    createdAt
  }
`);

export const CODE_BATCHES = graphql(`
  query CodeBatches($page: Int, $perPage: Int) {
    codeBatches(page: $page, perPage: $perPage) {
      total
      items {
        ...CodeBatchFields
      }
    }
  }
`);

export const CREATE_CODE_BATCH = graphql(`
  mutation CreateCodeBatch($input: CodeBatchInput!) {
    createCodeBatch(input: $input) {
      batch {
        ...CodeBatchFields
      }
      codes
    }
  }
`);

export const DISABLE_CODE_BATCH = graphql(`
  mutation DisableCodeBatch($id: String!) {
    disableCodeBatch(id: $id) {
      ...CodeBatchFields
    }
  }
`);

export const SHOP_COUNTS = graphql(`
  query ShopCounts($items: Boolean!, $orders: Boolean!, $codes: Boolean!) {
    shopItems(all: true) @include(if: $items) {
      id
    }
    orders(page: 1, perPage: 1) @include(if: $orders) {
      total
    }
    codeBatches(page: 1, perPage: 1) @include(if: $codes) {
      total
    }
  }
`);
