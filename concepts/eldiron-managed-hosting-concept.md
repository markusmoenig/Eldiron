# Eldiron managed multiplayer hosting

Status: concept for later implementation. The current project-mode work must leave room for this service without requiring it.

## Product

Offer Eldiron creators a managed way to publish multiplayer games: choose multiplayer in the project, publish a deployment, and share a playable link. Eldiron operates the authoritative server, secure endpoint, durable storage and backups. Creators can also supply a self-hosted endpoint.

The project owns the single-player versus multiplayer choice. Hosting is a deployment concern for multiplayer projects; players do not choose the game's execution mode. Demo versus persistent hosting is a server admission/storage policy, not another project gameplay mode.

## Initial offerings

- Free or limited demos: accountless guests, disposable worlds, capacity/time limits and idle shutdown/reset.
- Paid persistent worlds: accounts, saved characters/world state, reserved resources, backups and restore controls.
- Larger plans: greater measured capacity, storage, regional placement and operational support.

Do not set prices or advertise player limits before measuring representative games. NPCs, scripts, region activity, bandwidth, storage and uptime affect cost as well as concurrent players. Avoid unlimited-resource promises. Validate creator demand and willingness to pay through a small pilot.

## Creator workflow

1. Set the project mode to multiplayer.
2. Select a self-hosted endpoint or create an Eldiron-hosted deployment.
3. Publish a versioned client/server content release.
4. Receive an HTTPS game link and WSS gameplay endpoint.
5. Manage releases, logs, capacity, player access, backups and restores from a dashboard.

Creator Play remains a temporary local test by default, including for multiplayer projects. Remote testing is an explicit operation and must not alter a production save accidentally.

## Architecture

Start with one isolated process/container per hosted world. Several bounded small worlds can share a machine; accounts, saves, secrets and runtime resources remain isolated between tenants. Game scripts require execution/resource limits and a process/container boundary; engine-level player connection handles are not tenant isolation.

A control plane owns organizations, projects, deployment IDs, versions, billing and lifecycle. A world host owns simulation, admission, character ownership, replication and durable state. Do not place billing/cloud credentials or deployment administration into the gameplay protocol.

Projects carry public connection configuration. A provider-neutral deployment identity may be retained alongside the resolved endpoint, allowing later managed publishing to replace endpoints without changing gameplay or the renderer. Credentials and resume tokens remain outside project files and URLs. A stable deployment ID, content revision and process incarnation are distinct identities.

Use WSS for gameplay and HTTPS for public versioned client content. Validate protocol/content compatibility, keep server source/private state out of client packages, and support controlled release/rollback. Existing authored `.eldiron` projects are development documents, not a finished public-client export format.

Persistent accounts resolve to stable account UUIDs; additional login providers can attach later. Keep gameplay account identities distinct from creator/organization billing accounts. Namespace player accounts and saves per game/deployment unless an explicit identity-linking design is introduced.

## Operations required before paid hosting

- Automated provisioning, TLS, DNS/routing, health checks and safe shutdown/restart.
- Enforced CPU, memory, script-execution, storage, bandwidth and connection quotas.
- Durable storage ownership, backups, restore drills, version compatibility and migrations.
- Monitoring for simulation latency, save failures, resource pressure and slow clients.
- Authentication rate limits, tenant access controls, secret handling and abuse management.
- Billing, usage reporting, support and documented retention/deletion policies.
- Pilot measurements of busy regions, reconnects, failure recovery and operating cost.

The current CPU-only dedicated host, WSS/native/WASM adapters, durable world records, private replication and disposable demo mode are foundations. They do not yet constitute a production multi-tenant hosting platform.

## Milestones

1. Finish project-level modes and reliable local/remote player startup.
2. Finish public graphical content export and remote interaction/UI integration.
3. Run a small manually operated hosting pilot with isolated worlds and tested backups.
4. Add managed publishing, deployment lifecycle and creator dashboard.
5. Introduce paid plans based on measured cost and supported capacity.
6. Add regional placement and more advanced scaling when real usage justifies them.


## Gameplay foundation update

Combat, inventory and dialogue use bounded authoritative requests with stable per-character action streams. A deployment's world and its account directory remain host-owned; socket identity cannot select a tenant, account owner, callback or item price. Saved effects and sequence receipts commit together; disposable demo guests use no store. These boundaries can support managed world namespaces and additional login providers without exposing billing or control-plane credentials in game projects. The current complete-file checkpoint backend is for small worlds; managed scale requires measured capacity and a transactional persistence backend before offering throughput guarantees. See [the gameplay request contract](../docs/network-gameplay-requests.md).


## Spatial/content foundation update

The dedicated host controls interest centers, budgets and trusted spectator/target subscriptions. Clients cannot choose tenants, arbitrary region lists or asset URLs. Public geometry/physics packages are immutable HTTPS resources; optional prefetch is bounded and does not expose live occupants or account state. A world owner shares published packages across its players, while per-connection download caches are disposable. Managed policy can use the trusted runtime subscription API without changing game ownership or the public action vocabulary. Disk/CDN publication, partial streaming and measured capacity remain operations work. See [the spatial/content contract](../docs/spatial-subscriptions-content.md).

## Recovery/operations foundation (phase 8)

