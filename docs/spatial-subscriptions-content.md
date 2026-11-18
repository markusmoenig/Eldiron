# Spatial subscriptions and region content

Dedicated worlds now subscribe each player to nearby live objects instead of replicating every occupant of every region. Single player and Creator Play retain their existing complete embedded views. The network still uses WSS/WebSocket over TCP, including browser WASM; the expanded content protocol is **4 / `eldiron.v4`**. Upgrade clients and server together.

## Coordinates and interest

Networking cells are **32 world units**, in the region's horizontal **X/Z** plane (map vertices use X/Y for that plane). Cell coordinates use `floor(position / 32)`, including negative coordinates. They are independent of renderer chunks, collision chunks, editor pixels and viewport zoom.

The default entry window extends 64 world units from the authoritative actor on both axes. Already visible objects remain until their bounds leave an additional 16-unit margin. The per-region index inserts object bounds into every intersected cell, deduplicates query results and checks actual AABB intersection. Very large extents use a bounded overflow list instead of allocating millions of cells. Prop bounds come from resolved world geometry; actor/item bounds include their radius and declared display size. Spatial indexes are rebuilt once per publication and shared across recipients; this does not partition simulation work.

Enter sends a complete object upsert. Leave sends a replica removal. Moving across cells does not change the connection generation, repeat startup or reset movement prediction. A later re-entry sends one complete current object. Nothing in interest tracking removes or edits authoritative or saved entities/items.

The actor, owned followers, active dialogue/trade participants and container targets remain observable as required. Trusted host target pins and full-region subscriptions can support party views or spectators. These never bypass the normal public/private projection: a distant character pin does not reveal its inventory, wallet or private attributes. Interest is a bandwidth policy, not an authorization grant.

Object-addressed speech follows interest. Private replies remain addressed to the player, including across region transfers. Region-wide broadcasts/music follow the current or an explicit whole-region subscription because the existing audio API has no position; a remote party/target pin does not subscribe to its ambient bus. Events from other unsubscribed regions are omitted; stale interactive permissions are not carried across transfers.

## Host settings and APIs

Optional project configuration:

```toml
[multiplayer.spatial]
radius = 64
margin = 16
prefetch_regions = ["Dungeon entrance"]
```

Radius is 4–512 units; margin is 0–128. Up to four prefetch names must resolve to authored regions. Creator/local play does not require these settings or a server. Existing project execution/deployment fields continue to work.

Trusted integrations use `GameSession::set_spatial_subscriptions(connection, Some(SpatialSubscriptions))`. Its region/target pins are bounded; region IDs are validated. `None` restores the complete embedded view. `prefetched_content` returns immutable references for configured preloads. A public client cannot submit a viewing center, region list, radius, spectator target or preload destination. The center always follows server state. These host APIs also let future managed deployments supply policy without changing authored gameplay or touching account ownership. Project edits retain the existing strict durable-save compatibility checks; runtime host policy can be changed separately.

## Separate content loading

Live snapshots contain only subscribed objects and region metadata. Static art, UI, prefab definitions and other rendering assets still belong to the matching packaged client release. A public region package now carries an explicit geometry layout—vertices, linedefs, sectors, meshes, surfaces and nested profiles with an allowlist of rendering properties—plus the server-compiled full-region `MapMini` and `CollisionWorld`. It carries no occupants, inventories, account records, scripts, behavior graphs or arbitrary author metadata. Private strings in nested geometry properties and object names are excluded. Static public artwork and UI are supplied by the separate [public client exporter](public-client-export.md); the region endpoint never serves an authored `.eldiron` document.

Packages are fetched by SHA-256 over HTTPS (`/content/<digest>`), with HTTP restricted to explicit loopback testing. The client verifies original bytes, layout/reference identity and geometry bounds/topology before installation. Region layout and matching collision install before the associated frame is presented. The graphical Player combines the public layout with release-local art, baked imagery and prefab materials; the connection viewer uses the same package verification and CPU collision path.

Full-region collision is available ahead of movement, regardless of which dynamic objects are visible. Static layout/art revisions use `RegionContentRef`; collision mutations have a separate `physics_revision` and immutable package digest. The host checks subscribed regions even when their interested prop delta is empty. Opening a distant door therefore updates collision without resetting the movement generation or an unrelated dialogue. Actual layout/content changes use the existing atomic baseline/reconciliation boundary. Prefab assets, arbitrary generator parameters and new art not present in the client release require a matching exported `.eldiron-client` release.

`Preload` advertises optional packages without granting live state. Native and WASM downloads run separately from the ordered gameplay reader, with at most four pending preload jobs. Slow/failed preloads do not block input, acknowledgments or heartbeats; a required package can be fetched normally later. The cache retains up to four regions and 16 MiB of encoded package payloads, evicts old entries and rejects stale revision replacement. These encoded limits do not include the game's packaged art or GPU resources. Required packages install from cache when available; eviction is a cache miss, never permission to predict without matching collision.

The host memoizes immutable packages across players within one world and caps encoded published content at 64 MiB. Individual packages remain capped at 8 MiB. Evicted HTTP content can require reconnect/resync. There is no disk/CDN cache, download resume, arbitrary URL fetch, partial geometry/collision streaming or cross-process region migration in this phase.

## Verification and hosting boundary

Tests cover negative cells, large extents, enter/leave margins, distant updates/speech, private replies, pins, preloads without live state, transfers, persistent source retention, geometry privacy and cache bounds. Dedicated-host tests cover package revisions and runtime layout/collision updates. Existing movement, reconciliation, accounts, inventory/dialogue and durable-save tests remain applicable. Native/WASM Player and Creator builds, actual WSS clients and the two-browser WASM harness exercise the shared transport/content path.

A managed provider can own each world and apply its subscription/cache budgets while leaving account and deployment administration outside gameplay requests. The current file store and full-region package/index approach are foundations for small hosted worlds. Interest management reduces replication traffic; it does not establish MMO-scale simulation or storage throughput. See [the hosting concept](../concepts/eldiron-managed-hosting-concept.md). Measurements, overload policy and operations remain the next phase.
