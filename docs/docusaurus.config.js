// @ts-check
// `@type` JSDoc annotations allow editor autocompletion and type checking
// (when paired with `@ts-check`).
// There are various equivalent ways to declare your Docusaurus config.
// See: https://docusaurus.io/docs/api/docusaurus-config

const gruvboxLightTheme = {
  plain: {
    color: "#3c3836",
    backgroundColor: "#fbf1c7",
  },
  styles: [
    {
      types: ["comment", "prolog", "doctype", "cdata"],
      style: { color: "#928374", fontStyle: "italic" },
    },
    {
      types: ["punctuation"],
      style: { color: "#7c6f64" },
    },
    {
      types: ["namespace"],
      style: { color: "#8f3f71" },
    },
    {
      types: ["number", "boolean", "constant", "symbol", "deleted"],
      style: { color: "#d3869b" },
    },
    {
      types: ["property"],
      style: { color: "#5d9ad6" },
    },
    {
      types: ["attr-name", "selector"],
      style: { color: "#fabd2f" },
    },
    {
      types: ["string", "char", "builtin", "inserted"],
      style: { color: "#d6a46f" },
    },
    {
      types: ["operator", "entity", "url"],
      style: { color: "#d79921" },
    },
    {
      types: ["atrule", "attr-value", "keyword"],
      style: { color: "#dcd08a" },
    },
    {
      types: ["function"],
      style: { color: "#5d9ad6", fontWeight: "bold" },
    },
    {
      types: ["class-name"],
      style: { color: "#fabd2f" },
    },
    {
      types: ["builtin", "support", "support.function"],
      style: { color: "#6aa2e8" },
    },
    {
      types: ["variable"],
      style: { color: "#8fbf8f" },
    },
    {
      types: ["important", "bold"],
      style: { fontWeight: "bold" },
    },
    {
      types: ["italic"],
      style: { fontStyle: "italic" },
    },
  ],
};

const gruvboxDarkTheme = {
  plain: {
    color: "#ebdbb2",
    backgroundColor: "#282828",
  },
  styles: [
    {
      types: ["comment", "prolog", "doctype", "cdata"],
      style: { color: "#928374", fontStyle: "italic" },
    },
    {
      types: ["punctuation"],
      style: { color: "#a89984" },
    },
    {
      types: ["namespace"],
      style: { color: "#d3869b" },
    },
    {
      types: ["number", "boolean", "constant", "symbol", "deleted"],
      style: { color: "#d3869b" },
    },
    {
      types: ["property"],
      style: { color: "#5d9ad6" },
    },
    {
      types: ["attr-name", "selector"],
      style: { color: "#fabd2f" },
    },
    {
      types: ["string", "char", "builtin", "inserted"],
      style: { color: "#d6a46f" },
    },
    {
      types: ["operator", "entity", "url"],
      style: { color: "#fabd2f" },
    },
    {
      types: ["atrule", "attr-value", "keyword"],
      style: { color: "#dcd08a" },
    },
    {
      types: ["function"],
      style: { color: "#5d9ad6", fontWeight: "bold" },
    },
    {
      types: ["class-name"],
      style: { color: "#fabd2f" },
    },
    {
      types: ["builtin", "support", "support.function"],
      style: { color: "#6aa2e8" },
    },
    {
      types: ["variable"],
      style: { color: "#8fbf8f" },
    },
    {
      types: ["important", "bold"],
      style: { fontWeight: "bold" },
    },
    {
      types: ["italic"],
      style: { fontStyle: "italic" },
    },
  ],
};

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

