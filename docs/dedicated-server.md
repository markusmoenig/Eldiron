# Dedicated worlds and online demos

The dedicated host runs the existing CPU runtime without a window, GPU, audio device, Player, or presentation dependency. Existing single-player launchers and Creator Play remain temporary in-process sessions. Networking is opt-in through the new server and remote client crate.

## Run a disposable online demo

```sh
cargo run -p eldiron-server -- \
  --project test_projects/3DStarter.eldiron --demo \
  --bind 127.0.0.1:8443 --insecure-loopback
cargo run -p eldiron-online -- \
  --server ws://127.0.0.1:8443/play --guest Alice --insecure-loopback
```

Open another terminal with `--guest Bob`. Commands are `w/a/s/d` to walk, `x` to stop, and `q` to log out; press Enter after a command. The native online client exercises the network session; it does not render the GPU game scene.

A demo process never opens a world store or writes an account database. Disconnected guest bodies are stashed out of collision/replication. When the last playing guest leaves, the host creates a fresh authored world, reclaiming all transient actors, starter items and behavior state. A demo incarnation permits at most 128 guest creations before becoming idle/resetting. Demo guests cannot enter a saved host, and saved accounts cannot enter a demo.

## Run a saved host with WSS

```sh
cargo run -p eldiron-server -- \
  --project game.eldiron --state worlds/main.save \
  --bind 0.0.0.0:8443 --tls-cert fullchain.pem --tls-key privkey.pem \
  --origin https://demo.example.com
```

Use a certificate trusted by the intended clients. The server terminates TLS directly. Expose its port as 443 for public browser deployments. Plain WS is available only with the explicit test flag and an actual loopback bind. No forwarding header can enable insecure public hosting. `--demo` and `--state` are mutually exclusive. Pass each permitted browser origin as a separate `--origin`; browsers are denied by default, while native clients without an Origin header are accepted.

Register or log in from the native online client:

```sh
cargo run -p eldiron-online -- --server wss://game.example.com/play \
  --username alice --register --name Alice
cargo run -p eldiron-online -- --server wss://game.example.com/play --username alice
```

For an account connection, the first line of stdin is the password. Input is ordinary stdin and is echoed by an interactive terminal; use a pipe from a password manager when appropriate. Passwords are never command-line arguments. The example client creates a character for a new account or selects its first existing character. The shared protocol also supports explicit listing and selection of other characters. Each account can create eight characters.

Usernames are case-insensitive ASCII letters/digits/underscore/hyphen, 3–32 bytes. Passwords are 12–256 UTF-8 bytes. Local credentials use Argon2id with independent random salts, 19,456 KiB memory, two iterations and one lane. Hash/verify jobs run on a bounded blocking pool outside the simulation owner. Stable account UUIDs own characters; credential verification produces that UUID. Future login providers can resolve to the same UUID without rewriting character ownership. Authenticated password changes, global session revocation and private operator recovery are now supported; see [account management](account-management.md). External providers, email verification and a public administration service remain future work.

Account records and password hashes live only in private `WorldSave.host_data["accounts.v1"]`. New native save files use owner-only Unix permissions. These records never enter replication snapshots or public content; private durable snapshots and backups retain them. Keep the save file and TLS key private. Existing schema-1 saves without host metadata still load. Unowned characters from a prior trusted Dedicated fixture are not automatically claimable by a new account.

Registration commits before authorization. Character creation runs its normal startup callbacks, captures grants and owner association, and atomically saves a request receipt before publishing success or snapshots. Retrying the same creation UUID and parameters returns the original character, including after restart. Reusing that UUID with different parameters is rejected. A storage/settling failure after live mutation ends the host; it cannot checkpoint or publish that uncommitted world. Reopening resolves an ambiguous commit through the atomic checkpoint and receipt. The receipt ledger currently has a fixed 4,096-operation capacity; new registrations/creations are rejected when full. Compaction is future operations work.

The host supplies time in 20 ms movement steps. Ordinary autonomous world changes are checkpointed every five seconds and on Ctrl-C; a crash can lose changes since the last checkpoint. New remote gameplay requests commit their effects and per-character sequence receipts before success or replication, rather than waiting for that timer. Disconnect stops input and releases exclusive control while the saved body remains in the authoritative world. There is no combat-logout/despawn policy yet. A login/resume replaces the account's previous socket. One-hour opaque resume tokens live only in host/client memory, rotate on use, and are revoked on logout; server restart requires password login again.

## WASM connection viewer

```sh
cargo build -p eldiron-network --target wasm32-unknown-unknown
# Use the wasm-bindgen CLI version matching Cargo.lock.
wasm-bindgen target/wasm32-unknown-unknown/debug/eldiron_network.wasm \
  --target web --out-dir clients/online/web/pkg
python3 -m http.server 8080 --directory clients/online/web
```

Allow `--origin http://localhost:8080` on the demo host and open that URL. A public HTTPS page must connect to WSS with a trusted certificate. Local plain HTTP/WS testing is supported by the viewer's explicit loopback option. The viewer loads the Rust WASM module and uses the same codec, private snapshots, prediction, reconciliation, remote interpolation and compiled collision content as the native adapter. Open two windows to test shared actors. WASD controls movement.

This is a basic actor/connection viewer, not the existing game's GPU renderer. The canonical graphical Player now supports project-level multiplayer startup and a built-in account/demo lobby; see [project-play-modes.md](project-play-modes.md). Creator Play stays local. Remote combat, inventory and dialogue now use [explicit gameplay requests](network-gameplay-requests.md). Creator and the CPU CLI now export [public client packages](public-client-export.md) for the canonical desktop/browser/Apple Player. `BrowserSocket` exposes the full typed account protocol for that integration, while `DemoClient` is intentionally accountless. Economic actions and dialogue selection use the bounded request vocabulary; arbitrary engine `EntityAction`s are never deserialized from clients.

