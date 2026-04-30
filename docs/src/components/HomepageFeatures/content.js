export const homepageContent = {
  hero: {
    eyebrow: "Retro RPG Creator",
    title: "Build retro RPG worlds for 2D, 3D, and interactive fiction",
    description:
      "Eldiron is a game creator for classic RPGs. One editor brings together map building, tile workflows, visual scripting, narrative authoring, and cross-platform play.",
    actions: [
      {
        label: "Getting Started",
        href: "/docs/getting_started",
        className: "button button--primary button--lg",
      },
      {
        label: "Read Dev Updates",
        href: "/blog",
        className: "button button--secondary button--lg",
      },
    ],
    screenshot: {
      label: "New turn based simulation",
      version: "Eldiron v0.9.8",
      image: "/img/screenshots/Eldiron_v0.9.7_TB.png",
      alt: "Eldiron turn-based simulation screenshot",
    },
  },
  sections: [
    {
      key: "news",
      eyebrow: "News",
      title: "What changed recently",
      description:
        "Follow the latest Eldiron releases, workflow improvements, and documentation updates as the project moves toward v1.",
      type: "news",
      items: [
        {
          date: "Apr 26, 2026",
          title: "Eldiron v0.9.7",
          description:
            "Turn-based and hybrid simulation modes, NPC sequences, multiple-choice menus, nested dialogs, organic painting, and new renderer/post-processing controls.",
          href: "/blog/2026/04/26/eldiron-v0.9.7",
          linkLabel: "Read more",
        },
        {
          date: "Apr 19, 2026",
          title: "Eldiron v0.9.3",
          description:
            "A mostly bugfix release with stronger 2D support, including LOS, point-and-click style intents, auto-walk, and runtime world/region render control.",
          href: "/blog/2026/04/19/eldiron-v0.9.3",
          linkLabel: "Read more",
        },
        {
          date: "Apr 6, 2026",
          title: "Eldiron v0.9.2",
          description:
            "Dungeon Tool, Tile Picker rewrite, expanded Tile Graph workflows, Builder Graph, and broader authoring improvements.",
          href: "/blog/2026/04/06/eldiron-v0.9.2",
          linkLabel: "Read more",
        },
      ],
    },
    {
      key: "formats",
      eyebrow: "World Building",
      title: "Choose the presentation that fits your game",
      description:
        "Build top-down adventures, isometric worlds, and first-person dungeons with one connected editor and one shared project pipeline.",
      type: "formats",
      items: [
        {
          eyebrow: "2D",
          title: "Build classic top-down adventures",
          description:
            "Draw regions, paint with tiles, script interactions, and build retro RPG worlds with a fast map-making workflow.",
          image: "/img/screenshots/Eldiron_v0.92_2D.png",
          alt: "Eldiron 2D screenshot",
          href: "/docs/building_maps/creating_2d",
          linkLabel: "Explore 2D Workflow",
        },
        {
          eyebrow: "3D",
          title: "Shape dungeons, towns, and terrain in 3D",
          description:
            "Mix sectors, profiles, terrain, materials, and tile painting to create first-person or isometric worlds without a separate 3D toolchain.",
          image: "/img/screenshots/Eldiron_v0.92_3D.png",
          alt: "Eldiron 3D screenshot",
          href: "/docs/building_maps/creating_3d_maps",
          linkLabel: "Explore 3D Workflow",
        },
        {
          eyebrow: "Text",
          title: "Build text-based adventures in the same world",
          description:
            "Use authoring, intents, rules, and shared world data to create interactive fiction and text-style play directly from your Eldiron project.",
          image: "/img/screenshots/Eldiron_v0.92_CLI.png",
          alt: "Eldiron text-based play screenshot",
          href: "/docs/creator/authoring",
          linkLabel: "Explore Text Workflow",
        },
      ],
    },
    {
      key: "tools",
      eyebrow: "Key Tools",
      title: "Focused workflows inside the editor",
      description:
        "From fast dungeon blockouts to procedural tiles and narrative authoring, these tools shape the way worlds come together in Eldiron.",
      type: "tools",
      items: [
        {
          title: "Simulation Modes",
          description:
            "Choose realtime play, fully turn-based stepping, or a hybrid mode that advances on player action and then continues after an idle timeout. This lets the same project support active RPG movement, deliberate tile-by-tile tactics, or Ultima-style pacing.",
          image: "/img/screenshots/Eldiron_v0.9.7_TB.png",
          alt: "Turn-based simulation mode settings in Eldiron",
          href: "/docs/configuration/game",
          linkLabel: "Open docs",
        },
        {
          title: "Dungeon Tool",
          description:
            "Block out rooms, corridors, shafts, and doors in a conceptual grid, then let Eldiron generate editable map geometry from it. You can even dynamically change fog and other render settings.",
          image: "/img/screenshots/Eldiron_v0.92_DT.png",
          alt: "Dungeon Tool preview",
          href: "/docs/creator/tools/dungeon",
          linkLabel: "Open docs",
        },
        {
          title: "Interactive Fiction",
          description:
            "Layer narrative metadata onto sectors, linedefs, and entities, and use Eldiron's powerful intent system to build a world model that can be explored entirely through text.",
          image: "/img/screenshots/Eldiron_v0.92_IF.png",
          alt: "Authoring workflow screenshot",
          href: "/docs/creator/authoring",
          linkLabel: "Open docs",
        },
        {
          title: "Tile Graph",
          description:
            "Author procedural tile groups with reusable node graphs, layered materials, automatic wrapping, and more. Tile graphs can span multiple tile blocks, allowing for larger procedural detail.",
          image: "/img/screenshots/Eldiron_v0.92_TG.png",
          alt: "TileGraph screenshot",
          href: "/docs/creator/docks/tile_node_graph",
          linkLabel: "Open docs",
        },
        {
          title: "Builder Graph",
          description:
            "Create reusable builder graphs for props, furniture, fences, and other structural assemblies, with live preview, material slots, and attachment points for nested builds.",
          image: "/img/screenshots/Eldiron_v0.92_BG.png",
          alt: "Builder Graph preview",
          href: "/docs/builder_graph",
          linkLabel: "Open docs",
        },
        {
          title: "Tile Picker",
          description:
            "Arrange tiles on the new Tile Picker board, and create, edit, and share collections, tile groups, and tile graphs from one central workflow.",
          image: "/img/screenshots/Eldiron_v0.92_TP.png",
          alt: "Tile Picker screenshot",
          href: "/docs/creator/docks/tile_picker_editor",
          linkLabel: "Open docs",
        },
      ],
    },
  ],
};
