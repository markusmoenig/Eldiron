---
title: "Rules"
sidebar_position: 6.5
---

**Rules** define project-wide gameplay formulas.

You can change them in the creator via the **Game / Rules** item in the project tree.

## Format

Rules use **TOML**.

```toml
[combat]
incoming_damage = "value + attacker.STR - defender.ARMOR"

[combat.messages]
incoming_key = "combat.damage.incoming"
incoming_category = "warning"
outgoing_key = "combat.damage.outgoing"
outgoing_category = "system"

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

## Combat Messages

You can also define automatic combat messages in rules so every monster does not need its own `take_damage` message script.

```toml
[combat.messages]
incoming_key = "combat.damage.incoming"
incoming_category = "warning"
outgoing_key = "combat.damage.outgoing"
outgoing_category = "system"
```

The message key is looked up in **Game / Locales** using the active locale from `Game -> Settings`.

```toml
[game]
locale = "en"
```

Example locale entries:

```toml
[en]
combat.damage.incoming = "{attacker} hits you for {amount} damage"
combat.damage.outgoing = "You hit {defender} for {amount} damage"
```

Supported placeholders inside locale strings:

- `{attacker}`
- `{defender}`
- `{amount}`
- `{kind}`
- `{from_id}`
- `{target_id}`

Message categories:

- `incoming_category`
- `outgoing_category`

These use the same category string you already use with `message(...)`, for example `system`, `warning`, or custom categories styled by your game.

If you do not want localization for a rule message, you can still use literal `incoming` / `outgoing` strings instead of `incoming_key` / `outgoing_key`.

Automatic combat messages are only sent when a player is involved:

- `incoming`: only if the defender is a player
- `outgoing`: only if the attacker is a player

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

For general localization and built-in `system.*` keys, see [Localization](./localization).
