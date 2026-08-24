export const homepageContent = {
  hero: {
    eyebrow: "Retro RPG Creator",
    title: "Build retro RPG worlds for 2D, 3D, and interactive fiction",
    description:
      "Eldiron is a game creator for classic RPGs. One editor brings together map building, tile workflows, Eldrin scripting, narrative authoring, and cross-platform play.",
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
      label: "Stonefall Dungeon.",
      linkLabel: "See release.",
      href: "/blog/2026/08/15/eldiron-v0.93.0",
      version: "Eldiron v0.93.0",
      image: "/img/Eldironv0.93_stonefall.png",
      alt: "Eldiron v0.93.0 Stonefall Dungeon battle with the Ranger and Alden",
    },
  },
  sections: [
    {
      key: "rules-announcement",
      eyebrow: "NOW AVAILABLE",
      title: "Official Eldiron Ruleset",
      description:
        "Eldiron v0.93.0 turns rulesets into project-selectable packages and expands the official fantasy rules from levels 1–10. Races, classes, actions, spells, conditions, equipment, progression, resources, and crafting share one runtime while projects remain free to extend or replace the rules they need.",
      href: "/docs/official_rules",
      linkLabel: "Read the official rules",
      thumbnail: {
        image: "/img/rules/combat-dice-ink.png",
        alt: "Black-and-white RPG dice, sword, shield, armor, and orc marker illustration",
      },
      type: "announcement",
    },
    {
      key: "artists-call",
      eyebrow: "Artists wanted",
      title: "Help shape the look of Eldiron",
      description:
        "Showing Eldiron at its best is becoming increasingly difficult without more high-quality artwork. I'm especially looking for artists who can contribute 2D artwork for sample projects and rulesets, as well as 3D artists interested in offering design guidance or creating models directly within Eldiron. If you would like to help, please get in touch on Discord. Contributors will receive visible credit here on the website.",
      href: "https://discord.gg/ZrNj6baSZU",
      linkLabel: "Get in touch on Discord",
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
          date: "Aug 15, 2026",
          title: "Eldiron v0.93.0",
          description:
            "Build with selectable ruleset packages and procedural recipes, use Actions and Words of Power, and explore Stonefall as a Dungeon Master-style Source project.",
          href: "/blog/2026/08/15/eldiron-v0.93.0",
          linkLabel: "Read more",
        },
        {
          date: "Jul 16, 2026",
          title: "3D Painting Video",
          href: "https://www.youtube.com/watch?v=6xSJTX0ES54",
          thumbnail: {
            image: "https://i.ytimg.com/vi/6xSJTX0ES54/maxresdefault.jpg",
            alt: "Eldiron v0.92.0 3D Painting video thumbnail",
          },
        },
        {
          date: "Jul 15, 2026",
          title: "Eldiron v0.92.0",
          description:
            "Paint persistent detail directly onto 3D surfaces, add generated patterns and stamps, and build editable rooms and dungeons quickly with the new Block Tool.",
          href: "/blog/2026/07/15/eldiron-v0.92.0",
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
