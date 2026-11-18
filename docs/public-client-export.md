# Public multiplayer client releases

Creator's **File → Export Public Client…** writes a `.eldiron-client` package for a multiplayer project. Set its WSS endpoint in Project Settings first. Export captures a copy of the committed authoring views; it does not change the project path, mark the document saved, open a save store, or start a world. Keep saving the authored `.eldiron` separately. Single player and Creator Play continue using authored projects and the embedded runtime.

For automated publishing:

```sh
cargo run -p eldiron-online --bin eldiron-client-export -- \
  --project private/game.eldiron \
  --output public/game.eldiron-client \
  --endpoint wss://games.example.com/play \
  --deployment-id my-game-release
```

The source must already select multiplayer. Overrides change only public deployment metadata. CLI output must be a fresh `.eldiron-client` path; a completed, synced temporary file is published without replacing an existing release. The output is decoded and validated before publication. Creator's native/browser/Apple file services support the same export as a separate document transfer. Export pickers require the client extension and never reuse the authored document's file handle.

Run the canonical desktop Player with the package path:

```sh
cargo run -p eldiron-client -- public/game.eldiron-client
```

The desktop, browser and foreign/Apple Player loaders recognize the format by its manifest. Creator and dedicated-server document loaders reject it. A public release cannot start an authoritative session or run Creator local tests; failed online connections stay online and report the error.

## Browser deployment

Build a public Player directory from the private multiplayer source:

```sh
python3 tools/package_web.py player /tmp/my-public-player --offline \
  --project private/game.eldiron \
  --endpoint wss://games.example.com/play \
  --deployment-id my-game-release
```

Use a fresh output directory for each release. The tool exports `game.eldiron-client`, builds the WebGPU Player and CPU bake codec worker, and selects that package in `index.html`. Alternatively, package the engine without `--project`, copy an exported client package into its public directory, and open `index.html?project=game.eldiron-client`. Serve over HTTPS and allow that page's origin on the dedicated host. Explicit HTTP/WS loopback testing remains supported. The account/demo lobby follows the server's admission mode; the same package works with disposable demo or persistent account hosts whose content matches.

Only put the browser engine, worker and public client package in the web root. Keep the authored source, world saves and their `.parts` directories, accounts, backups, metrics and TLS private keys outside it. The browser packaging command does not copy these files. Use an otherwise empty output directory; it does not audit unrelated files already present there.

## Incremental delivery

Use a streamed release when regional geometry, paint or bakes should load only on entry:

```sh
cargo run -p eldiron-online --bin eldiron-client-export -- \
  --project private/game.eldiron --streamed-dir public/release-001

python3 tools/package_web.py player public/release-002 --offline \
  --project private/game.eldiron --streamed
```

Both destinations must be fresh directories. The CLI writes `packs/<sha256>.eldiron-pack` files, syncs them, and atomically publishes `game.eldiron-client` last. Publish the whole directory under an immutable release URL. Global pack hashes are independent of endpoint/deployment overrides, allowing unchanged bytes to be reused by a publisher/CDN. Serve pack URLs with `Cache-Control: public, max-age=31536000, immutable`. The client uses ordinary browser HTTP caching and verifies bytes on every load; it does not create an application-wide persistent cache.

Version-two bootstraps carry the public UI/configuration, template metadata, palette/rules metadata and a region catalog. Global artwork, fonts/audio, icons, avatars and prefab assets are partitioned into packs near 4 MiB decoded each. These global packs are required before the lobby; the browser fetches at most four concurrently and reports progress. Region geometry, paint and bakes are fetched only when authoritative content requires that map. Server optional physics preloads do not force regional artwork downloads. The ordered gameplay lane installs the verified artwork before the matching geometry/collision and frame. A missing, damaged or timed-out pack reports a connection/load error, without starting local authority.

Pack locations are derived from the bootstrap location plus their lowercase SHA-256 names. Manifests cannot supply arbitrary URLs or filesystem paths. Assets use HTTPS, native local release directories, or explicit loopback HTTP; redirects and browser credentials are disabled for pack requests. Each pack checks encoded size, decoded size, checksum, depth, kind and region identity. Required region jobs cancel with their owning connection.

