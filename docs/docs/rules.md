---
title: "Rules"
sidebar_position: 6.5
---

**Rules** define project-wide gameplay formulas such as damage calculation.

You can change them in the creator via the **Game / Rules** item in the project tree.

## Format

Rules use **TOML**.

```toml
[combat]
incoming_damage = "value + attacker.STR - defender.ARMOR"

[combat.kinds.physical]
incoming_damage = "value + attacker.STR - defender.ARMOR"

[combat.kinds.spell]
incoming_damage = "value + attacker.INT - defender.RESIST"

[combat.kinds.fire]
incoming_damage = "value + attacker.INT - defender.FIRE_RESIST"
```

## Combat Rules

The first system using rules is damage calculation.

Available variables in combat formulas:

- `value`: The incoming base amount.
- `attacker.<attr>`: Reads an attacker attribute from data.
- `defender.<attr>`: Reads a defender attribute from data.
- `weapon.<attr>`: Reads the attacker's equipped weapon attribute when available.

If `combat.kinds.<kind>.incoming_damage` exists, it overrides the base combat formula for that damage kind.

Damage kinds are passed through the runtime payload:

- `deal_damage(...)` defaults to `physical`
- spells default to `spell`
- custom kinds like `fire`, `ice`, or `poison` can override the base rule

After the server resolves the final amount, `take_damage` receives:

- `amount`: final incoming damage
- `from_id`: attacker id
- `damage_kind`: kind string
- `attacker_name`: resolved attacker name

The server applies the final damage automatically after `take_damage` returns.
