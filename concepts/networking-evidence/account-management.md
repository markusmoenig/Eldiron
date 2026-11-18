# Account management validation — 2026-11-18

This phase adds persistent-world password changes, global logout and private offline recovery. WSS/subprotocol v4 and save containers remain unchanged. Public clients receive only the account-management capability and sanitized result codes; credentials/account directories stay private.

## Focused checks

- Network library: 20 tests pass, including masked-secret Debug, message roundtrips, password-size bounds and a legacy Welcome without the capability.
- Credential test: salted Argon2id/canonical names, legacy account records defaulting to security revision zero, and revision changes retaining the UUID.
- Dedicated account tests: real loopback WebSockets reject unauthenticated and incorrect-current-password updates, retain the session after a wrong password, commit a correct replacement, deliver success before closure, reject old passwords/resume tokens and accept the new password under the same UUID.
- Serialized-owner race tests reject login/password worker results captured before revocation. Global logout invalidates old tokens. Restart retains the new credential revision and the original saved character/creation receipts.
- Offline recovery test invokes the actual `eldiron-world` executable with piped input, verifies a fresh private snapshot and unchanged source bytes, retains the world/body/inventory/action data, accepts the recovered password, and rejects an existing output path. A recovered snapshot also opens under a new world owner.
- Injected storage failures before commit and after durable commit/before acknowledgment leave the old in-memory metadata head unpublished and require reload. Security commits do not allocate creation receipts.
- Native account UI test exercises masked current/new/confirmation inputs, clearing on mismatch/navigation, password change, re-login/rejoin to the same body with exactly one character, and global logout without a local authority.
- Connection regression retains the final account acknowledgment across EOF and a full UI event channel. Earlier ordered replies cannot disappear just because closure arrived before the delivery tick.
- Project-mode runtime tests retain single player and Creator's temporary local override.

## Browser runtime

The public-package WASM/WebGPU Player passes registration, password change, new-password login, authoritative world join, F10 account settings and global logout against a saved local host. The final account state is signed out with the successful acknowledgment displayed. Rendering continues with zero framebuffer readbacks and zero uncaught browser exceptions. The web directory serves only `.eldiron-client`; the private source/save stays with the host. [Machine-readable results](account-management-browser.json) include GPU and transport counters.

The small hand-built JSON fixture originally omitted authoring graphs, which decoding generated with fresh IDs on each process load. Normalizing it through the project archive before export gives stable fingerprints. The initial mismatch was correctly rejected; the test retains all release-identity checks. Browser input also waits for the server handshake and two animation frames around focus/text dispatch.

## Build and renderer compatibility

Native server/Player checks, Player/Creator WASM checks and Creator staticlib checks pass. Six existing host regressions and the real WSS/HTTPS private-replication test also pass. The renderer boundary script passes all declared native, Apple and browser profiles. This does not substitute for a physical Apple-device runtime run.

## Scope

Browser runtime results exercise the public-package WebGPU client against a local saved host. These account tests use explicit loopback WS; previous WSS release/deployment evidence remains applicable to transport. They do not constitute an internet deployment pilot, managed identity provider, email reset flow or hosting capacity guarantee. Operator recovery requires stopping/switching the intended private world owner; resetting a copy cannot revoke a separate process still running the old save.
