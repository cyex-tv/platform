// The paint studio's GraphQL operations (kept apart from the site's own queries; codegen reads both).
import { graphql } from '$lib/gql';

export const STUDIO_PAINT_FIELDS = graphql(`
  fragment StudioPaintFields on Paint {
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
      id
      host
      ready
      width
      height
      animated
      files {
        name
        format
        scale
        width
        height
        isStatic
      }
    }
    updatedAt
  }
`);

export const STUDIO_PAINTS = graphql(`
  query StudioPaints {
    cosmetics {
      paints {
        ...StudioPaintFields
      }
    }
  }
`);

export const STUDIO_CREATE_PAINT = graphql(`
  mutation StudioCreatePaint($input: PaintInput!) {
    createPaint(input: $input) {
      ...StudioPaintFields
    }
  }
`);

export const STUDIO_UPDATE_PAINT = graphql(`
  mutation StudioUpdatePaint($id: String!, $input: PaintInput!) {
    updatePaint(id: $id, input: $input) {
      ...StudioPaintFields
    }
  }
`);

export const STUDIO_COSMETIC_IMAGE = graphql(`
  query StudioCosmeticImage($id: String!) {
    cosmeticImage(id: $id) {
      id
      host
      ready
      width
      height
      animated
      files {
        name
        format
        scale
        width
        height
        isStatic
      }
    }
  }
`);
