# Gameplay parity and external identity validation — 2026-11-18

Scope: authoritative player controls/command input, NPC-companion inventory, public visual projection, combat-aware logout and optional provider login. Hosting implementation was explicitly deferred by the user. WSS protocol 4 retains additive, default-false capability negotiation; save containers and character identities remain stable.

## Automated checks

| Check | Result |
| --- | --- |
| Network library: bounds, request conversion, retries, capability gating, companion routing, ordered content, socket/cache limits and content identity | 23 passed |
| Engine request suite: ownership, companion membership/revocation at consumption, foreign player denial, walking on host time, camera permissions, declared crafting, combat timing, containers, equipment, purchases and dialogue | 11 passed |
| Dedicated library: provider claim validation, stale/cancelled proof rejection, link/unlink and namespace preservation, passwordless fallback security, safe logout/rejoin, scheduling and existing operations | 11 passed |
| Dedicated account security/recovery integration | 2 passed |
| Runtime prediction, project modes and private replication | 9 + 2 + 9 passed |
| Public single-file/streamed export privacy, integrity and loading | 8 passed |
| Public equipment/rig/light projection unit check | 1 passed |
| Native graphical account settings/rejoin regression | 1 passed |
| Actual WSS private snapshots and certificate-checked HTTPS content | 1 passed |

A focused public-geometry check also verifies string rendering modes, particles, sanitized lights, removal, legacy packages without a light field, and invalid-light rejection. Safe logout is tested with unoccupied-region suspension enabled: departure/combat grace keeps the region awake. Active targeting delays removal, stashed living bodies remain alive, and restart/rejoin preserves the exact body and creation receipts. The default retains bodies; demos keep their existing disposable behavior.

Signed identity tests require a trusted RSA key, issuer, audience/authorized party, expiry, issuance freshness and request nonce. They reject altered signatures/claims and unknown keys. Owner tests reject a provider result begun before global revocation or cancellation, preserve a linked UUID across restart, reject stale unlink, preserve unrelated accounts and refuse to turn an external-only record into a local credential through password-change verification. Disabled password records use hashes of independently random discarded passwords, avoiding a known fallback under older password readers.

## Native and browser compatibility

Native server/Player builds, Player/Creator WASM checks, Creator staticlib and the renderer boundary matrix pass. The dedicated networking additions do not introduce an identity service into single player, Creator tests or demos. A physical Apple-device/Windows/Linux runtime pass is not claimed.

The full public-package WebGPU browser test uses an owned local RSA-signed OIDC device issuer. It registers a local account, changes its password, creates/joins one saved body, links a provider with current-password proof, approves in a separate browser page, signs in through that provider, verifies cancellation stops polling, unlinks with password proof and returns to local login. It rejoins one body throughout and finishes with global logout. No private `.eldiron` source is served. There are zero uncaught browser exceptions and zero framebuffer readbacks.

[Machine-readable browser results](gameplay-parity-and-identity-browser.json) record the actual GPU, identity and transport checks. The fixture uses explicit loopback WS/HTTP; the independent WSS/HTTPS test covers transport certificate validation. Production provider credentials, discovery/JWKS endpoints and user login are configuration and deployment work; no production account/provider was contacted for these tests.

## Reproduction

```sh
cargo test -p eldiron-network --lib --offline
cargo test -p rusterix --no-default-features --lib server::region::requests::tests --offline
cargo test -p eldiron-server --lib --test account_security --test transport --offline
cargo test -p eldiron-runtime --test prediction --test project_mode --test replication --offline
cargo test -p eldiron-shared --no-default-features --lib public_client --offline
cargo test -p eldiron-player --lib account_settings_change_password_ --offline -- --ignored --test-threads=1
ELDIRON_IDENTITY_TEST=1 node tools/verify_public_client_browser.mjs WEB_DIR MATCHING_PRIVATE_PROJECT
```

The browser runner expects an exported public game with the matching loopback endpoint and a packaged WebGPU Player directory, as described in the existing public-export workflow. Its temporary private identity configuration, key, save, Chrome profile and services are test-owned and cleaned up. The test provider helper is not a production authentication server.

The contracts and settings are documented in [gameplay parity](../../docs/network-gameplay-parity.md) and [external login](../../docs/external-login.md). These checks do not claim unrestricted engine-action APIs, implicit player-account merging, SMTP delivery, a hosting control plane or MMO-scale capacity.