## Wire and content boundaries

- WSS/WebSocket over TCP with subprotocol `eldiron.v4`, binary messages, a 12-byte magic/version/direction/length envelope and named MessagePack payloads. Native uses tokio-tungstenite/rustls; WASM uses the browser WebSocket and TLS stack. There is no UDP/WebTransport requirement.
- Client messages: 64 KiB maximum, at most 32 movement frames per batch. Server messages: 4 MiB maximum. Decode rejects wrong directions, versions, trailing bytes, unknown requests, excessive nesting and oversized fields. Credentials/tokens redact Debug output.
- Snapshots remain recipient-projected DTOs. Socket identity determines control; requests cannot supply an account owner, controlled entity, speed, position, teleport or arbitrary host event. Frame generations/sequences and server acknowledgments retain the existing reconciliation rules.
- Physics content is fetched separately at `https://host/content/<sha256>` (HTTP only in explicit loopback tests). It contains public region geometry, compiled `MapMini`, collision volumes/openings/floors and sampled terrain, never authored Maps, scripts, templates, VM globals, arbitrary properties, inventories or account data. Clients verify the digest and content reference before enabling prediction. Terrain transfers retain exact collision grid corners and omit the authored terrain source.
- Baselines and prop changes carry immutable content hints. Content cache is capped at 64 MiB; eviction can require reconnect/resync. Static public artwork and UI are distributed separately as versioned `.eldiron-client` packages. Existing renderer chunk sizes remain unrelated to socket ownership or future interest-management cells.
- The server accepts 64 concurrent sockets, limits accepts/auth attempts per IP, bounds four hash jobs, caps per-socket message rate, requires hello before authentication, expires unauthenticated peers after 15 seconds and idle peers after 45 seconds, and bounds channels and writes. A slow client's queue closes its connection rather than silently dropping a delta. Clients send application Ping periodically.

## Validation

```sh
cargo test -p eldiron-network
cargo test -p eldiron-server
cargo test -p eldiron-runtime
cargo check -p eldiron-network --target wasm32-unknown-unknown
python3 tools/check_renderer_boundaries.py
```

The TLS integration test needs localhost socket access and the `openssl` command. It creates an ephemeral certificate, proves ordinary trust rejects it, then explicitly trusts it to exercise two native WSS clients, private replication, predicted input acknowledgments and HTTPS physics downloads. Host tests cover account isolation, creation retries/restart, resume rotation and disposable-world isolation. Runtime tests cover failed and ambiguously committed live host operations. A terrain regression checks exact facet heights after public-content serialization.

`tools/verify_online_browser.mjs` exercises two actual Chromium tabs running the Rust WASM adapter over WSS and HTTPS. It uses a disposable profile and pins only its temporary certificate. Build the server and WASM module, generate the web bindings into `/tmp/eldiron-online-web/pkg`, then run `node tools/verify_online_browser.mjs`. Override `ELDIRON_WEB`, `ELDIRON_SERVER` or `ELDIRON_CHROME` if needed. The harness removes its temporary keys/profile and stops its server.

Validation for this implementation: 37 runtime tests passed; account hashing and codec tests passed; the dedicated-host test covered ownership, retry/restart, token rotation and demo/save isolation. Real native clients passed WSS account registration, failed-login rejection, private replication, fixed-input acknowledgments and HTTPS content delivery with certificate verification. Two actual Chromium tabs passed the WASM WSS/HTTPS demo harness. Sampled terrain roundtripping preserved collision facets exactly in the terrain regression. Native Player/client/Creator and browser Player/Creator checks passed, as did the independently resolved renderer-boundary checks. Full remote graphical gameplay and Apple-device/Windows/Linux runtime testing are not part of these results.


## Spatial subscriptions and content updates

Dedicated players now receive nearby objects in their authoritative region, with subscription margins and trusted target/region pins. Full-region public geometry and collision load separately, and optional asynchronous preloads do not grant live state. Physics revisions update even for distant doors without resetting movement or dialogue context. Single player and Creator views remain complete. See [spatial-subscriptions-content.md](spatial-subscriptions-content.md) for settings, coordinate conventions, package/cache limits and the managed hosting boundary.

## Recovery and operations

The server now supports configurable player admission, private JSON metrics and periodic verified snapshots. The native `eldiron-world` tool inspects/copies/restores snapshots to fresh paths and permits conservative deployment-only migrations. See [server operations](server-operations.md) for flags, recovery procedure, inactive-region/overload policy and repeatable load measurements. Accountless demos remain disposable; embedded single player, Creator tests and protocol 4 WASM clients keep their existing paths.

Current clients/server use WSS protocol 4 with compact entity poses. See [performance and storage compatibility](network-performance.md) before mixing releases.


Saved dedicated worlds now use a journaled working head plus a private `.parts` directory, with bounded groups of gameplay actions. Use portable `.snapshot` exports for copying/backups. See [working-save and recovery rules](server-operations.md). WebSocket protocol 4, single-player/Creator behavior and browser transport are unchanged.


## Release hardening gate

The repeatable real-network gate now exercises delayed/fragmented WSS, simultaneous ownership requests, lost acknowledgments, forced process restart, slow-peer isolation and cross-world credentials. The dedicated CLI checkpoints on Unix SIGTERM as well as Ctrl-C; socket tasks cannot outlive their owner. Native graphical and WASM input sends are independent of display refresh. See [release hardening](network-release-hardening.md) for commands, scope and remaining coverage.