const isGhPages = process.env.DOCS_GH_PAGES === "1";
const isProd = process.env.NODE_ENV === "production";

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: "Eldiron",
  tagline: "Retro RPG Creator",
  favicon: "img/favicon.svg",

  // Set the production url of your site here
  url: isGhPages ? "https://markusmoenig.github.io" : "https://eldiron.com",
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: isGhPages ? "/Eldiron/" : "/",

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: "markusmoenig", // Usually your GitHub org/user name.
  projectName: "Eldiron-Docs", // Usually your repo name.

  onBrokenLinks: "warn",
  onBrokenMarkdownLinks: "warn",

  // Even if you don't use internationalization, you can use this field to set
  // useful metadata like html lang. For example, if your site is Chinese, you
  // may want to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  headTags: [
    {
      tagName: "link",
      attributes: {
        rel: "stylesheet",
        href: "https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css",
      },
    },
  ],

  presets: [
    [
      "classic",
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: "./sidebars.js",
        },
        blog: {
          showReadingTime: true,
          feedOptions: {
            type: ["rss", "atom"],
            xslt: true,
          },
          // Useful options to enforce blogging best practices
          onInlineTags: "warn",
          onInlineAuthors: "warn",
          onUntruncatedBlogPosts: "warn",
        },
        theme: {
          customCss: "./src/css/custom.css",
        },
        ...(isProd
          ? {
              gtag: {
                trackingID: "G-35R29223CG",
                anonymizeIP: true,
              },
            }
          : {}),
      }),
    ],
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      // Replace with your project's social card
      image: "img/eldiron-banner.png",
      colorMode: {
        respectPrefersColorScheme: true,
      },
      navbar: {
        title: "Retro RPG Creator",
        logo: {
          alt: "Eldiron Logo",
          src: "img/logo-black.svg",
          srcDark: "img/logo-white.svg",
        },
        items: [
          {
            type: "docSidebar",
            sidebarId: "tutorialSidebar",
            position: "left",
            label: "Docs",
          },
          { to: "/blog", label: "Blog", position: "left" },
          { to: "/intro", label: "History", position: "left" },
          { to: "/sponsor", label: "Sponsor", position: "left" },
          { to: "/games", label: "Games", position: "left" },
          {
            type: "html",
            position: "right",
            value: `
              <a href="https://discord.gg/ZrNj6baSZU" class="navbar-icon" title="Eldiron Discord">
                <img src="https://img.shields.io/badge/Discord-Join%20Server-458588?style=flat&logo=discord" alt="Join Discord"/>
              </a>
            `,
          },
          {
            type: "html",
            position: "right",
            value: `
              <a href="https://www.patreon.com/eldiron" class="navbar-icon" title="Support on Patreon">
                <img src="https://img.shields.io/badge/Patreon-Support-458588?style=flat&logo=patreon" alt="Support on Patreon"/>
              </a>
            `,
          },
          {
            type: "html",
            position: "right",
            value: `
              <a href="https://www.youtube.com/channel/UCCmrO356zLQv_m8dPEqBUfA" class="navbar-icon" title="Eldiron YouTube Channel">
                <img src="https://img.shields.io/youtube/channel/subscribers/UCCmrO356zLQv_m8dPEqBUfA?style=flat&color=458588&logo=youtube&label=Subscribe" alt="YouTube subscribers"/>
              </a>
            `,
          },
          {
            type: "html",
            position: "right",
            value: `
              <a href="https://github.com/markusmoenig/Eldiron" class="navbar-icon" title="GitHub Repository">
                <img src="https://img.shields.io/github/stars/markusmoenig/Eldiron?style=flat&color=458588&logo=github" alt="GitHub stars"/>
              </a>
            `,
          },
          {
            type: "html",
            position: "right",
            value: `
              <a href="https://github.com/markusmoenig/Eldiron/releases" class="navbar-icon" title="Download Eldiron">
                <i class="fas fa-download"></i>
              </a>
            `,
          },
        ],
      },
      footer: {
        style: "dark",
        links: [
          {
            title: "Community",
            items: [
              {
                label: "YouTube",
                to: "https://www.youtube.com/channel/UCCmrO356zLQv_m8dPEqBUfA",
              },
              {
                label: "Discord",
                to: "https://discord.gg/ZrNj6baSZU",
              },
              {
                label: "Bluesky",
                to: "https://bsky.app/profile/markusmoenig.bsky.social",
              },
              {
                label: "X",
                to: "https://x.com/EldironRPG",
              },
            ],
          },
          {
            title: "Sponsor",
            items: [
              {
                label: "Patreon",
                to: "https://patreon.com/eldiron",
              },
              {
                label: "GitHub Sponsors",
                to: "https://github.com/markusmoenig",
              },
              {
                label: "PayPal",
                to: "https://paypal.me/markusmoenigos",
              },
            ],
          },
          {
            title: "Links",
            items: [
              {
                label: "Getting Started",
                to: "/docs/getting_started",
              },
              {
                label: "Downloads",
                to: "https://github.com/markusmoenig/Eldiron/releases",
              },
              {
                label: "GitHub",
                to: "https://github.com/markusmoenig/Eldiron",
              },
            ],
          },
        ],
        copyright: `Copyright © ${new Date().getFullYear()} Markus Moenig`,
      },
      prism: {
        theme: gruvboxLightTheme,
        darkTheme: gruvboxDarkTheme,
        additionalLanguages: ["toml"],
      },
    }),
};

export default config;
