# Combat, inventory, dialogue and resource requests

The graphical Player routes multiplayer gameplay through the same native/browser WSS transport as movement. Single player and Creator Play keep using trusted embedded engine actions; they do not need accounts, a socket or a durable store. Multiplayer never starts a local simulation to perform these actions.

Protocol **4**, WebSocket subprotocol **`eldiron.v4`**, uses the existing bounded binary MessagePack envelope over WSS/TCP. Upgrade clients and server together. Movement retains prediction, reconciliation and remote interpolation. Combat, item transfers and dialogue are authoritative: input and UI selection respond locally, but damage, costs, ownership, rewards and dialogue transitions appear through server replication.

## Public vocabulary

`ClientMessage::Action(GameAction)` contains the server-issued character stream, monotonically increasing action sequence, world incarnation, replica generation, region, and one bounded `GameRequest`:

| Request | Authoritative checks and behavior |
| --- | --- |
| Intent; entity/item interaction | Built-in or authored ruleset intent/action; target existence/visibility; actual distance; shared gameplay eligibility, costs, cooldowns and target policy. Learned spell invocations use declared ruleset actions, not the legacy raw `spell:` shortcut. |
| Prop interaction; targeted terrain intent | Existing authored part/target; server anchor/range; bounded coordinates and declared intent. No client-supplied event, movement speed or teleport. Empty-intent terrain clicks use project-enabled 2D click-to-walk with server-owned routing and fixed time. |
| Resource | `ResourceRequest` names an access UUID, interaction, bounded quantity and optional exact owned tool/vessel runtime IDs. The server resolves the source, range, requirements, stock, outputs and costs. Clients cannot name an actor, create outputs, set stock or advance time. |
| Open container | Own container, nearby world container or nearby corpse container. A live other player's inventory is inaccessible. |
| Transfer | Inventory/equipment, pickup/drop, root container contents and container-to-container transfers. Actor comes from the connection; destinations in another player are rejected. Explicit NPC-follower addresses require current server-assigned membership and a living non-player companion. Nonowned container contents require the currently opened server container session, with reach checked again. Normal stack, equipment and capacity rules still apply. |
| Choose | An opaque offer UUID issued privately to this socket. The server looks up the exact original choice and rechecks recipient, active dialogue session, expiry and range. The client cannot choose a callback, item price, reward or arbitrary choice index. |
| Command slot | Authored slot and command validation through the existing ruleset. |
| Camera | Current camera or a project-allowed camera; changed movement parameters require a new baseline. |
| Inspect self; craft | Fixed inventory/stats/look queries, or an existing bounded recipe ID executed by the shared crafting handler. |

IDs refer to the current authoritative region. Requests never name the controlled actor or account. Every request requires a live controlled player and the current world/generation/region. Server-assigned living NPC companions can supply inventory/container sources and inventory/equipment destinations; membership is checked again by remote action handlers. Other players remain protected even if party metadata is present. Arbitrary engine actions, raw text execution, free-form chat and admin events are not remote APIs. GPU command input resolves visible names and public rule metadata to these typed requests before sending. A project-specific interaction intent must be declared in the ruleset or actor intent configuration.

`Processed` means validation passed and the normal handler consumed the request. It does not promise that an effect occurred: ordinary capacity, eligibility and cooldown failures still use the engine's private warning messages. Envelope denials use a bounded `RequestError`; clients display a local warning. Authoritative private frames carry resulting inventory, wallet, equipment, combat and dialogue state.

## Retry and durability contract

On character activation the host sends `ActionStream { stream, next_sequence }`. A saved character's stream is its stable character UUID, independent of account login method or socket. A disposable guest uses its connection UUID and an in-memory journal. The client permits one action in flight and 64 unsent actions, retries the exact envelope after 750 ms, and backs off on rate limiting. A generation/region change discards unsent actions and offers; an unresolved in-flight identity remains until acknowledged or resolved by the server watermark. It is never reused for a new command. Offers arriving before a new context's frame cannot enable action input until that frame is applied.

The host accepts the exact next sequence, returns a cached result for an identical recent retry, rejects altered payloads for that sequence and rejects gaps. Each stream retains the last 128 results plus a durable high-water mark. Older compacted requests return `AlreadyCommitted`; compaction can never make them executable again. Watermarks survive restart and are retained if a character record is removed. The 4,096-entry account/character-creation ledger is separate from this bounded gameplay journal.

For saved worlds the exclusive simulation owner validates and executes, settles immediate engine work, captures effects and pending behavior, and atomically commits the world and receipt **before** replying or publishing frames. A failure after mutation poisons the owner and ends the host; it cannot publish or later checkpoint uncommitted effects. Reopen loads either the prior complete checkpoint or the new effects and their receipt. Denied requests also consume a sequence and receive a receipt. No client-provided time advances the world.

The host allows eight new actions per second per connection, in addition to the existing socket/buffer/message limits. Recent receipt retries do not reserve another simulation action. Demo hosts use the same policy and shared handlers but never open a save or account store. Their journals and effects disappear when the demo incarnation resets.

