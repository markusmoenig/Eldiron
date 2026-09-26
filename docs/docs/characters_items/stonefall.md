---
title: "Stonefall Dungeon"
---

Open `test_projects/StonefallDungeon.eldiron` in Creator, or run it with an Eldiron client. The starter catalog includes the same example.

Stonefall uses first-person grid movement with a start screen, party profiles and inventory screen. Player input, initial entity settings, enemy routines and combat, Alden's recruitment/healing, torches and the Bone Key exit are authored as nodes.

Walls, floors and ceilings are editable with the Wall tool. Twenty room and corridor sections each have paired floor and ceiling surfaces, bounded by fully open wall spans so passages remain clear. The dungeon keeps ceiling heights of 2.25, 3.0 and 4.25 units. The wall torch is a mounted prefab with particle and light effects. Decorative beams and recessed niches have been simplified.

Alden's support branch reacts to Party Damaged, checks the injured member's health and invokes the ruleset's Minor Heal action. Party capacity, action costs, reagents and cooldowns come from the ruleset.
