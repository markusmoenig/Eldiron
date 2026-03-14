---
title: "Audio"
sidebar_position: 7
---

This page collects all audio-related workflow and scripting in one place.

## Overview

Audio in Eldiron has three main parts:

- **Assets**: Import and manage audio files in the project tree.
- **Audio FX**: Define small generated sound effects in `Game / Audio FX`.
- **Rules integration**: Trigger combat audio from `Game / Rules`.
- **Runtime commands**: Play, stop and mix audio buses from server scripts.

See also:

- [Project Tree: Assets](creator/project_tree#assets)
- [Server Commands](characters_items/server_commands)

## Audio Assets

Audio files are managed in the **Assets** section of the project tree.

Supported formats:

- **WAV**
- **OGG**

To import audio:

1. In the project tree, open the **Assets** section.
2. Click **+**.
3. Select **Add Audio Asset**.
4. Choose a `.wav` or `.ogg` file.

The asset name is what you use in scripts, for example:

```eldrin
play_audio("battle_theme")
```

## Audio FX

Small procedural effects can be defined in **Game / Audio FX** as TOML.

These effects are loaded like normal clips, so you use them with the same command:

```eldrin
play_audio("door_open")
play_audio("fire_cast")
```

Example:

```toml
[sfx.door_open]
wave = "noise"
duration = 0.12
attack = 0.002
decay = 0.10
release = 0.02
gain = 0.35
freq = 900
freq_end = 280
noise = 0.85
lowpass = 0.45
```

Current supported parameters:

- `wave`: Base waveform or tone source. Supported values: `sine`, `square`, `saw`, `triangle`, `noise`.
- `duration`: Total length of the sound in seconds.
- `attack`: Fade-in time from silence to full volume.
- `decay`: Time to fall from peak volume to sustain volume.
- `sustain_level`: Volume level held after decay, before release.
- `release`: Fade-out time at the end.
- `gain`: Overall output volume.
- `freq`: Starting pitch or frequency.
- `freq_end`: Ending pitch or frequency for a sweep over time.
- `noise`: Amount of noise mixed into the tone.
- `lowpass`: Low-pass filter strength to soften the sound.
- `repeat`: Number of repeated pulses or segments.
- `repeat_gap`: Time gap between repeats.
- `tremolo_depth`: Strength of volume wobble.
- `tremolo_freq`: Speed of the tremolo wobble.

The **Data** dock for `Game / Audio FX` includes a `Play` button. It previews the `sfx.*` section your text cursor is currently inside.

Generated effects can be used anywhere normal audio names are used:

- `play_audio("door_open")`
- combat rules audio such as `incoming_fx = "hit"`
- spell or weapon item overrides such as `attack_fx = "fire_cast"`

## Combat Audio From Rules

`Game / Rules` can play audio automatically when damage is dealt.

```toml
[combat.audio]
incoming_fx = "hit"
outgoing_fx = "attack"

[combat.kinds.fire.audio]
outgoing_fx = "fire_cast"
```

Per-item overrides are also supported on weapons and spell items:

- `attack_fx`
- `attack_bus`
- `attack_gain`
- `hit_fx`
- `hit_bus`
- `hit_gain`

Item overrides win over the shared rules defaults.

## Runtime Audio Buses (Layers)

When playing audio, use a bus/layer so music and effects can be mixed independently.

Common bus names:

- `music`
- `sfx`
- `ui`
- `ambience`
- `voice`

You can also use custom bus names.

## Server Script Commands

### `play_audio`

Plays an audio asset:

```eldrin
play_audio("door_open")
play_audio("battle_theme", "music", 0.8, true)
```

Parameters:

- `name` (required): audio asset name.
- `bus` (optional): defaults to `"sfx"`.
- `gain` (optional): `0.0..4.0`, defaults to `1.0`.
- `looping` (optional): defaults to `false`.

### `clear_audio`

Stops currently playing audio:

```eldrin
clear_audio("music") // stop only one bus
clear_audio() // stop all buses
```

### `set_audio_bus_volume`

Sets bus volume:

```eldrin
set_audio_bus_volume("music", 0.5)
set_audio_bus_volume("sfx", 1.0)
```

`volume` is clamped to `0.0..4.0`.

## Typical Usage Pattern

```eldrin
// Start background music in loop
play_audio("village_theme", "music", 0.7, true)

// Play one-shot effect
play_audio("sword_hit", "sfx", 1.0, false)

// Duck music for a cutscene
set_audio_bus_volume("music", 0.35)

// Restore normal level
set_audio_bus_volume("music", 0.7)

// Stop music when leaving area
clear_audio("music")
```

## Related References

- [Server Commands](characters_items/server_commands)
- [Project Tree](creator/project_tree)
- [Events](characters_items/events)