Purchases use staged buyer/seller copies so inventory capacity, partial stack merges, payment and seller credit succeed together. Invalid negative prices cannot transfer items. This shared-handler fix also applies to local play.

Equipment swaps validate the reverse leg too: an inventory item displaced into an equipment slot must satisfy that slot and class policy before either item moves.

## Equipment maintenance

Owned-item requests also execute declared repair actions. For example, an
`Item` request with the exact owned item ID and `action:repair_bone_armor` uses
the existing handler and sequencing contract. No new request variant or wire
version is needed. The server rechecks ownership, family, condition, materials,
usable tool and deployed station, and stages all costs with the condition change.
Repair preserves runtime/template identity, quality and other item properties.
Live foreign inventories and corpse-address impersonation cannot supply repair
targets. Successful retries return the original receipt without another repair.

Passive item bonuses are calculated from committed equipment and attachment
state, rather than stored actor totals. The same item and attachment identities
survive ordinary ownership transfers, repair, recipient-filtered item updates,
reconnects and durable restoration. Existing item attributes carry the typed
attachment state; no request or wire-version change is needed. A new owner gains
the wearer contribution only after equipping the item. Replaying a committed
equip or repair cannot add another source.

`Transfer` to `Inventory(None)` resolves an equipped source to a concrete free
slot. A full inventory rejects that destination without removing equipment.
This differs from the legacy profile-drop action, which intentionally leaves
already-owned items in place.

## World resource identity and commitment

Resource profiles and shared pools use stable ruleset definition identifiers. Independent map sources derive their identity from map and placement UUIDs; an explicit logical instance UUID can group access parts. Pools share one owner across regions of the same world. These identities are separate from artwork UUIDs, item-template UUIDs and runtime item IDs. Two placements using the same tree image do not share stock implicitly.

`GameRequest::Resource` adapts to the same `EntityAction::ResourceInteract` handler used by trusted local actions. Source settlement and extraction are serialized under the world resource lock. The handler stages inventory, vessel contents, tool condition and skill changes before committing them with the supply debit. Contending actors cannot collect the same remaining batch. Failed extraction changes none of those action effects; elapsed recovery or expiry can still settle.

Saved resource state and the simulation clock travel in the existing world checkpoint. The clock advances once per running fixed simulation step, independently of render polling and player count, with no wall-clock/offline catch-up. Initial-stock and display-name edits preserve live stock; incompatible policy edits require migration. No separate resource persistence backend is introduced.

Player resource requests use the sequencing, durable receipts and retry rules above. An identical committed retry cannot extract twice or charge wear twice. The direct trusted NPC API shares atomic execution; it does not introduce a universal journal for arbitrary scripted operations or a mandatory NPC economy.

Public baselines include `resource_views`; deltas may replace that collection. Views expose access/source identity, profile, phase, appearance, supplies, next recovery, revision and optional tile mapping. The private world-resource ledger is not replicated. Clients apply these views to map parts and remains; optional authored tile changes do not replace meshes or collision. A `Processed` receipt retains the meaning above, including engine warnings when extraction cannot succeed.

## Dialogue lifetime and recovery

`Offers` is sent only to the owning recipient, before its associated frame. A new dialogue group replaces that NPC's prior offers; expired or consumed sessions are removed. Committed action output and replacement offers are flushed before processing the next gameplay request, including groups created within the same simulation tick. Cancel uses an issued offer too. A reconnect, baseline/context replacement or server restart invalidates socket-local offer UUIDs; reopen the interaction to obtain new offers. Restored engine dialogues and container UI are dismissed, while committed rewards and choice effects remain saved. A retried completed choice returns its receipt without running its callback again.

## Hosting boundary and remaining work

There is one world owner per host. The gameplay API knows character streams and authoritative requests, not billing, deployment credentials, tenant routing or a cloud provider. A future managed deployment can resolve its public endpoint and select a world/account namespace without changing game actions or login ownership. See [the hosting concept](../concepts/eldiron-managed-hosting-concept.md).

The native file backend uses bounded incremental journals and action groups, with coherent checkpoints and committed receipt watermarks. See [storage/replication performance](network-performance.md) for measured limits; this is not a capacity guarantee for arbitrary games. Spatial subscriptions and full-region geometry/collision packages are implemented; see [spatial-subscriptions-content.md](spatial-subscriptions-content.md). Public client export and incremental region asset delivery are implemented. Streaming individual chunks within a region and managed provider operations remain separate extensions. Renderer chunks do not determine permissions or action routing.

The WASM connection viewer exposes `DemoClient.request(JSON.stringify(request))` using the same Rust DTO/sequencer. Its `state().action` reports the last action result; the full GPU Player uses its existing combat/inventory/dialogue UI instead.

## Verification

