# Project-level single player and multiplayer

The project chooses its execution mode. Players do not choose single/multiplayer from a game-mode menu.

In Creator, open **Project Settings → Project Execution → Game mode**. The dropdown edits the project's TOML and uses the existing undo/save system. Old projects without `game.mode` remain single player. New project defaults explicitly select single player.

```toml
[game]
mode = "single_player"
```

An online project uses:

```toml
[game]
mode = "multiplayer"

[multiplayer]
endpoint = "wss://game.example.com/play"
deployment_id = ""               # Optional public publishing identity.
allow_insecure_loopback = false
```

For a local networking test, explicitly enable loopback WS on both client and server:

```toml
[multiplayer]
endpoint = "ws://127.0.0.1:8443/play"
allow_insecure_loopback = true
```

Public connections require WSS. Endpoints cannot contain credentials, queries or fragments, and currently use the server's `/play` route. No passwords, bearer tokens, cloud credentials or billing data belong in project files. Missing/invalid multiplayer endpoints fail before a local world can start.

## Startup behavior

- Single player follows the existing embedded `GameSession` path, including configured start screens, automatic registration and `game.start`/class commands.
- Multiplayer prepares client display content without creating region instances, starting world/entity scripts or granting starter possessions. A built-in GPU lobby connects through the native/browser WebSocket adapter. The configured play screen and game renderer consume the remote private replica and predicted presentation after joining.
- The server's Welcome selects demo or persistent admission. Demo hosts show a name/Play flow. Persistent hosts accept username/password registration or login, return the account's characters, and allow Play, Next character or New character. Password fields are masked and cleared after submission; credentials and resume tokens are not saved in the project.
- Reconnect uses the in-memory rotated resume token, re-lists owned characters and permits rejoining the same saved body. Explicit logout clears the token and sends Logout. Disconnecting a transport releases control without automatically logging out/revoking resume.
- The local input/session clock cannot advance an online world. Mouse yaw and fixed movement frames use prediction, reconciliation and remote interpolation. The server supplies authoritative time, ownership and collision.

The built-in online lobby is created in the running player copy; it is not inserted into the authored project or save file. Authored online buttons can use `game.login`, `game.register`, `game.start`, `game.next_character`, `game.create_character`, `game.connect` and `game.logout`. Fields use `online.username`, `online.password` and `start.name`; input widgets can declare `secret = true` to mask text. The initial lobby is automatic for multiplayer projects, rather than depending on a pre-existing authored login screen.

## Creator testing

Creator Play remains an explicit temporary local host for either project mode. Multiplayer documents can be authored and tested before an endpoint exists. The original document keeps `mode = "multiplayer"`; running Play does not switch it to single player or touch its deployed save/account service. CPU hosts can use `PlayerSession::new_local_test` to request this override explicitly. Dedicated servers continue to be explicitly launched authoritative hosts.

The canonical desktop, foreign/Apple and WASM GPU player follows the project setting. The legacy/reference adapters remain compatibility tools. The text terminal client currently rejects multiplayer projects with a direction to use the graphical player, rather than silently running their world locally. The separate native `eldiron-online` connection client remains available for protocol testing.

## Content and hosting boundaries

Public multiplayer distributions now use a matching [`.eldiron-client` release](public-client-export.md), exported from Creator or the CPU CLI. Canonical desktop, browser and foreign/Apple Players accept the public package; authored `.eldiron` loading remains available for development and single player. Gameplay packets never transfer the authoring document, scripts or account data. Public region geometry and server-compiled physics continue over HTTPS and install before the associated frame. Static visuals, prefab materials and UI come from the public release. See [spatial-subscriptions-content.md](spatial-subscriptions-content.md) for subscriptions, preloads and cache behavior.

The connection fingerprint normalizes the ruleset selection and ignores project mode and `[multiplayer]` deployment metadata. A self-hosted versus managed export can resolve a different public endpoint without changing its gameplay identity. Authored gameplay changes still fail compatibility. Durable saves retain their existing stricter project/schema fingerprint; local characters and online characters are never automatically merged.

`deployment_id` is metadata for a future publisher/control plane. It does not yet provision or resolve an Eldiron-hosted deployment. See [the managed hosting concept](../concepts/eldiron-managed-hosting-concept.md). Hosting administration remains outside gameplay networking.

## Current limits

The graphical connection path supports account/session management, predicted movement and explicit validated combat, inventory and dialogue requests. See [network-gameplay-requests.md](network-gameplay-requests.md) for permissions, durable retries and supported actions. Arbitrary engine actions and raw text commands remain outside the wire vocabulary. Server-compiled physics is authoritative, but not every private dynamic visual state has a public rendering DTO yet. Network errors return to the connection screen; there is no fallback to a hidden local world.

This phase preserves the local renderer/UI behavior and adds project-driven online startup. It does not claim feature-complete multiplayer interactions, automatic hosting, or automatic hosted publishing.

## Checks

- Shared configuration tests cover legacy defaults, strict mode/field types, endpoint restrictions and Creator overrides.
- Runtime tests prove a multiplayer client creates no local region instances, does not advance local ticks or expose authored occupants, and that Creator can run the same document locally without an endpoint.
- Project settings tests cover comment/deployment preservation and serialized project roundtrips.
- Native connection tests exercise the graphical player's lobby, demo join, account creation, movement acknowledgments and reconnect to the same saved body. They require localhost socket access and run with `--ignored` filtered to `multiplayer_project_`.
- Native/WASM builds and the renderer boundary script cover the shared mode and transport paths. Browser runtime/GPU acceptance is distinct from a successful WASM compile.

## Account security controls

Persistent servers advertise account-management support. The graphical lobby's Account button and in-game F10 open masked password-change and global-logout controls. Authored commands use `game.account`, `game.account_back`, `game.change_password` and `game.logout_all`. Successful changes sign out all sessions; saved account/character ownership remains stable. Old servers and demos keep the existing connection flow. See [account management and private recovery](account-management.md).
