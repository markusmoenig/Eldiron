# Fauna foundation

Implemented baseline, 2026-11-18. This supplies the missing living source for
animal hides in the world-to-leather chain.

## Supplied animals

Use Creator's **Project Add → Add Animal**. It opens **Entity Nodes** with
**Entity → Animal** already connected. The Animal node selects Deer, Pig, Cattle, Sheep,
Goat, Horse or Chicken from the active ruleset. Leave Entity race and class
empty; the species supplies both. Place instances using the existing character
placement workflow. Add Animal is a plain menu entry; species selection lives
on the node, with no animal submenu or ruleset enable switch.

Animals retain the same editable **Behavior Nodes** as NPCs. Add a
**Routine → Random Walk** graph for wandering, or author other behavior.
Selecting a species never generates or replaces the character's behavior graph.

Creator populates the picker from the bundled ruleset selected by the project.
A project authored before fauna was added can use the current species immediately.

The shared `animals` catalogue references ordinary races and the `Animal` class.
Races supply anatomy, attributes, collision radius, presentation size and the
remains binding. The class supplies no humanoid clothes or inventory and disables
automatic NPC respawning by default. A project's gameplay can override that.

The Random Walk behavior wanders and rests on walkable ground. Its distance, movement
speed, pause and optional named area remain ordinary behavior-node controls.
Use a named area or physical enclosure to keep livestock in a pasture; the
default distance is a per-leg wandering distance, not a permanent spawn leash.

## One MZ source for 2D and 3D

Each species has an independently editable MZ geometry document under
`crates/ruleset/rulesets/eldiron/v1/assets/geometry/animals/`. These are mesh
construction rows, palette-bound PBR materials, a quadruped or bird armature,
skin bindings and keyed motion rows. They are not raster stand-ins or humanoid
models with different names.

Every living model includes **Idle**, **Walk**, **Run**, **Graze**, **Attack** and
**Dead** clips. MZ's ordinary motion editor owns their timing and joint keys.
The stock routine uses Idle and Walk; the other clips are available for further
behavior work. Their presence does not implement fleeing, feeding or combat
decisions. Directional 2D and isometric sprites and native 3D geometry sample the
same saved rig and clips. Sprite framing accommodates broad quadrupeds.

Creator's Appearance workspace offers these models in its model picker.
**Edit model** opens the species' MZ construction and motion rows. Project model
variants and overrides use the existing appearance editing and save workflow.

The corresponding `*_remains` document uses the species' anatomy resting on its
side as static geometry. A carcass keeps the animal's facing. It is an independent
editable appearance, allowing later spent or decayed artwork without resetting
its resource state.

Review the supplied assets and animations:

```sh
cargo run -p rusterix --example animals -- /tmp/eldiron-animals --gpu
```

This exports eight-direction pose sheets and six motion GIFs per species, plus
native GPU 3D walking previews. Omit `--gpu` for a CPU-only sprite review.

## A complete source, not a hypothetical ingredient

Living animals cannot be skinned. Ordinary authoritative combat kills the animal;
the existing death lifecycle creates its finite resource-bearing carcass once.
It does not roll hides into the ground or create vendor stock.

| Animal | Primary carcass supply | Other finite stock |
| --- | --- | --- |
| Deer | 1 raw hide | Existing deer profile retained unchanged |
| Pig | 1 raw hide | 2 raw bones |
| Cattle | 2 raw hides | 2 raw bones |
| Sheep | 1 raw hide | 2 raw bones |
| Goat | 1 raw hide | 2 raw bones |
| Horse | 2 raw hides | 2 raw bones |
| Chicken | 4 feathers | 1 raw bone |

Hides require an owned tool with `skin`, such as the world-crafted stone knife.
Collecting bones from the new animal profiles also requires that tool. Feather
collection does not. A successful tool-based withdrawal costs one condition;
failed or repeated withdrawals spend nothing. Separate supplies allow harvesting
a hide without discarding the remaining bones. These carcasses never regenerate;
they become spent when empty and expire after two simulated days.

The existing tanning recipe consumes one raw hide, two tree bark and four fresh
water at a deployed tanning frame to produce two leather. Flint, branches, flax,
bark and river water bootstrap the knife, frame and vessel. The tested path starts
with empty pockets, hunts a living pig and produces leather from its hide.

## Authoring and identity

The Animal configuration node generates ordinary character identity data.
Selecting Pig is equivalent to the following; behavior remains a separate graph:

```toml
[attributes]
animal = "pig"
race = "Pig"
class = "Animal"
player = false
```

The Pig race inherits the following source and appearance bindings:

```toml
[races.Pig]
default_avatar = "pig"

[races.Pig.attributes]
remains_profile = "pig_remains"
remains_geometry = "ruleset:geometry/pig_remains"
size = 0.85
```

These snippets illustrate the relevant fields; the bundled race also defines its
health and other attributes. A placed character retains its normal template and
instance UUIDs. Death creates one resource-access UUID and one ordinary world item.
Repeated callbacks, reconnect and world reload preserve those identities and
remaining stock. This uses the existing local and multiplayer request, replication
and persistence paths without a new wire identity or a second animal simulation.

## Deliberate next layers

This pass supplies living sources, meshes, motion, placement and finite remains.
It does not add breeding, growth, hunger, domestication, riding, herding, milk,
shearing, egg laying, meat recipes or population management. Those should extend
the existing world-source and shared-operation contracts with explicit resources,
requirements and lifecycle rules. An animation named Graze does not consume food.

Animal population and economic participation remain game-defined policy for solo
and online games. An unlimited respawn or vendor should not silently replace the
world-source accounting. The supplied leather armor family, permanent learning,
repair and Fox inscription now build on this source foundation. Its focused
solo/dedicated scenario starts empty, hunts actual animals and preserves the
resulting equipment, knowledge and enchantments across world reload.
