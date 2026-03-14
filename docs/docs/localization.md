---
title: "Localization"
sidebar_position: 6.6
---

**Localization** is configured in the creator via **Game / Locales**.

Locale data uses **TOML**. Each top-level table is one language, for example `[en]`, `[de]`, or `[fr]`.

## Format

```toml
[en]
combat.damage.incoming = "{attacker} hits you for {amount} damage"
combat.damage.outgoing = "You hit {defender} for {amount} damage"
system.cant_do_that_yet = "Can't do that yet"
system.cant_afford = "You can't afford that"
system.you_bought = "You bought"
system.exit_menu = "Goodbye"

[de]
combat.damage.incoming = "{attacker} trifft dich fuer {amount} Schaden"
combat.damage.outgoing = "Du triffst {defender} fuer {amount} Schaden"
system.cant_do_that_yet = "Das geht noch nicht"
system.cant_afford = "Du kannst dir das nicht leisten"
system.you_bought = "Du hast gekauft"
system.exit_menu = "Auf Wiedersehen"
```

The active locale is selected in **Game / Settings**:

```toml
[game]
locale = "en"
```

Use `locale = "auto"` to follow the system locale.

## Key Names

Use namespaced keys.

- `system.*`: built-in runtime and UI strings
- `combat.*`: combat-related text
- your own domains like `quest.*`, `dialog.*`, `merchant.*`, `ui.*`

This keeps project strings organized and avoids collisions.

## Built-in System Keys

The currently used built-in system keys are:

- `system.cant_do_that_yet`
- `system.cant_afford`
- `system.you_bought`
- `system.exit_menu`

These are used by server/client systems directly, so they should exist in every supported locale.

## Message Resolution

Localized text is resolved by key first, then placeholders inside the final string are filled.

Example:

```toml
[en]
combat.damage.outgoing = "You hit {defender} for {amount} damage"
```

If rules reference `combat.damage.outgoing`, the runtime first loads that locale string, then replaces placeholders like:

- `{attacker}`
- `{defender}`
- `{amount}`
- `{kind}`
- `{from_id}`
- `{target_id}`

For your own custom `message(...)` calls, you can pass named parameters on the locale key itself.

Example locale entry:

```toml
[en]
dialog.hit = "You hit {target} for {amount} damage"
```

Example script message:

```eldrin
message(id(), "{dialog.hit,target=target.class_name,amount=N:3}", "system")
```

Supported parameter value forms:

- `E:<id>.<attr>` for entity attributes like `E:11.name` or `E:11.class_name`
- `It:<id>.<attr>` / `Item:<id>.<attr>` for item attributes
- `N:<value>` for integers
- `F:<value>` for floats
- `self.<attr>`, `sender.<attr>`, `attacker.<attr>`, `target.<attr>`, `item.<attr>` for message-context shortcuts
- plain text like `target=Urg`

That means:

- `E:11.name` usually gives the instance name, for example `Urg`
- `E:11.class_name` gives the template/class name, for example `Orc`
- `target.name` usually gives the sender instance name in a received message
- `target.class_name` usually gives the sender class name in a received message
- `self.name` gives the receiver entity name

## Rules Integration

Rules-driven combat messages should usually use locale keys, not hardcoded English strings.

```toml
[combat.messages]
incoming_key = "combat.damage.incoming"
incoming_category = "warning"
outgoing_key = "combat.damage.outgoing"
outgoing_category = "system"
```

This lets you translate combat text without changing gameplay rules.

## Text Widgets

Screen `text` widgets use the same locale-key system.

Example widget text:

```text
{ui.status.gold}: {PLAYER.FUNDS}
{ui.quest.ready}
```

Example locale entries:

```toml
[en]
ui.status.gold = "Gold"
ui.quest.ready = "Quest ready"
```

Current behavior:

- locale keys like `{ui.status.gold}` are resolved through `Game / Locales`
- existing `PLAYER.*` status placeholders still work in text widgets
- `PLAYER.FUNDS` shows the current player funds
- `PLAYER.<ATTR>` shows a player attribute value
- `WORLD.HOUR`, `WORLD.MINUTE`, `WORLD.TIME`, `WORLD.TIME_12`, and `WORLD.TIME_24` show the current in-game time
