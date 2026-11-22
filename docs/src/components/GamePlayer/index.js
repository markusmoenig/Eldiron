import useBaseUrl from "@docusaurus/useBaseUrl";
import { useMemo, useRef, useState } from "react";
import gateCover from "../../../../starters/images/gate-preview.png";
import hideoutCover from "../../../../starters/images/hideout-2d-preview.png";
import stonefallCover from "../../../../starters/images/stonefall-dungeon-preview.png";
import wayfarersRestCover from "../../../../starters/images/wayfarers-rest-preview.png";
import styles from "./styles.module.css";

const games = [
  {
    slug: "hideout",
    name: "Hideout 2D",
    format: "2D top-down",
    description:
      "Recover the key to your treasury, trade with a merchant, ignite a torch, and travel through a portal into the dungeon.",
    image: hideoutCover,
    imageAlt: "Hideout 2D viewed from above in Eldiron",
  },
  {
    slug: "wayfarers-rest",
    name: "Wayfarer’s Rest",
    format: "2D isometric",
    description:
      "Visit the inn and tannery, see the village change after dark, and explore the woods and crypt beyond town.",
    image: wayfarersRestCover,
    imageAlt: "The inn, paths, and trees of Wayfarer’s Rest in 2D Iso",
  },
  {
    slug: "stonefall",
    name: "Stonefall Dungeon",
    format: "Dungeon crawler",
    description:
      "Choose a class, recruit a companion, battle through Stonefall Dungeon, and find the Bone Key that opens the way out.",
    image: stonefallCover,
    imageAlt: "The Stonefall Dungeon example running in Eldiron",
  },
  {
    slug: "gate",
    name: "Gate",
    format: "First-person 3D",
    description:
      "Explore Eldiron from a first-person perspective and see its 3D dungeon rendering, movement, lighting, and interaction systems.",
    image: gateCover,
    imageAlt: "The Gate first-person example running in Eldiron",
  },
];

function EnterIcon() {
  return <span aria-hidden="true">▶</span>;
}

export default function GamePlayer() {
  const [selectedSlug, setSelectedSlug] = useState(games[0].slug);
  const [running, setRunning] = useState(false);
  const [frameVersion, setFrameVersion] = useState(0);
  const fullscreenRef = useRef(null);

  const selectedGame = useMemo(
    () => games.find((game) => game.slug === selectedSlug) ?? games[0],
    [selectedSlug],
  );
  const gameUrl = useBaseUrl(`/play/${selectedGame.slug}/`);

  const selectGame = (slug) => {
    if (slug === selectedSlug) return;
    setSelectedSlug(slug);
    setRunning(false);
    setFrameVersion((version) => version + 1);
  };

  const restartGame = () => {
    setRunning(true);
    setFrameVersion((version) => version + 1);
  };

  const enterFullscreen = async () => {
    if (!fullscreenRef.current?.requestFullscreen) return;
    try {
      await fullscreenRef.current.requestFullscreen();
    } catch (error) {
      console.error("Unable to enter fullscreen mode", error);
    }
  };

  return (
    <section className={styles.gamePlayer} aria-label="Playable Eldiron examples">
      <div className={styles.gameTabs} role="group" aria-label="Choose a game">
        {games.map((game) => (
          <button
            className={`${styles.gameTab} ${
              game.slug === selectedSlug ? styles.gameTabActive : ""
            }`}
            key={game.slug}
            type="button"
            aria-pressed={game.slug === selectedSlug}
            onClick={() => selectGame(game.slug)}
          >
            <span className={styles.gameName}>{game.name}</span>
            <span className={styles.gameFormat}>{game.format}</span>
          </button>
        ))}
      </div>

      <div className={styles.gamePanel}>
        <div className={styles.gameHeading}>
          <div>
            <p className={styles.eyebrow}>Playable in your browser</p>
            <h2>{selectedGame.name}</h2>
            <p>{selectedGame.description}</p>
          </div>
          <div className={styles.gameActions}>
            {running && (
              <button type="button" className={styles.secondaryButton} onClick={restartGame}>
                Restart
              </button>
            )}
            {running && (
              <button
                type="button"
                className={styles.secondaryButton}
                onClick={enterFullscreen}
              >
                Fullscreen
              </button>
            )}
            <a className={styles.secondaryButton} href={gameUrl} target="_blank" rel="noreferrer">
              Open separately
            </a>
          </div>
        </div>

        <div className={styles.viewport} ref={fullscreenRef}>
          {running ? (
            <iframe
              key={`${selectedGame.slug}-${frameVersion}`}
              className={styles.gameFrame}
              src={gameUrl}
              title={`Play ${selectedGame.name}`}
              allow="fullscreen; gamepad"
              allowFullScreen
            />
          ) : (
            <button
              type="button"
              className={styles.gamePreview}
              onClick={() => setRunning(true)}
              aria-label={`Load and play ${selectedGame.name}`}
            >
              <img src={selectedGame.image} alt={selectedGame.imageAlt} />
              <span className={styles.previewShade} />
              <span className={styles.playButton}>
                <EnterIcon />
                Play {selectedGame.name}
              </span>
              <span className={styles.downloadHint}>The game downloads when you press Play</span>
            </button>
          )}
        </div>
      </div>
    </section>
  );
}
