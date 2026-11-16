# OAuth fixtures

Answers the fake Twitch and Kick servers in `tests/auth_flow.rs` send. They follow the shapes in
the platforms' docs (Twitch: "Getting OAuth Access Tokens" and `Get Users`; Kick: "OAuth 2.1" and
`GET /public/v1/users`), with made-up values. They are not recordings of live answers yet: the
owner's OAuth apps and test accounts are needed for that (`docs/setup/owner-steps.md`). When they
exist, replace these with recorded answers, with tokens and emails scrubbed.

Fields cyex.tv doesn't read (Twitch `view_count`, Kick `email`) stay in, so a test shows that they
are ignored and that `email` is never stored.
