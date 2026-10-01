---
title: "Stonefall Dungeon"
---

Open `test_projects/StonefallDungeon.eldiron` in Creator, or run it with an Eldiron client. The starter catalog includes the same example.

Stonefall uses first-person grid movement with a start screen, party profiles and inventory screen. Player input, initial entity settings, enemy routines and combat, Alden's recruitment/healing, torches and the Bone Key exit are authored as nodes.

The dungeon walls use Wall Tool geometry. The previous hard-coded floors and ceilings have been removed so they can be rebuilt with Wall Tool surfaces and Construction Patterns. Until those surfaces are authored, the dungeon has no walkable floor or ceiling meshes. The wall torch is a mounted prefab with particle and light effects.

Alden's support branch reacts to Party Damaged, checks the injured member's health and invokes the ruleset's Minor Heal action. Party capacity, action costs, reagents and cooldowns come from the ruleset.
