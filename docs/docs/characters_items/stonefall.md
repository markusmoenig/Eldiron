---
title: "Stonefall Dungeon"
---

Open `test_projects/StonefallDungeon.eldiron` in Creator, or run it with an Eldiron client. The starter catalog includes the same example.

Stonefall uses first-person grid movement with a start screen, party profiles and inventory screen. Player input, initial entity settings, enemy routines and combat, Alden's recruitment/healing, torches and the Bone Key exit are authored as nodes.

The dungeon uses ordinary editable Geometry Objects for its walls, floors and ceilings. The torch prefab provides particle and light effects.

Alden's support branch reacts to Party Damaged, checks the injured member's health and invokes the ruleset's Minor Heal action. Party capacity, action costs, reagents and cooldowns come from the ruleset.
