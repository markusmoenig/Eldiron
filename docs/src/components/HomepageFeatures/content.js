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
      linkLabel: "Read the release post.",
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
        "Choose how to build and explore your world. All modes share characters, items, rules, and visual Behavior Nodes.",
      type: "formats",
      items: [
        {
          eyebrow: "2D",
          title: "Build classic top-down adventures",
          description:
            "Import a tileset or create procedural MaterialZ materials, then paint them onto your region maps with the Rect tool. Place characters and items, configure them with Entity Nodes, and build interactions, dialogue, and combat with Behavior Nodes.",
          image: "/img/creator/hideout2d-v096.png",
          alt: "Hideout 2D in Eldiron Creator v0.96.0, with the harbor, garden, characters, and Tile Picker",
          href: "/docs/building_maps/creating_2d",
          linkLabel: "Explore 2D Workflow",
        },
        {
          eyebrow: "2D Iso · New in v0.96",
          title: "Build isometric worlds with tilesets and MaterialZ",
          description:
            "New in v0.96, 2D Iso brings tile placement to an isometric grid. Import isometric tilesets or use procedural MaterialZ materials, then paint ground and place scenery with the Rect tool. Shape hills and plateaus, paint paths over the terrain, and add characters, items, and Behavior Nodes.",
          image: "/img/iso/wayfarers-rest-v096.png",
          alt: "Eldiron Creator v0.96.0 showing Wayfarer's Rest in 2D Iso, with a timber inn, trees, a well, painted paths, and shaped terrain",
          href: "/docs/building_maps/creating_2d#starting-an-isometric-project",
          linkLabel: "Explore 2D Iso Workflow",
        },
        {
          eyebrow: "3D",
          title: "Shape dungeons, towns, and terrain in 3D",
          description:
            "Start with simple shapes and build your world by hand using the Object, Face, Edge, and Vertex tools. Extrude walls, cut doorways, sculpt terrain, and apply tiles or MaterialZ materials to surfaces. Place characters and items, add Behavior Nodes, and play in first person or from an isometric camera.",
          image: "/img/creator/stonefall-first-person-v096.png",
          alt: "Stonefall Dungeon first-person gameplay in Eldiron Creator v0.96.0, showing a torchlit dungeon corridor and the game HUD",
          href: "/docs/building_maps/creating_3d_maps",
          linkLabel: "Explore 3D Workflow",
        },
        {
          eyebrow: "Text",
          title: "Build text-based adventures in the same world",
          description:
            "Draw named areas and connections on your region maps, then write descriptions for places, characters, and items in the Authoring dock. Configure player commands with Entity Nodes and create reactions with Behavior Nodes. Play through the terminal client with commands such as look, take, and talk.",
          image: "/img/screenshots/Eldiron_v0.92_CLI.png",
          alt: "Hideout 2D played in the Eldiron terminal client",
          href: "/docs/creator/authoring",
          linkLabel: "Explore Text Workflow",
        },
      ],
    },
    {
      key: "general-features",
      eyebrow: "Tools for Your World",
      title: "General Features",
      description:
        "Build on a shared set of creative tools and gameplay systems. Materials, painting, and terrain tools support the visual workflows; nodes, rules, authoring, and localization also carry through to text play.",
      type: "features",
      items: [
        {
          title: "Entity & Behavior Nodes",
          figure: "/img/dragonslayer.png",
          description:
            "Configure characters, equipment, items, and player input with Entity Nodes. Connect events and actions in Behavior Nodes to create NPC routines, dialogue, quests, and interactions, with live feedback while your game runs.",
          href: "/docs/characters_items/behavior_nodes",
          linkLabel: "Explore Nodes",
        },
        {
          title: "Editable Screens & Built-in UI",
          figure: "/img/lord.png",
          description:
            "Design and customize game screens with Eldiron's map tools. The built-in UI already understands inventories, equipment, spells, and character actions, drawing icons, descriptions, costs, and cooldowns from your game data and ruleset. Adjust responsive layouts and artwork, or create your own widgets.",
          href: "/docs/screens/",
          linkLabel: "Explore Screens & UI",
        },
        {
          title: "Editable RPG Rules",
          figure: "/img/lord.png",
          description:
            "Start with the bundled fantasy ruleset for races, classes, equipment, combat, spells, and progression. Edit definitions through the Rules editor so every character and client uses the same gameplay rules.",
          href: "/docs/creator/docks/ruleset_nodes",
          linkLabel: "Explore the Ruleset Editor",
        },
        {
          title: "MaterialZ Materials",
          figure: "/img/mage.png",
          description:
            "Create editable procedural materials by combining shapes, noise, colors, and imported images. Layer and blend them into roads, floors, walls, and terrain, with shared palette colors and reusable recipes.",
          href: "/docs/building_maps/procedural_materials",
          linkLabel: "Explore MaterialZ",
        },
        {
          title: "Terrain Tools",
          figure: "/img/farmer.png",
          description:
            "Raise, lower, and smooth the landscape with a brush, or draw editable guides for slopes, hills, and plateaus. Procedural ground materials and painted paths follow the terrain as you reshape it.",
          href: "/docs/building_maps/creating_3d_terrain",
          linkLabel: "Explore Terrain Tools",
        },
        {
          title: "Paint Brush",
          figure: "/img/adventurer.png",
          description:
            "Paint moss, dirt, cracks, puddles, and other details directly onto isometric terrain or 3D surfaces. Adjust brush size, opacity, and palette colors, or scatter organic stamps such as grass, flowers, and rubble.",
          href: "/docs/creator/tools/iso_paint",
          linkLabel: "Explore the Paint Brush",
        },
        {
          title: "Automatic Chunk Streaming",
          figure: "/img/adventurer.png",
          description:
            "Build large regions without manually dividing them into separate maps. Eldiron loads nearby scene chunks, prepares surrounding chunks in the background, and unloads distant geometry as the player moves. This keeps rendering focused on the area being explored.",
          href: "/docs/screens/widgets#game-widgets",
          linkLabel: "Explore Region Streaming",
        },
        {
          title: "Simulation Modes",
          figure: "/img/adventurer.png",
          description:
            "Choose realtime play, turn-based steps, or a hybrid that advances on player actions and after an idle timeout. Set the pace independently of your game's camera or presentation.",
          href: "/docs/configuration/game#game-configuration",
          linkLabel: "Explore Game Settings",
        },
        {
          title: "Authoring & Text Play",
          figure: "/img/mage.png",
          description:
            "Give places, connections, characters, and items names and descriptions in the Authoring dock. Use that text for looking around in graphical games or exploring the same authored world through the terminal client.",
          href: "/docs/creator/authoring",
          linkLabel: "Explore Authoring",
        },
        {
          title: "Localization",
          figure: "/img/lord.png",
          description:
            "Keep dialogue, quest text, combat feedback, and system messages in shared language tables. Reuse translated messages in rules and Behavior Nodes, with placeholders for character names, damage, and other game values.",
          href: "/docs/localization",
          linkLabel: "Explore Localization",
        },
      ],
    },
  ],
};
