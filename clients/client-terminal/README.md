# Eldiron Client Terminal

A terminal-based player client for games created with Eldiron. Play your RPGs in a classic text-adventure style directly from the command line.

## Features

- Text-based room descriptions and navigation
- ANSI color support for entities, items, and messages
- Auto-attack mode for combat
- Command aliases and intent resolution
- Full Rusterix engine integration with script support

## Installation

```bash
cargo install eldiron-client-terminal
```

## Usage

```bash
eldiron-client-terminal path/to/game.eldiron
```

If no path is provided, it searches for a `.eldiron` file in the current directory.

## Configuration

The terminal client respects the game's `authoring` configuration for:

- **Colors**: Customize entity, item, and message colors via `[colors]` in the authoring TOML
- **Character color rules**: Conditional coloring based on entity attributes (e.g., hostility level)
- **Auto-attack mode**: Configured via `[combat].auto_attack`
- **Command aliases**: Define aliases in `[alias]` section
- **Startup display**: Control what's shown on startup via `[startup].show` (`description`, `room`, or `none`)
- **Welcome message**: Set via `[startup].welcome`

## Example Commands

Once running, you can use standard text adventure commands:
- Movement: `north`, `south`, `east`, `west`, `up`, `down`
- Actions: `attack <target>`, `take <item>`, `use <item>`, `talk to <character>`
- Info: `look`, `inventory`, `help`

## Platform Support

Works on all platforms (macOS, Linux, Windows) with a compatible terminal emulator.
