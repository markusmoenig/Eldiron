
**Eldiron Creator v 0.8.50**

**New Features**

*Server*

- New 'entered' and 'left' entity system events when entering / leaving named sectors.
- New 'teleport' command which either takes one argument (destination sector name in the same region) or two, the destination sector name and the name of the destination region (which will transfer the character to a different region). The entity will be transferred to the center of the destination sector.
- New `[light]` data attributes to enable light emission for entities and items.
- New 'set_emit_light(True / False)` cmd to enable / disable light emission for an entity or an item.
- New special `active` item attribute which specifies if the item is active or not. On startup (or if the attribute is changed) a new `active` event is send to the item which can than decide what to do based on the value, like enabling / disabling light emission for a torch.

*Creator*

- Tileset tool: Preview icons now in the minimap.
- Tilepicker: Icons preview on hover in the minimap.

**Bug Fixes**

- Make game widgets honor the global render graph.
- Info viewer did not show item values correctly.

