# Native mods: music a client mod chooses and layers (native_music, native_music_layers)

<!-- claude-projects-attribution: {"github_login":"Vaspyyy"} -->
_Requested by **Ransom** · [project thread](https://claude.ai/code/project/chan_014LE6qsPXZ5YJ26HjaADSTZ?thread=cmsg_014LE6qsPXZ5YJ26HjaADSTZ5FViA2Ac9eywAJFW3BtEf9)_

> 🤖 Opened by Vaspyyy's AI agent (Claude Code, Claude Opus 5.5), not by Vaspyyy personally.

## What and why

Fixes #5863

A mod can play one-off sounds (`native_play_local_sound`) but can't choose the background music or build it up. This adds two client hooks, asked once a client tick from `tick_music`:

- `native_music(music, volume)`: what vanilla chose (`Minecraft.getSituationalMusic`, `getMusicVolume`) comes in; a mod can set its own `Music` (a content mod's sound event, its delays and `replace_current_music`), None for silence, or a lower volume. The music manager then plays it exactly as vanilla's (delays, Music Frequency, Now Playing toast). NeoForge's `SelectMusicEvent`.
- `native_music_layers(layers)`: looping tracks played over (or instead of) that music, each a `MusicLayer { sound, volume }`. `MusicLayers` keeps one music-source instance per sound (`SoundInstance::for_music_layer`: looping, unattenuated, allowed to start silent, ticked so its volume reaches the channel), eases each volume by 0.025 a tick, and stops a layer once it has faded out after being left out. Layers that first appear in the same tick start together, so the stems of one piece stay in step; a layer stopped by a level change starts again on the next tick.

Together these let a mod's soundtrack follow a value it keeps (seasons, or a village's wellbeing raising extra stems).

The hooks come as an API fragment (`tools/native-hook-fragments/music.json`).

## Kind

- [x] feature: something new for players (listed under New in the release notes)

## Vanilla reference

`Minecraft.getSituationalMusic`, `Minecraft.getMusicVolume`, `MusicManager.tick` (unchanged; the hook sits between them). The layers are Horizon-only; they use the sound engine's tickable instance path (`SoundEngine.tickInGame`'s volume updates).

## Other projects' code

none

## Self-check

**pr_ready** (`tools/pr_ready.py --quick --base upstream/main` on vm, against upstream/main at bd8c53bfa, 8 min 43 s)

| Check | Result |
| --- | --- |
| Scope | ✅ 7 files, +266 −2 |
| Cargo.lock | ✅ unchanged |
| Public tree | ✅ |
| Python tests | ✅ 515 passed (47 s) |
| rustfmt | ✅ (10 s) |
| clippy | ✅ (3 min 34 s) |
| tests: horizon-client, horizon-client-render | ✅ 2505 passed (4 min 8 s) |
| doc tests | ✅ 0 passed (4 s) |
| content root tests | ➖ content root untouched |
| tokens.css up to date | ➖ theme untouched |
| launcher rustfmt | ➖ launcher untouched |
| launcher clippy | ➖ launcher untouched |
| launcher tests | ➖ launcher untouched |
| Parity | ➖ not run (--quick): CI's parity job picks the affected harnesses |
| Label | player-facing: `feature` if players get something new, no label for a fix (`release-now` if urgent) |

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01XnU6QRKvdg3SMVP1onGqS3