The dedicated process now provides configurable admission, bounded per-client overload handling, private JSON CPU/queue/save/transport metrics, periodic immutable snapshots with verified retention, and fresh-path restore/deployment-only migration tools. World/account/character ownership and retry watermarks survive together. These are per-world building blocks for the future control plane: no public administrator API, fleet orchestration, billing, script sandbox or paid capacity guarantee is implemented. Each hosted world still needs an isolated owner process, protected storage and external resource enforcement. See [operations and recovery](../docs/server-operations.md) and its measurement evidence before selecting any hosting tier.

## Storage and replication optimization follow-up

The authority now shares immutable public projection work and sends compact motion deltas, while private grants/revocations remain complete recipient-specific replacements. Checked save encodings can be reused by storage adapters, native heads avoid redundant decoding under their exclusive lock, and the raw-payload save container remains readable alongside earlier snapshots. These reduce per-world CPU/serialization work without weakening committed action receipts. See [the contract and measured comparison](../docs/network-performance.md). Future managed storage/group commit must keep those durability and isolation guarantees; it is not implemented as a public database or hosting service here.


## Release hardening foundation

The per-world owner now checkpoints on Unix SIGTERM, owns/cancels all socket workers and reports a drained active-connection count on graceful exit. A real WSS release gate tests forced restarts, committed receipt retries, bounded slow-peer failure and separate-world token/account isolation, including worlds with identical game content. High-refresh native/WASM clients no longer tie send rate to redraw rate. These improve supervisor integration and tenant boundaries without adding any billing, fleet, public admin or cross-world identity service. See [release verification and limits](../docs/network-release-hardening.md); a representative long-running deployment pilot and external resource enforcement remain necessary before paid hosting.


## Client loading lifetime and bounded catch-up

Required public content downloads no longer block client socket receipt or heartbeats. Each connection owns bounded ordered gameplay staging, its fetch jobs and cache; closing/reloading the session cancels that work. UI backpressure retains the next event without granting local authority or bypassing private replication. Slow or failed content produces an explicit reconnect/resync condition rather than unbounded retained frames. These limits support isolated deployments but do not replace host/CDN bandwidth quotas, artifact publication or deployment resource enforcement. See [content delivery](../docs/client-content-delivery.md).


## Deployment-drill foundation

Longer local tests now exercise separate saved-world owners through bounded TLS relays, changing network conditions, outages, process restart/kill and verified backup restoration while another world continues. The test runner owns its temporary process group; the browser runner also verifies interruption cleanup. Queue, artifact-retention and per-generation memory checks provide operational evidence for a future pilot without establishing a plan size or concurrency guarantee. Browser probes include progress, prediction/reconciliation and memory diagnostics rather than treating a stale "Playing" label as health.

Keep one private working-save/parts/lock/backup/metrics namespace per world. Restore stops the old owner first and creates a fresh working path. The current demo reclaims all transient VM state when its world becomes empty; sustained demo visitor churn with an always-present player still needs a representative lifetime/quota policy before public managed hosting. Process memory/script limits, off-site storage, supervisor behavior on target hosts, real DNS/TLS renewal and fleet control remain required. See [the local deployment procedure](../docs/network-deployment-testing.md).

## Public release artifact foundation, 2026-11-18

A managed publisher can now export a multiplayer source to a `.eldiron-client` artifact using a CPU CLI or Creator, bind the deployment's public WSS endpoint/identity, and build the browser Player directory. The versioned manifest carries the authoritative content fingerprint plus payload integrity digest. The public schema includes renderer/UI assets and visual recipes while excluding behavior source, authoring graphs, instances, private configuration and saves. Native/WASM/Apple Players stay remote-only when loading this artifact; Creator testing still uses the private source locally.

Host the engine, codec worker and public package on an immutable release URL/CDN. Keep source projects, world/account stores, backups, metrics and private keys under the isolated world owner. This supplies the basic artifact contract for future managed publishing. Provisioning, release signing, rollback orchestration, deployment-ID resolution, billing, quotas and incremental asset delivery still belong to the future control plane. See [the export workflow](../docs/public-client-export.md).

## Incremental release delivery, 2026-11-18

Publishers can now emit format-two public bootstraps and immutable `packs/<sha256>.eldiron-pack` files. Public endpoint/deployment metadata remains separate from canonical gameplay identity. Required global art is bounded and batched; regional art follows authoritative entry without changing ownership, subscriptions or WSS protocol. A CDN can reuse unchanged hashed files with immutable cache headers; clients verify original bytes and decoded bounds before installation. The CPU CLI syncs packs and publishes the bootstrap last into a fresh directory. Source, accounts, saves and keys remain private.

This adds the incremental static-art delivery foundation to the hosting design. It does not create a publisher service, global login namespace, asset authorization system, runtime arbitrary-URL resolver or billing/control-plane API. Current source-payload budgets are explicit; installed regional art is retained within that budget and global art remains required at startup. See [the implemented contract](../docs/public-client-export.md#incremental-delivery).


## Account security foundation, 2026-11-18

Each persistent world now owns durable account security revisions. Password changes and global logout commit before revoking account tokens/sockets; stale authentication workers cannot publish authorization under an older revision. Additional providers can retain the same world-scoped account UUID and character ownership. No global account directory or administrator secret enters the public client artifact.

Operator password recovery is an offline fresh-snapshot operation for a stopped world owner, with private piped input, matching project validation and preserved world/account/character identities. A future control plane may wrap it in its own support authorization and deployment switch procedure. Public email recovery, cross-world identity, provider linking, billing and fleet orchestration are still future services. See [the implemented contract](../docs/account-management.md).
