# Multiplayer gameplay and visual parity

The graphical Player uses the authoritative request path for player interactions. Single player and Creator Play retain their trusted embedded actions. WSS protocol 4 stays in use; new requests require the server's default-false `gameplay_extensions` Welcome capability. Older servers remain playable without those controls.

## Player controls

Empty-intent terrain clicks now use the shared click-to-walk handler when `[game].auto_walk_2d = true` and the actor uses a supported 2D camera. Clients supply only a bounded destination. The server chooses route, speed, collision and fixed simulation time. Idle movement frames do not cancel an authoritative walk; directional input can cancel it. Scripted walking is presented with authoritative correction smoothing rather than predicting arbitrary script effects.

Camera changes use a typed request and project permissions:

```toml
[multiplayer.gameplay]
allowed_cameras = ["2d", "2d_iso", "firstp_mouse"]
```

Supported names are `2d`, `2d_grid`, `2d_iso`, `2d_iso_grid`, `iso`, `firstp`, `firstp_grid` and `firstp_mouse`. Defaults permit retaining the current camera only. Unknown/duplicate names and malformed tables fail dedicated startup. A camera parameter change uses the existing movement-baseline reset, so old prediction parameters cannot remain authoritative.

The GPU command input resolves text against the recipient's visible replica and public rules metadata. It sends typed entity/item interactions, owned-container transfers, declared spell/action invocations, crafting or self-inspection. Examples include `attack Goblin`, `open Bag`, `take Coin from Bag`, `cast fire bolt on Goblin`, `use heal`, `craft bowl`, `inventory` and `stats`. Ambiguous or hidden names are rejected. The server repeats target, ownership, range, learned-action, cost and cooldown checks. Crafting accepts a bounded declared recipe ID and uses the shared crafting handler; no arbitrary command string crosses the wire.

NPC companion inventory/equipment controls use explicit follower addresses. The authority requires a live NPC with current server-assigned party membership under the controlling leader, in the current region. Permission is checked again when consuming queued remote inventory actions. Membership does not authorize writes to another player's body, even when that player has a party-leader attribute. Existing stack, slot, class, capacity, container and reverse-swap rules still apply. Party membership changes revoke private inventory projection through the existing recipient replacement path.

AI routes, behavior sequences, direct teleports, clocks, arbitrary callbacks, prices and administrator events remain trusted server operations. Free-form chat is not a gameplay command channel. Those operations are intentionally outside the player request vocabulary.

## Public visual state

Public entity/item projection retains renderer inputs for procedural equipment icons and rigs: channels, palette indices, template pixels/dimensions and component colors. Private quest/server attributes remain excluded from other recipients. Full replacements retain attribute removal and permission-revocation behavior.

Public region geometry now carries sanitized map lights and the visual properties needed for recipe particles and campfires. Bounded string-valued rendering modes are retained. Light properties use a separate numeric/vector/bool allowlist; private fields and invalid nonfinite light values cannot enter a valid public package. A new published light collection replaces the old collection, including removal. Packages from before this extension omit that field and retain their release-local lights. Changed region content still installs by its exact authoritative content reference before associated frames.

These are explicit public presentation fields, not a transfer of authored behavior, arbitrary map properties or server assets. New custom visual attributes must be added deliberately to the public policy. Public single-file and streamed exports use the same projection.

## Logout and combat

The default persistent-world policy is `retain_body`: leaving closes control and stops input while the body remains vulnerable in the world. Disposable demos keep their immediate transient retirement/reset behavior.

Projects can instead choose:

```toml
[multiplayer.logout]
policy = "safe_despawn"
grace_seconds = 30
```

The grace is 10–3600 seconds of shared simulation time. A disconnected living character remains present until both departure and last damage dealt/received are outside the grace, and no live actor is targeting it. Attacks and damage can therefore prevent hiding. No packet, render frequency or offline wall time advances this timer. Restart gives present offline characters a fresh departure grace. Safe-despawn grace keeps the region awake even when unoccupied-region suspension is configured, so suspension cannot freeze the combat check. The default retain-body policy continues to respect the project’s existing simulation-suspension policy.

Eligible bodies are stashed with their existing inventory/identity, then checkpointed before disappearance is published. Stashing for logout does not mark a living body dead. Rejoining restores that marked living body, clears the offline marker, commits and enables normal control; it never grants another starter inventory or resurrects a genuinely dead character. Corpse/death behavior stays under the game's shared rules. Storage failure fails closed under the existing durable-owner contract.

Gameplay-camera permissions and logout policy participate in canonical content identity. Endpoint/deployment addresses remain independent. These policies belong to the project/world owner and require no managed hosting service.
