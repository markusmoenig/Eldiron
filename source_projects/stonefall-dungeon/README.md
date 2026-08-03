# Stonefall Dungeon

Dungeon Master-style Eldiron Source sample project.

Build:

```sh
target/release/eldiron-source build source_projects/stonefall-dungeon
```

Play:

```sh
target/release/eldiron-source play source_projects/stonefall-dungeon
```

Run the graphical client with:

```sh
target/release/eldiron-client source_projects/stonefall-dungeon/build/stonefall-dungeon.eldiron
```

This project is intentionally source-first: regions, characters, screens, and
tile symbol mappings live in `.els` files and compile to a regular `.eldiron`
project.

## Encounters

Stonefall uses the official ruleset's Orc and Skeleton avatars directly. The
project only authors encounter roles and behavior:

- the first Orc teaches the basic attack loop and wears a project-local
  recipe-patterned hide vest and procedural iron helmet;
- an Orc Brute follows in the low eastern room;
- Skeletons and a ranged Bone Archer defend the crypt;
- the level-three Bone Warden guards the exit chamber and drops a Bone Key.

Hostiles patrol after losing a target and keep proximity tracking active, so
leaving combat does not permanently disarm them. Damage also causes immediate
retaliation. `follow_attack` respects the equipped weapon range, allowing the
Bone Archer to use the official Hunting Bow at range while melee enemies close
in. Defeated enemies drop their ruleset equipment and a small, thematic
crafting collectible.

The exit marker is a real blocking gate: walking into it or inspecting it with
Look checks the player's inventory. It opens only after the player takes the
Bone Key from the Warden's remains. Custom monster atlases can replace these
shared avatars later without changing the encounter scripts or map.

The vest demonstrates project-owned procedural appearance on a ruleset-owned
Avatar. `items/orc-patterned-vest.els` binds the `torso` marker channel to
`recipes/orc-vest.recipe`; the official Orc and equipment definitions remain
unchanged. The equipped `items/orc-iron-helm.els` combines the reusable SDF
silhouette in `recipes/orc-helm.recipe` with the bright hammered-steel Material
in `recipes/orc-helmet-material.recipe`. Its shallow cap, separate brow and
cheek guards use a bright blue-grey steel finish. The runtime fits that overlay
to the actual head-marker bounding box for each Avatar direction.

Targeted action buttons are persistent in this example: selecting Attack once
keeps it active for subsequent targets until another mode is chosen. On death,
the player returns to the authored dungeon entrance with restored ruleset health
instead of remaining beside the hostile that killed them.

## Procedural environment tiles

Stonefall's wall, floor, and ceiling live as height-first text recipes in
`recipes/`. Eldiron Source executes them with the project's art palette while
building and embeds their results as normal tiles. No generated PNGs or
edge-crossfade scripts are part of the asset pipeline. Recipe `coverage`
controls how many dungeon cells share one continuous texture footprint; the
primary wall uses coarse, perturbed masonry rather than a small regular
brick grid.

Reusable color graphs, Roughness/Metallic/Opacity/Emission data, and
micro-normal settings live in unified `Surface` blocks inside
multi-declaration `.recipe` files. Materials are independent of tile height;
tile recipes own geometry and any material masks. The three environment recipes
reference `dungeon.recipe`.

The terrain in `regions/dungeon.els` is a first-person dungeon with distinct
chambers, connecting galleries at least three cells wide, a trap hall, a crypt,
a fountain room, and an exit chamber. `main.els` keeps the map readable by assigning
single-character symbols to:

- the procedural irregular-stone wall;
- the procedural flagstone floor;
- the procedural rough-stone ceiling.

Tile recipes may set `blocking = true`, so Stonefall's map can infer collision
from `wall-stone` and `wall-niche` instead of repeating it on every symbol.
Walkable symbols may use
`ceiling=<tile>` to theme ceilings per chamber and
`ceiling_height=<world-units>` to give chambers low, normal, or tall
proportions. A Region-level `ceiling_height` supplies the default (3.0 when
omitted). The compiler closes height changes with generated transition faces.
Blocking symbols may use `profile=pillar` for a protruding structural column.

Architectural features travel with their tile recipe. `wall-niche.recipe`
contains the complete recess definition:

```text
Geometry
  Box Recess
    operation = Subtract
    surface = niche-stone
    position = F3(0.12, 0.76, 0.0)
    size = F3(0.76, 1.06, 0.46)
```

The map only says `"!" = wall-niche`. The generic `Subtract` operation carves
the Box on every exposed placement face, keeps the surrounding wall solid, and
applies `surface` to the cavity. The same recipe uses the signed
`Recess.distance` field to remove stone from a noisy, smoothstepped strip around
the opening:

```text
Height RecessJoint
  source = 1.0 - Smoothstep(0.025, 0.080, Abs(Recess.distance) + (BorderWear - 0.5) * 0.028)
```

That synchronized material mask makes the mortar joint follow the generated
opening while remaining slightly worn and irregular. Changing the recipe
updates material, collision, and geometry together without editing the map.

`ceiling-stone.recipe` uses the same general path for structural detail:

```text
Geometry
  Box CrossBeam
    operation = Add
    surface = ceiling-beam-wood
    position = F3(0.41, 0.0, 0.0)
    size = F3(0.18, 0.12, 1.0)
```

The Source adapter supplies a ceiling-local placement basis and the additive
Box projects below the stone ceiling. `ceiling-beam-wood.recipe` provides its
hewn height detail and uses the reusable `dungeon/beam_wood` material for dark
grain, pores, roughness, and micro-normal variation. There is no beam-specific
or niche-specific geometry feature in the Recipe AST.