- Resource tests cover independent/shared sources, recovery and expiry, vessel identity, atomic extraction, save/reload and dedicated-host contention/retries. See [local resource tests](../crates/runtime/tests/world_resources.rs) and [dedicated resource tests](../servers/dedicated/tests/world_resources.rs).
- Engine request tests exercise authoritative HP damage, costs, learned-action eligibility and cooldowns; contested pickup; equipment and swap policy; container access/range; dialogue recipient/expiry/reuse; purchase rollback and payment.
- Runtime/network tests exercise connection/world/generation/region/dead-actor checks, bounded decoding and queues, frame/offer ordering, retries and reconnect watermarks. Save tests inject failure before and after commit, reopen, and retry after receipt compaction without a second item/payment.
- Dedicated-host tests cover private opaque offers, wrong-recipient requests, consumed/replaced choices within one tick and saved pickup retries across restart. Real native WSS clients verify sequence replies and certificate-checked HTTPS content.
- Graphical Player socket tests exercise intent selection, unequip/re-equip, movement and account reconnect without a local world. Local Player/runtime tests, native/WASM Player and Creator builds, and renderer-boundary checks preserve the embedded and browser paths.
- `tools/verify_online_browser.mjs` runs two Chromium tabs using the actual Rust WASM adapter, pinned temporary WSS/HTTPS certificates, private shared actors, accepted/denied sequenced requests and fixed movement. This is a connection-viewer check; it does not claim device coverage for every native platform or a full browser GPU gameplay acceptance test.

## Gameplay parity extension

The default-false Welcome capability `gameplay_extensions` gates camera/self-inspection/crafting, empty-intent walking and NPC-companion inventory requests on upgraded servers. Endpoint metadata remains separate from canonical content identity, while camera permissions and logout policy are included. See [the gameplay, visual and combat-logout contract](network-gameplay-parity.md).

## Persistent recipe knowledge

Learning uses existing entity/item action requests: `learn_boneworking` targets
an available teacher and `study_boneworking` targets the exact owned manual.
There is no client-supplied acquisition or knowledge-edit request. The host
checks source, skill, costs and the current permanent journal before committing.
Crafting checks current recipe access again before spending ingredients or wear.

The versioned `knowledge_journal` travels in ordinary saved and owner-replicated
entity attributes. Acquisitions have their own UUIDs and stable knowledge IDs;
teacher/book runtime IDs and creator UUIDs record historical provenance. Moving
or deleting a source does not revoke learned knowledge. Temporary equipment
grants remain derived from committed equipment state and never write acquisitions.

Committed player retries reuse existing durable receipts rather than rerunning
learning or batch crafting, including after reconnect/save restoration. The
[focused runtime scenario](../crates/runtime/tests/learning.rs) exercises both
solo and dedicated sessions, owner replication, book transfer, independent
learners, source removal, reconnect and restored receipt replay. No protocol
variant, schema version or networking identity migration is added by learning.

## Targeted gameplay enchanting

`inscribe_bear_cuirass` uses the existing owned-item action request and recipe
transaction. Clients select an exact item address; they do not submit a new
attachment ID, result item or numeric bonus. The host checks current knowledge,
skill, ownership, target eligibility, materials, vessel contents, tool condition
and deployed station access. It commits the attachment, optional authored name,
costs, wear and training together, or none of them.

The existing `strength` effect ID remains stable while its supplied label is
**Bear's Vigor** and its supplied item name is **Bone Cuirass of the Bear**.
An inscribed base cuirass retains its original item/definition identity and gains
one attachment UUID. Ordinary item state/deltas carry this change; repair,
ownership transfer and restoration preserve it without storing wearer bonuses
as actor base stats. Duplicate attachment attempts are rejected without costs.

Player retries use the existing durable receipt path, including after the item
has changed owners or a checkpoint has been restored. The [focused world-chain
test](../crates/runtime/tests/enchanting.rs) covers solo and dedicated sessions,
owner replication, exact item/attachment identity, repair, transfer, reconnect,
save/reload and receipt replay. No new network DTO or persistence schema is
introduced by this operation.

## Crafting and knowledge journal

The journal inspects the owning character and visible stations from the same
replica used for play. It runs the shared immediate recipe planner on isolated
scratch state, never sending a replacement inventory, learned record or preview
attachment to the host. Ordinary entity/self or owned-item actions perform
crafting, repair and enchanting. The host rechecks all requirements at commit;
an earlier ready preview cannot authorize a stale operation.

Public station snapshots additionally expose `station_capabilities` and their
usable condition. Private item properties and container stock/contents remain
filtered. The owning character's knowledge, vessel contents, tools and selected
items already use private owner state. No new request/response DTO, wire version
or durable schema is introduced for the journal.

Pins use the existing `CommandSlot` request and character attributes. A known
craft recipe need not belong to a class's initial action-bar list; pinning still
grants no knowledge or crafting permission. Existing input scopes, item IDs and
creator UUIDs prevent an item replacement or changed baseline from inheriting
a captured operation. Requests and pins retain existing durable retry handling.

The [focused runtime scenario](../crates/presentation/tests/journal_runtime.rs)
covers solo and dedicated owner snapshots, teacher discovery, public station
inspection, stale-preview refusal, pin/craft persistence, reconnect and receipt
replay. Native presentation tests separately exercise selection, exact targets,
pinning and GPU text/source retention.
