# Editable ruleset geometry

These standalone MZ documents are the defaults installed in new projects.
Creator stores authored project overrides separately. Geometry and Grip edits
participate in the existing project undo/save workflow; ruleset refreshes retain
those overrides.

For weapons, open **Appearance** under the item and choose **Model On** or
**Model Off**. Characters provide **Edit model**. The root group supplies dimension controls, and opening it exposes the
ordinary construction rows. Grips are editable in the Attachments view.

| Weapon | Construction | Published dimensions |
| --- | --- | --- |
| [Training Sword](training_sword.mzgeometry.json) | Wooden practice blade, guard, handle and pommel | Blade length/width/thickness, guard width, handle length |
| [Hand Axe](hand_axe.mzgeometry.json) | Extruded iron head, turned shaft and leather grip | Head width, haft length |
| [Round Shield](round_shield.mzgeometry.json) | Curved shell with front icon texture and handle | Width, height, curve |
| [Novice Mace](novice_mace.mzgeometry.json) | Ash haft, wrapped grip, iron core and six radial flanges | Overall length, head size, grip width |
| [Hunting Bow](hunting_bow.mzgeometry.json) | Curved stave, taut string, bound riser and nocked arrow | Bow height, arrow length, curve |
| [Training Spear](training_spear.mzgeometry.json) | Tapered shaft, wrapped socket, faceted leaf point and butt cap | Overall length, point width, shaft width |

The mace, bow and spear surface recipes live next door in `../materials`.
Their companion palette JSON files bind every color to project palette indices,
matching the corresponding icon components. `weapon-iron`, `weapon-ash` and
`weapon-wrap` are shared; `bow-grip`, `bow-string`, `arrow-shaft`, `arrow-feather`
and `spear-binding` describe the remaining parts. Metallic response belongs to
iron rather than wood, leather or string. The icon compositions remain separately
editable inventory artwork.

Held items use their Grip frame to fit the character socket. The bow is carried
in the left hand while the right hand draws; the spear uses a forward two-hand
grip. Weapon-specific ready, walking and attack clips are described below.

`orc.mzgeometry.json` and `skeleton.mzgeometry.json` provide the other humanoid
body recipes. The `garment_*.mzgeometry.json` documents consume the current
wearer body to construct fitted clothing, with the same palette-bound material
library in 2D and 3D.

Every official item now also has a standalone world model. The 35 additional
models cover dropped clothing, focus items, bags, arrows, reagents, materials,
resource nodes, coins and torches. Inventory keeps its separately editable MZ
icon; the 3D scene uses the item's `appearance_geometry` recipe. Harvesting,
blocking, takeability and other gameplay properties remain ruleset data.

The new models expose **Overall size** in their root group. Open the group for
ordinary loft, contour, extrusion, box, transform, material and texture rows.
The `item-*.material` recipes and companion palette bindings in `../materials`
match the inventory artwork's palette slots. Glass bottles use opaque stylized
surfaces with modelled highlights; they do not simulate transparent glass.

`worn_small_bag`, `worn_ritual_censer` and `worn_sunward_charm` consume the
current wearer through `geometry:input/wearer`. Their Asset row references the
same editable standalone model, followed by a mounting Grip and Attach row.
The bag and censer hang at opposite hips from the pelvis; the charm attaches
to the chest. Socket position, rotation and grip remain editable in MZ. Equipped
accessories retain their rigid skin weights and attachment frames alongside
fitted clothing on human, orc and skeleton characters.

`torch` and `torch_off` are separate editable states selected by the existing
`active` item attribute. `appearance_geometry_off` is the explicit unlit binding;
the Appearance workspace exposes both model states. The lit model's MZ recipe
can attach palette-linked particle effects through the existing particle system.
The item's behavior still owns light switching and fuel use; the unlit model
does not emit. The inventory icon retains its separate four-frame flame animation.

Review all 42 model/icon states and accessory poses without a GPU window:

```sh
cargo run -p eldiron-creator --example ruleset_models -- /tmp/mz-models
```

Add a trailing palette size (for example `16`) to review a restricted project
palette. To review the actual scene paths with ground models and equipped actors:

```sh
cargo run -p eldiron-creator --example appearance_scene -- test_projects/3DStarter.eldiron /tmp/mz-item-scenes --default-palette --items
```


