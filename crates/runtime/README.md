# Eldiron runtime

`GameSession` owns one local CPU game: project data, engine assets, server,
current region, simulation clock and pending server outputs. It creates no
window, GPU, audio device, UI context or font layout engine. The terminal uses
this owner; graphical hosts will borrow the same session from presentation.

- `start` validates configuration, prepares authoritative rules/templates/maps,
  creates region workers and queues the configured local player.
- `advance(elapsed)` schedules simulation steps independently of render rate.
  Catch-up is bounded per call and retains remaining debt. Pausing clears wall
  time debt. `poll` receives worker results without advancing simulation.
- `tick_once` supports command-driven clients and explicit startup settling.
- One collector drains engine messages, speech, choices, audio requests, world
  text and container requests into region mailboxes. A consumer calls
  `take_events`, or takes only messages/speech. Event interests discard channels
  that a host does not consume. Collection is separate from presentation.
- `stop` and dropping a started session shut down its server. A session that
  was merely constructed does not clear global engine state.

CPU asset preparation currently lives in `shared::runtime_assets` so existing
Creator/client adapters can use it without a dependency cycle. Graphics-specific
invalidations remain in `shared::rusterix_utils`. This is a migration boundary;
application startup adapters and graphical presentation still need adoption.

Remaining architecture work: the engine's region/player routing registries are
still process globals, so multiple concurrently running local sessions are not
isolated. Legacy CPU bitmap/value carriers still come through the UI-disabled
framework crate; moving their persisted storage into the CPU data layer remains
part of the rewrite. These limitations do not imply GPU/UI initialization.

Verify the independent boundary with:

```sh
python3 tools/check_renderer_boundaries.py
cargo test --offline -p eldiron-runtime -- --test-threads=1
cargo test --offline -p eldiron-client-terminal
```

The single test thread reflects the remaining global engine registries, not a
renderer requirement. Startup requests are asynchronous; hosts continue polling
while region/player registration completes.
