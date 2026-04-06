import Link from "@docusaurus/Link";
import useBaseUrl from "@docusaurus/useBaseUrl";
import { homepageContent } from "./content";
import styles from "./styles.module.css";

function Screenshot({ src, alt, className }) {
  return <img className={className} src={useBaseUrl(src)} alt={alt} />;
}

function SectionHeader({ eyebrow, title, description }) {
  return (
    <div className={styles.sectionHeader}>
      <p className={styles.eyebrow}>{eyebrow}</p>
      <h2>{title}</h2>
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
            <h3>{item.title}</h3>
            <p>{item.description}</p>
            <Link to={item.href}>{item.linkLabel}</Link>
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

function ToolsSection({ section }) {
  return (
    <section className={styles.toolsSection}>
      <SectionHeader
        eyebrow={section.eyebrow}
        title={section.title}
        description={section.description}
      />
      <div className={styles.toolsGrid}>
        {section.items.map((item) => (
          <article key={item.title} className={styles.toolCard}>
            <div className={styles.toolImageWrap}>
              <Screenshot className={styles.toolImage} src={item.image} alt={item.alt} />
            </div>
            <div className={styles.toolCopy}>
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

function HomepageSection({ section }) {
  if (section.type === "news") {
    return <NewsSection section={section} />;
  }

  if (section.type === "formats") {
    return <FormatSection section={section} />;
  }

  if (section.type === "tools") {
    return <ToolsSection section={section} />;
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
            <span>{hero.screenshot.label}</span>
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
