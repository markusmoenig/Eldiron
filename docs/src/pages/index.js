import Layout from "@theme/Layout";
import HomepageFeatures from "@site/src/components/HomepageFeatures";
import useBaseUrl from "@docusaurus/useBaseUrl";

export default function Home() {
  const bannerUrl = useBaseUrl("img/eldiron-banner.png");

  return (
    <Layout
      title={`Retro RPG Creator`}
      description="Eldiron is a retro RPG game creator for 2D, isometric, and first-person adventures. Build your own classic RPG worlds with powerful tools and cross-platform freedom."
    >
      <main>
        <HomepageFeatures />

        {/* Sponsor Thank You Section */}
        <section
          style={{ textAlign: "center", marginTop: "3rem", padding: "0 1rem" }}
        >
          <h2 style={{ color: "var(--ifm-color-primary)" }}>
            Thanks to Our Supporters
          </h2>
          <p
            style={{
              fontSize: "1.05rem",
              maxWidth: "700px",
              margin: "0 auto 1rem",
            }}
          >
            A heartfelt thank you to everyone supporting Eldiron via Patreon and
            GitHub Sponsors. Your support helps me keep building and improving
            this project.
          </p>
          <div
            style={{
              fontWeight: "bold",
              marginBottom: "0.5rem",
              color: "var(--ifm-color-primary)",
              fontSize: "1.2rem",
            }}
          >
            Patreon Supporters
          </div>

          <div
            style={{
              fontWeight: "bold",
              marginBottom: "0.5rem",
              fontSize: "0.95rem",
              color: "var(--ifm-color-primary)",
            }}
          >
            Lord:
          </div>
          <div style={{ marginBottom: "1rem" }}>—</div>

          <div
            style={{
              fontWeight: "bold",
              marginBottom: "0.5rem",
              fontSize: "0.95rem",
              color: "var(--ifm-color-primary)",
            }}
          >
            Dragon Slayer:
          </div>
          <div style={{ marginBottom: "1rem" }}>
            Scott Hamill, Omer Golan-Joel, Mike Plaza
          </div>

          <div
            style={{
              fontWeight: "bold",
              marginBottom: "0.5rem",
              fontSize: "0.95rem",
              color: "var(--ifm-color-primary)",
            }}
          >
            Adventurer:
          </div>
          <div style={{ marginBottom: "1rem" }}>
            Charleston Marks, Elias, Thomas Osborne, Kendric Tonn
          </div>

          <div
            style={{
              fontWeight: "bold",
              marginBottom: "0.5rem",
              fontSize: "0.95rem",
              color: "var(--ifm-color-primary)",
            }}
          >
            Farmer:
          </div>
          <div style={{ marginBottom: "1rem" }}>
            Jonathan Picket, Tom Carlson
          </div>

          <div
            style={{
              fontWeight: "bold",
              marginBottom: "0.5rem",
              color: "var(--ifm-color-primary)",
              fontSize: "1.2rem",
            }}
          >
            GitHub Sponsors
          </div>
          <div>rijupahwa, cnasc</div>
        </section>

        {/* Banner at the bottom */}
        <section style={{ padding: "2rem 1rem", textAlign: "center" }}>
          <img
            src={bannerUrl}
            alt="Eldiron Banner"
            style={{
              display: "block",
              margin: "0 auto",
              maxWidth: "1000px",
              width: "100%",
              height: "auto",
            }}
          />
        </section>
      </main>
    </Layout>
  );
}
