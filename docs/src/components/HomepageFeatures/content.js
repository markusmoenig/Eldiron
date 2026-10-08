export const homepageContent = {
  hero: {
    eyebrow: "Retro RPG Creator",
    title: "Build retro RPG worlds for 2D, 3D, and interactive fiction",
    description:
      "Eldiron is a game creator for classic RPGs. One editor brings together map building, tile workflows, behavior nodes, narrative authoring, and cross-platform play.",
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
      label: "Bring your world to life with Nodes.",
      linkLabel: "See release.",
      href: "/blog/2026/10/01/eldiron-v0.95.0",
      version: "Eldiron v0.95.0",
      image: "/img/eldiron_v0.95.0.png",
      alt: "Eldiron Creator v0.95.0 showing Hideout 2D and an Orc routine with highlighted Behavior Nodes",
    },
  },
  sections: [
    {
      key: "rules-announcement",
      eyebrow: "NOW AVAILABLE",
      title: "Official Eldiron Ruleset",
      description:
        "Eldiron supports project-selectable ruleset packages with official fantasy rules from levels 1–10. Races, classes, actions, spells, conditions, equipment, progression, resources, and crafting share one runtime while projects remain free to extend or replace the rules they need.",
      href: "/docs/official_rules",
      linkLabel: "Read the official rules",
      thumbnail: {
        image: "/img/rules/combat-dice-ink.png",
        alt: "Black-and-white RPG dice, sword, shield, armor, and orc marker illustration",
      },
      type: "announcement",
    },
    {
      key: "news",
      eyebrow: "News",
      title: "What changed recently",
      description:
        "Follow the latest Eldiron releases, workflow improvements, and documentation updates as the project moves toward v1.",
      type: "news",
      items: [
        {
          date: "Oct 4, 2026",
          title: "Interactive Nodes Video",
          href: "https://youtu.be/SYcvM4JLixw",
          thumbnail: {
            image: "https://i.ytimg.com/vi/SYcvM4JLixw/maxresdefault.jpg",
            alt: "Eldiron Interactive Nodes video thumbnail",
          },
        },
        {
          date: "Oct 1, 2026",
          title: "Eldiron v0.95.0",
          description:
            "Create gameplay with Behavior Nodes, configure characters with Entity Nodes, and build shared wall and surface patterns. Live execution highlighting makes your game easier to follow.",
          href: "/blog/2026/10/01/eldiron-v0.95.0",
          linkLabel: "Read more",
        },
        {
          date: "Sep 13, 2026",
          title: "Eldiron v0.94 Feature Overview",
          href: "https://youtu.be/jrbZ6ErD93c",
          thumbnail: {
            image: "https://i.ytimg.com/vi/jrbZ6ErD93c/maxresdefault.jpg",
            alt: "Eldiron v0.94 feature overview video thumbnail",
          },
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
          title: "Interactive Fiction",
          description:
            "Layer narrative metadata onto sectors, linedefs, and entities, and use Eldiron's powerful intent system to build a world model that can be explored entirely through text.",
          image: "/img/screenshots/Eldiron_v0.92_IF.png",
          alt: "Authoring workflow screenshot",
          href: "/docs/creator/authoring",
          linkLabel: "Open docs",
        },
        {
          title: "Nodes",
          description:
            "Build character routines, combat, dialogue, and quests with visual Behavior Nodes. Configure entities and shared construction patterns through nodes, and follow active gameplay with live execution highlighting.",
          image: "/img/eldiron_v0.95.0.png",
          alt: "Eldiron Creator showing an Orc routine with highlighted Behavior Nodes",
          href: "/docs/characters_items/behavior_nodes",
          linkLabel: "Open docs",
        },
        {
          title: "3D Painting",
          description:
            "Paint persistent organic detail directly onto 3D surfaces with varied brushes, generated patterns, material finishes, and anchored vegetation, rubble, and prop stamps.",
          image: "/img/Eldironv0.92.png",
          alt: "3D Painting in Eldiron v0.92.0",
          href: "/docs/creator/tools/iso_paint",
          linkLabel: "Open docs",
        },
        {
          title: "Prefab Tool",
          description:
            "Create, edit, paint, and place reusable linked Prefabs while retaining fast modular construction stamps for rooms, corridors, walls, doorways, stairs, and columns.",
          image: "/img/Eldironv0.92_block.png",
          alt: "Prefab Tool and modular construction workflow",
          href: "/docs/creator/tools/blocks",
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
