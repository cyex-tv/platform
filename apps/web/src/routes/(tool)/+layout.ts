// The signed-in pages render in the browser only: the session cookie belongs to the API's
// domain, so the server can't see who you are.
export const ssr = false;
export const prerender = false;
