# 0007: Licence and borrowed code

- Status: accepted
- Date: 2026/11/16

## Context
The repos are public from the start. Other emote platforms' code has restrictive licences:
- the 7TV Extension is Apache-2.0 + Commons Clause;
- the 7TV monorepo is multi-licensed with a CLA;
- BTTV uses a custom licence that forbids redistribution.

FFZ's client is Apache-2.0.

## Decision
- **AGPL-3.0** for `cyex-tv/platform`, `cyex-tv/extension` and `cyex-tv/infra`. Anyone who runs a modified copy as a service must publish their changes.
- Clients that only call the public API (Chatterino, other extensions, bots) are not affected.
- **No code is copied from 7TV or BTTV.** Their public protocols and response shapes may be studied and supported. FFZ code may be reused with its Apache-2.0 notice kept.
- `cargo deny` in CI allows only licences compatible with AGPL-3.0, and fails on unknown licences and on advisories. The web and extension use a licence checker with the same allowlist.
- Data licences: `CDLA-Permissive-2.0` is allowed (added 2026/11/16) for bundled CA certificate data (`webpki-root-certs`, pulled in by the TLS verifier). It is permissive, asks only that the licence text travel with the data, and doesn't bind the code around it.
- No CLA for now. Outside contributions are under AGPL-3.0 (inbound = outbound).

## Consequences
- Forks that host cyex.tv must stay open.
- Some companies avoid AGPL code. This doesn't matter for clients, which use the API, not the code.
- If a client library is ever split out for others to embed, it may get a permissive licence in its own ADR.
