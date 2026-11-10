import Link from "@docusaurus/Link";
import useBaseUrl from "@docusaurus/useBaseUrl";
import { homepageContent } from "./content";
import styles from "./styles.module.css";

function Screenshot({ src, alt, className }) {
  return <img className={className} src={useBaseUrl(src)} alt={alt} />;
}

function SectionHeader({ eyebrow, title, description, id }) {
  return (
    <div className={styles.sectionHeader}>
      <p className={styles.eyebrow}>{eyebrow}</p>
      <h2 id={id}>{title}</h2>
      <p>{description}</p>
    </div>
  );
}

function NewsSection({ section }) {
  return (
    <section className={styles.newsSection}>
      <SectionHeader
        eyebrow={section.eyebrow}
        title={section.title}
        description={section.description}
      />
      <div className={styles.newsGrid}>
        {section.items.map((item) => (
          <article key={item.href} className={styles.newsCard}>
            <p className={styles.newsDate}>{item.date}</p>
            <h3 className={styles.newsTitle}>{item.title}</h3>
            {item.thumbnail && (
              <Link
                className={styles.newsThumbnailLink}
                to={item.href}
                aria-label={item.title}
              >
                <Screenshot
                  className={styles.newsThumbnail}
                  src={item.thumbnail.image}
                  alt={item.thumbnail.alt}
                />
              </Link>
            )}
            {item.description && (
              <p className={styles.newsDescription}>{item.description}</p>
            )}
            {item.linkLabel && (
              <Link className={styles.newsLink} to={item.href}>
                {item.linkLabel}
              </Link>
            )}
          </article>
        ))}
      </div>
    </section>
  );
}

function FormatSection({ section }) {
  return (
    <section className={styles.formatsSection}>
      <SectionHeader
        eyebrow={section.eyebrow}
        title={section.title}
        description={section.description}
      />
      <div className={styles.formatStack}>
        {section.items.map((item) => (
          <article key={item.title} className={styles.formatCard}>
            <div className={styles.formatImageWrap}>
              <Screenshot
                className={styles.formatImage}
                src={item.image}
                alt={item.alt}
              />
            </div>
            <div className={styles.formatCopy}>
              <p className={styles.eyebrow}>{item.eyebrow}</p>
              <h3>{item.title}</h3>
              <p>{item.description}</p>
              <Link to={item.href}>{item.linkLabel}</Link>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

function GeneralFeaturesSection({ section }) {
  return (
    <section className={styles.featuresSection} aria-labelledby="general-features-title">
      <SectionHeader
        id="general-features-title"
        eyebrow={section.eyebrow}
        title={section.title}
        description={section.description}
      />
      <div className={styles.featuresGrid}>
        {section.items.map((item) => (
          <article key={item.title} className={styles.featureCard}>
            <div className={styles.featureArt} aria-hidden="true">
              <Screenshot
                src={item.figure}
                alt=""
                className={styles.featureFigure}
              />
            </div>
            <div className={styles.featureCopy}>
              <h3>{item.title}</h3>
              <p>{item.description}</p>
              <Link to={item.href} className={styles.featureLink}>
                {item.linkLabel} <span aria-hidden="true">→</span>
              </Link>
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

function AnnouncementSection({ section }) {
  return (
    <section className={styles.announcementSection}>
      <div
        className={`${styles.announcementPanel} ${
          section.thumbnail ? "" : styles.announcementPanelTextOnly
        }`}
      >
        <div className={styles.announcementCopy}>
          <p className={styles.announcementEyebrow}>{section.eyebrow}</p>
          <h2>{section.title}</h2>
          <p>{section.description}</p>
          <Link to={section.href}>{section.linkLabel}</Link>
        </div>
        {section.thumbnail && (
          <div className={styles.announcementThumbnailWrap}>
            <Screenshot
              className={styles.announcementThumbnail}
              src={section.thumbnail.image}
              alt={section.thumbnail.alt}
            />
          </div>
        )}
      </div>
    </section>
  );
}

function HomepageSection({ section }) {
  if (section.type === "news") {
    return <NewsSection section={section} />;
  }

  if (section.type === "announcement") {
    return <AnnouncementSection section={section} />;
  }

  if (section.type === "formats") {
    return <FormatSection section={section} />;
  }

  if (section.type === "features") {
    return <GeneralFeaturesSection section={section} />;
  }

  return null;
}

export default function HomepageFeatures() {
  const { hero, sections } = homepageContent;

  return (
    <div className={styles.homepage}>
      <section className={styles.heroSection}>
        <div className={styles.heroCopy}>
          <p className={styles.eyebrow}>{hero.eyebrow}</p>
          <h1 className={styles.heroTitle}>{hero.title}</h1>
          <p className={styles.heroText}>{hero.description}</p>
          <div className={styles.heroActions}>
            {hero.actions.map((action) => (
              <Link key={action.href} className={action.className} to={action.href}>
                {action.label}
              </Link>
            ))}
          </div>
        </div>
        <div className={styles.heroShot}>
          <div className={styles.heroShotHeader}>
            <span>
              {hero.screenshot.label}
              {hero.screenshot.href && hero.screenshot.linkLabel && (
                <>
                  {" "}
                  <Link to={hero.screenshot.href}>{hero.screenshot.linkLabel}</Link>
                </>
              )}
            </span>
            <strong>{hero.screenshot.version}</strong>
          </div>
          <Screenshot
            className={styles.heroImage}
            src={hero.screenshot.image}
            alt={hero.screenshot.alt}
          />
        </div>
      </section>
      {sections.map((section) => (
        <HomepageSection key={section.key} section={section} />
      ))}
    </div>
  );
}
