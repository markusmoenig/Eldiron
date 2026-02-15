---
title: "Getting Started"
sidebar_position: 1
---

As explained in the [Creator's Project Tree](/docs/creator/project_tree) chapter, [characters](/docs/creator/project_tree/#characters) and [items](/docs/creator/project_tree/#items) are reusable templates for certain classes of characters or items.

![Characters and Items](/img/docs/characters_items.png)

You define them by editing their behavior and attributes:

* **Name**. The name of the character or item class, such as *Orc*. You can change the name anytime.
* **Visual Scripting**. Edit behavior using nodes in the visual scripting editor; the behavior is translated into Eldrin Script.
* **Eldrin Scripting**. Edit behavior source code directly in the [Eldrin Scripting](eldrin_scripting_language) language.
* **Attributes**. The initial character and item attributes. Most of these can be changed later via scripting; however, some attributes define core values set during startup.

---

[Eldrin Script](eldrin_scripting_language) drives the entire behavior system of **Eldiron**. Visual scripts are translated into *Eldrin Script*, and you can always view the current source code in the editor. Whether you use visual scripting or *Eldrin* scripting directly is up to you; just note that when you use visual scripting, any manual edits in the scripts will be overwritten.

## Difference between Characters and Items

Even though characters and items are very similar. There are some key differences:

- Items cannot take damage or move and cannot die (but they can be destroyed).
- Items are designed to be handled in inventories of characters.

Every character and item has a unique ID used to reference it.

---

**Attributes** of characters and items are set in the **Attributes** **TOML** editor.

Most important attributes:

```toml
player = true
```

Defines a player based character. Only players get send [user events](client_commands). These commands map user input to **actions** and **intents**.

You can find a complete reference of all [attributes here](attributes).

---

## Player Characters

While all characters and items get send [event](events), only player characters get send **user events**. *User events* are used to map user events (key presses, mouse events) to an [action](client_commands/#action) or [intent](client_commands/#intent). These are client commands, as user input is processed on the client and send to the server. All other commands are processed on the server.
