// Types for every GraphQL operation the site sends, generated from the committed SDL (PLAN.md
// Phase 6: never hand-write types the schema describes). `npm run codegen` after a schema or
// query change; CI checks the output is current.
import type { CodegenConfig } from '@graphql-codegen/cli';

const config: CodegenConfig = {
  schema: '../../schema/schema.graphql',
  documents: ['src/lib/queries/*.ts', 'src/lib/paint-studio/queries.ts'],
  generates: {
    'src/lib/gql/': {
      preset: 'client',
      presetConfig: { fragmentMasking: false },
      config: { useTypeImports: true, enumsAsTypes: true, documentMode: 'string' },
    },
  },
  ignoreNoDocuments: false,
};

export default config;