## Weapon animations

All three character recipes contain editable weapon clips. Item appearance
attributes select them in both directional 2D sprites and native 3D geometry:

| Item | Attack | Duration | Other clips |
| --- | --- | --- | --- |
| Training Sword | `sword-slash` | 0.60 s | Standard idle / walk |
| Hand Axe | `axe-chop` | 0.72 s | Standard idle / walk |
| Novice Mace | `mace-swing` | 0.78 s | Standard idle / walk |
| Hunting Bow | `bow-shot` | 1.05 s | `bow-ready`, `bow-walk` |
| Training Spear | `spear-thrust` | 0.72 s | `spear-ready`, `spear-walk` |

The bow pushes forward during the draw, its limbs flex and its two string
segments meet at the nock. Release sends the arrow forward, hides it after flight,
and restores it for the next shot. The spear draws back, thrusts with a forward
weight shift, then recovers. Both hands remain on the shaft. Attacks keep their
feet planted; ready walking clips include alternating steps. Human, orc and
skeleton poses account for their authored proportions.

`appearance_idle`, `appearance_walk` and `appearance_attack` select character
clip IDs. `appearance_hold_joint` chooses the holding hand, and
`appearance_two_handed` suppresses off-hand geometry. The server visual timer and
both renderers share `appearance_attack_time`; an explicit entity
`avatar_attack_time` overrides it. A fresh attack serial restarts playback, and
movement does not interrupt an active attack.

Open a character recipe and select its named motion to edit the body keys. Open
Hunting Bow and select **Bow · draw, release and reload** to edit the bow's own
six-joint rig and keys. Its `bow-shot` tracks join the character's identically
named clip at runtime. MZ's selected-key **Joint branch: Visible / Hidden** control
sets the discrete visibility channel used to hide and re-nock the arrow. Geometry,
material bindings, grips, rig hierarchy and keys remain ordinary saved MZ data.

The released arrow is an animation visual, not a new server projectile. Existing
ammunition, hit resolution, damage and attack cooldown rules still own combat;
this pass does not defer damage to the visual release frame.

Review exact poses for all three races in 2D, or add `--gpu` for native 3D frames:

```sh
cargo run -p eldiron-creator --example weapon_animations -- /tmp/mz-attacks --gpu
```

The export includes ready, draw/windup, strike/release and recovery phases for
all five weapons, plus the evaluated bind rigs for checking attachment contact.

## Orc anatomy and movement

`orc.mzgeometry.json` has its own authored body profiles: a broad barrel chest
and upper back, sloping trapezius, thick forward-set neck, square jaw, long heavy
arms, enlarged forearms/hands, shorter thick legs and wide feet. The unarmored
chest and shoulders use the palette-bound `orc-skin` material. Trousers and the
waist use the shared wool materials.

The compact **Orc anatomy** group keeps the standard proportion controls and
adds **Hand mass**, **Forearm mass** and **Forward stoop**. 100% refers to the
authored orc baseline. Height fits after the proportion controls. Open the group
to edit individual profiles, bind frames and the ordinary 17-joint humanoid rig.
All body part IDs and equipment sockets remain compatible with fitted clothing
and the shared item models.

The orc's idle clip uses a slow breathing/settling cycle. Its 1.35-second walk
has a broad stance, longer planted phases, double support, low foot lift, body
weight shifts and delayed arm swing. Heavy strikes use a more deliberate wind-up
and a stronger torso/hip follow-through. Bow and spear body keys were authored
against this rig, including extra clearance in front of the chest and exact
support-hand/string contacts. These are saved MZ motion rows, sampled by the same
2D and 3D appearance evaluator.

Review the body from front, side, back and three-quarter views, together with
linen, chainmail, a weapon, shield, bag and charm:

```sh
cargo run -p eldiron-creator --example orc_appearance -- /tmp/mz-orc-review --gpu --animate
```

The export includes the 2D directional sprites, native 3D views and walk/attack
frame sequences. An optional `--before <recipe>` adds an earlier MZ recipe to the
comparison without changing any project. For the actual scene paths, use
`appearance_scene` with `--clothing --orc --default-palette` and either starter
project.
