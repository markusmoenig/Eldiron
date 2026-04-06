import Link from "@docusaurus/Link";
import useBaseUrl from "@docusaurus/useBaseUrl";
import styles from "./styles.module.css";

const newsItems = [
  {
    date: "Mar 15, 2026",
    title: "Eldiron v0.9.1",
    description:
      "Global rules, localization, audio FX, much better realtime visual scripting debugging, and workflow improvements.",
    href: "/blog/2026/03/15/eldiron-v0.9.1",
  },
  {
    date: "Mar 7, 2026",
    title: "Eldiron v0.9",
    description:
      "Avatars, redesigned spellcasting, procedural meshes, audio support, and the core building blocks for v1.",
    href: "/blog/2026_03_07-eldiron-v0.9",
  },
  {
    date: "Feb 7, 2026",
    title: "Eldiron v0.8.100",
    description:
      "Creator workflow improvements, bug fixes, and another step toward a more polished authoring pipeline.",
    href: "/blog/2026/02/07/eldiron-v0.8.100",
  },
];

const formatSections = [
  {
    eyebrow: "2D",
    title: "Build classic top-down adventures",
    description:
      "Draw regions, paint with tiles, script interactions, and build retro RPG spaces with a fast map-making workflow.",
    image: "/img/screenshots/hideout2d_v0.9.png",
    alt: "Eldiron 2D screenshot",
    href: "/docs/building_maps/creating_2d",
    cta: "Explore 2D Workflow",
  },
  {
    eyebrow: "3D",
    title: "Shape dungeons, towns, and terrain in 3D",
    description:
      "Mix sectors, profiles, terrain, materials, and tile painting to create first-person or isometric worlds without a separate 3D toolchain.",
    image: "/img/screenshots/dungeon3d_v0.9.png",
    alt: "Eldiron 3D screenshot",
    href: "/docs/building_maps/creating_3d_maps",
    cta: "Explore 3D Workflow",
  },
];

const toolSections = [
  {
    title: "Dungeon Tool",
    description:
      "Block out rooms, corridors, shafts, and doors in a conceptual grid, then let Eldiron generate editable map geometry from it.",
    image: "/img/screenshots/dungeon3d_iso.png",
    alt: "Dungeon Tool preview",
    href: "/docs/creator/tools/dungeon",
  },
  {
    title: "Interactive Fiction",
    description:
      "Layer narrative metadata onto sectors, linedefs, characters, and items so the same world can drive text-forward play as well as 2D and 3D presentation.",
    image: "/img/docs/screenshot.png",
    alt: "Authoring workflow screenshot",
    href: "/docs/creator/authoring",
  },
  {
    title: "Tile Picker",
    description:
      "Browse project tiles, groups, collections, and treasury content from one dock, then push directly into editing or assignment workflows.",
    image: "/img/screenshots/tilesets.png",
    alt: "Tile Picker screenshot",
    href: "/docs/creator/docks/tile_picker_editor",
  },
  {
    title: "TileGraph",
    description:
      "Author procedural tile groups with reusable node graphs, layered materials, and generated outputs that behave like first-class assets.",
    image: "/img/creator_nodes_v0870.png",
    alt: "TileGraph screenshot",
    href: "/docs/creator/docks/tile_node_graph",
  },
];

function Screenshot({ src, alt, className }) {
  return <img className={className} src={useBaseUrl(src)} alt={alt} />;
}

export default function HomepageFeatures() {
  return (
    <div className={styles.homepage}>
      <section className={styles.heroSection}>
        <div className={styles.heroCopy}>
          <p className={styles.eyebrow}>Retro RPG Creator</p>
          <h1 className={styles.heroTitle}>Build retro worlds in 2D, 3D, and text-first formats</h1>
          <p className={styles.heroText}>
            Eldiron is a game creator for classic RPGs. One editor brings together map
            building, tile workflows, visual scripting, narrative authoring, and
            cross-platform play.
          </p>
          <div className={styles.heroActions}>
            <Link className="button button--primary button--lg" to="/docs/getting_started">
              Getting Started
            </Link>
            <Link className="button button--secondary button--lg" to="/blog">
              Read Dev Updates
            </Link>
          </div>
        </div>
        <div className={styles.heroShot}>
          <div className={styles.heroShotHeader}>
            <span>Current main version</span>
            <strong>Eldiron v0.9.1</strong>
          </div>
          <Screenshot
            className={styles.heroImage}
            src="/img/screenshots/Eldiron_v0.9.png"
            alt="Eldiron main version screenshot"
          />
        </div>
      </section>

      <section className={styles.newsSection}>
        <div className={styles.sectionHeader}>
          <p className={styles.eyebrow}>News</p>
          <h2>What changed recently</h2>
          <p>
            A quick snapshot of the latest releases and documentation updates.
          </p>
        </div>
        <div className={styles.newsGrid}>
          {newsItems.map((item) => (
            <article key={item.href} className={styles.newsCard}>
              <p className={styles.newsDate}>{item.date}</p>
              <h3>{item.title}</h3>
              <p>{item.description}</p>
              <Link to={item.href}>Read more</Link>
            </article>
          ))}
        </div>
      </section>

      <section className={styles.formatsSection}>
        <div className={styles.sectionHeader}>
          <p className={styles.eyebrow}>World Building</p>
          <h2>Choose the presentation that fits your game</h2>
          <p>
            The homepage should sell the bigger picture: Eldiron is not one isolated
            editor mode, but a connected toolset for several RPG styles.
          </p>
        </div>
        <div className={styles.formatStack}>
          {formatSections.map((section) => (
            <article key={section.title} className={styles.formatCard}>
              <div className={styles.formatImageWrap}>
                <Screenshot
                  className={styles.formatImage}
                  src={section.image}
                  alt={section.alt}
                />
              </div>
              <div className={styles.formatCopy}>
                <p className={styles.eyebrow}>{section.eyebrow}</p>
                <h3>{section.title}</h3>
                <p>{section.description}</p>
                <Link to={section.href}>{section.cta}</Link>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className={styles.toolsSection}>
        <div className={styles.sectionHeader}>
          <p className={styles.eyebrow}>Key Tools</p>
          <h2>Focused workflows inside the editor</h2>
          <p>
            Instead of a long feature dump, these are the systems that define how
            Eldiron feels to use.
          </p>
        </div>
        <div className={styles.toolsGrid}>
          {toolSections.map((tool) => (
            <article key={tool.title} className={styles.toolCard}>
              <Screenshot className={styles.toolImage} src={tool.image} alt={tool.alt} />
              <div className={styles.toolCopy}>
                <h3>{tool.title}</h3>
                <p>{tool.description}</p>
                <Link to={tool.href}>Open docs</Link>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