Bounds: bootstrap 4 MiB decoded/encoded; at most 1,024 pack descriptors; each encoded/decoded pack at most 16 MiB; aggregate declared global data at most 128 MiB and regional data at most 128 MiB. Loaded regional art is retained for the release lifetime within that aggregate budget, so revisits within a connection require no extra artwork fetch. These limits bound decoded source payloads, not total renderer/GPU memory. Larger individual media/regions require content partitioning; single-texture/audio streaming and global-artwork-on-demand are future work.

Canonical desktop and browser Players load streamed releases. Creator's File export and opaque-byte foreign/Apple document loading retain the single-file format; foreign hosts can assemble streamed globals with the shared API and supply their release origin when adding such a host integration. Single-player source loading and Creator Play are unchanged. Protocol 4 and durable save formats are unchanged; readers retain version-one single-file compatibility.

## Published content and privacy

Both bootstrap and single-file ZIPs have exactly two entries:

- `manifest.json`: format `eldiron.client`, version 1 (single file) or 2 (streamed), exact engine version, network protocol, original canonical server content revision and SHA-256 of the payload.
- `client.msgpack`: named MessagePack with an explicit public presentation/visual schema.

The package contains static public geometry, screen layouts and declarative UI, selected rendering configuration, public UI rules metadata, locales, audio FX and font/audio/image assets. It freezes visual tiles, materials, animations, icons, bitmap avatars, palette values and procedural appearance/attachment recipes. It includes paint and embedded orthographic imagery. Prefabs carry sanitized geometry and rendering/door metadata. Visual recipes are public artwork descriptions; clients need them to render variable character appearances.

It excludes world/region/character/item behavior source and debug source, executable tile modules, behavior graphs, authoring modules, authored actor/item instances and inventories, private configuration tables, gameplay rule results and unrelated rule domains. New authoring fields are excluded unless explicitly added to the public projection. UI text, art, names, public requirements and visual recipes are inspectable; this format is not encryption or DRM. Do not put secrets in public UI, artwork or locale strings.

The single-file package includes all static region layouts and visuals; streamed releases defer region packs until entry. Neither format is a hidden-map/undiscovered-content protection mechanism: pack descriptors and public assets are inspectable. Spatial subscriptions still determine which live objects and private gameplay state each connection receives. Runtime layout/collision updates continue through immutable, digest-verified `/content/<sha256>` packages; visual assets added to a project require a matching client release.

## Compatibility and hosting

The manifest retains the **server's original content identity**, calculated before stripping private fields or preparing derived visuals. The remote Player sends that identity rather than hashing the stripped presentation. Endpoints, deployment IDs and project mode do not change this canonical gameplay identity; authored content changes do. The SHA-256 checks integrity, not publisher authenticity: deliver packages through a trusted HTTPS origin. No account credentials or resume tokens are exported.

Both the archive and decoded payload are capped at 128 MiB. The reader bounds entry sizes and MessagePack depth, rejects unknown/duplicate ZIP entries, verifies engine/protocol/version and checksum, and validates geometry, identities, textures and public-field policies. These are the single-file format limits. Streamed version-two releases use the separate bootstrap/pack budgets above. Runtime region content caches keep their separate existing limits.

Managed hosting can build this artifact from a private source release, bind public endpoint/deployment metadata, and serve it immutably under a versioned URL. The authoritative host retains the matching private source and storage. This implements the artifact boundary; it does not provision hosts, resolve deployment IDs, sign releases, bill customers or supply a tenant control plane.

## Verification

Run package privacy/integrity and metadata tests without graphical dependencies:

```sh
cargo test -p eldiron-shared --no-default-features --lib public_client --offline
cargo test -p eldiron-creator --lib public_export_completion --offline
cargo test -p eldiron-player --lib public_client_package_ --offline -- --ignored --test-threads=1
```

The ignored Player test needs loopback sockets. It exports the 3D starter, verifies the host identity, loads frozen visuals and the action catalog, joins the authoritative demo and checks starter equipment and remote-only execution. Browser/OS runtime coverage remains distinct from native and WASM build checks.

Recorded acceptance results and the full browser harness are documented in [public export evidence](../concepts/networking-evidence/public-client-export.md).

Streamed-release verification is recorded in [incremental asset evidence](../concepts/networking-evidence/incremental-assets.md).
