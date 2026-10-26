---
title: "AI for Text Play"
---

The terminal client can interpret natural language while using the existing text
runtime. Rooms, connections, characters, items, input mappings, rules and behavior
remain authored through the normal Creator tools. No additional map model is
required. AI support is enabled through the shared crate’s optional `text-ai`
feature, which the terminal enables; the Creator does not require it.

## Start with ordinary text play

```sh
cargo run --offline -p eldiron-client-terminal -- game.eldiron --mode text --ai-provider disabled
```

Existing commands such as `look`, `inventory`, `go north`, and authored intents
keep their existing parser and runtime behavior. Roguelike mode is unchanged.

## Reuse a ChatGPT account

The account store is the same host-local store used by the previous AI support.
An existing saved sign-in and selected model can be reused. Otherwise run:

```sh
cargo run --offline -p eldiron-client-terminal -- auth login
cargo run --offline -p eldiron-client-terminal -- auth models
cargo run --offline -p eldiron-client-terminal -- auth model MODEL_SLUG
cargo run --offline -p eldiron-client-terminal -- game.eldiron --mode text --ai-provider chatgpt
```

`auth status`, `auth accounts`, `auth use ACCOUNT_UUID`, and `auth logout` manage
local accounts. Sign-in happens only through the explicit `auth login` command.
Credentials are kept outside projects and exports.

## Other providers

Set `ELDIRON_AI_MODEL`, `ELDIRON_AI_BASE_URL`, and, if required,
`ELDIRON_AI_API_KEY`, then start with `--ai-provider environment`. The endpoint
must support the Chat Completions JSON response contract. HTTPS is required for
remote servers; HTTP is supported for loopback services.

Without a configured provider, text play keeps its ordinary command behavior.
`--ai-provider disabled` explicitly disables AI even when environment settings
are present. AI interpretation is available in text mode.

## Play

Describe one action naturally, such as “I want to ask the guard about this
place.” The available actions come from the current visible scene and the
player's existing input mappings. Ambiguous requests can produce a clarification.
The game executes the selected action through its original command handlers.
The actual game result is printed immediately, followed by optional AI narration.

Explicit commands remain available while a request is pending. They discard the
pending reply and use the existing parser. In the interactive terminal, `cancel`
(or `/cancel`) discards a pending AI reply. Completed game actions remain applied.
Provider failures, incomplete responses and stale replies do not replay actions.

Only public room text, visible targets, exits, inventory and displayed player
stats are sent as scene context. Private authoring/source data is not included.
The AI cannot add exits, invoke unavailable intents, load files or change rules.
Requests have size and time limits, and their workers do not own game state.

AI settings and conversation history are not serialized into project files. The
existing project formats and game runtime remain unchanged.
