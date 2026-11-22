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
      label: "Wayfarer’s Rest in 2D Iso.",
      linkLabel: "Read the release post",
      href: "/blog/2026/11/22/eldiron-v0.96.0",
      version: "Eldiron v0.96.0",
      image: "/img/eldiron_v0.96.0.png",
      width: 3324,
      height: 1944,
      alt: "Eldiron Creator v0.96.0 showing Wayfarer’s Rest in 2D Iso, with the village paths, stone bridge, and MaterialZ editor",
    },
  },
  sections: [
    {
      key: "rules-announcement",
      eyebrow: "Official Eldiron Ruleset",
      title: "One ruleset for classic and sandbox RPGs",
      paragraphs: [
        "Our goal is a complete, shared RPG ruleset that creators can pick from. A classic adventure might offer a few classes such as Warrior and Ranger, level progression, and a selection of weapons, armor, and spells.",
        "A sandbox can give players a flexible character who learns skills through play. They can gather wood from trees, learn recipes from townsfolk, craft and repair equipment, and sell what they make. A fighter can take up a trade, and a crafter can go adventuring.",
        "Both styles draw from the same bundled catalogue. Choose what players can find, learn, and use through their starting options and the items, resources, teachers, and encounters you place in the world. A single game can combine both styles.",
        "In Creator, getting started is as simple as picking the classes you want to offer and dragging and dropping characters and items from the project tree onto the map. The bundled ruleset handles how they work.",
      ],
      href: "/docs/official_rules",
      linkLabel: "Read the official rules",
      thumbnail: {
        image: "/img/rules/combat-dice-ink.png",
        alt: "Black-and-white RPG dice, sword, shield, armor, and orc marker illustration",
      },
      type: "announcement",
    },
    {
      key: "multiplayer-announcement",
      eyebrow: "Multiplayer Preview",
      title: "Host a shared world with Eldiron",
      paragraphs: [
        "Dedicated multiplayer is already implemented in Eldiron v0.96. A server runs the shared world and saves character progress and world state between sessions. Players can connect from the desktop or WebGPU browser client, and Creator can export a client package for your server.",
        "We’re preparing a public demo so you can soon join a shared world and explore it with other players.",
      ],
      href: "/docs/creator/public_client_export",
      linkLabel: "Read the multiplayer setup guide",
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
          date: "Nov 22, 2026",
          title: "Eldiron v0.96.0",
          description:
            "Build in 2D Iso with MaterialZ, explore Wayfarer’s Rest, and try world-sourced crafting and the dedicated multiplayer preview.",
          href: "/blog/2026/11/22/eldiron-v0.96.0",
          linkLabel: "Read more",
        },
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
          title: "Shared RPG Rules",
          figure: "/img/lord.png",
          description:
            "Build around the bundled classes, equipment, spells, and recipes. Place resources and stations, and use Behavior Nodes to give characters roles in your world.",
          href: "/docs/rules",
          linkLabel: "Explore the Ruleset",
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
