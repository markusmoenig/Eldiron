---
title: "Public Client Export"
sidebar_position: 2.5
---

Use a **.eldiron-client** release to let players connect to a dedicated
multiplayer world. The Player package carries public assets and connection
settings. The host uses the authored **.eldiron** project.

## Export from Creator

1. Open the authored multiplayer project and set its public `wss://`
   endpoint in **Game / Settings**.
2. Choose **File → Export Public Client…** and save the
   `.eldiron-client` release.
3. Open the release in the desktop Player. Check that it connects to the
   intended host and reaches its lobby.

## Deploy the browser Player

From an Eldiron source checkout, run the browser packager with the path to
your authored project and a new output directory:

```sh
python3 tools/package_web.py player /tmp/my-public-player --offline \
  --project private/game.eldiron
```

The packager exports `game.eldiron-client` and configures the browser
Player to load it. Serve the output over HTTPS and allow the page’s origin
on the dedicated host. Keep the authored project, world saves and accounts
in private host storage.

For release directories with on-demand regional artwork, add `--streamed`
and publish the entire output directory, including its `packs/` directory.
Use a new release URL for each published build.
