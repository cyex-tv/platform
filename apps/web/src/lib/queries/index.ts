// Every GraphQL operation the site sends, one file per area; `$lib/queries` re-exports them all.
// `codegen.ts` reads every file in this folder, so a new area is one file here plus a line below.
export * from './emotes';
export * from './channels';
export * from './sets';
export * from './account';
export * from './my-emotes';
export * from './set-editor';
export * from './my-channels';
export * from './editors';
export * from './account-settings';
export * from './review';
export * from './reports';
export * from './roles';
export * from './admin-users';
export * from './audit';
export * from './emote-pages';
export * from './cosmetics';
export * from './profile';
export * from './activity';
export * from './shop';
export * from './search';
export * from './staff';
export * from './review-batch';
export * from './site';
export * from './personal';
export * from './top-people';
