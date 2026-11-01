# Native mods: biome colours a client mod changes, for seasons (native_biome_colors)

<!-- claude-projects-attribution: {"github_login":"Vaspyyy"} -->
_Requested by **Ransom** · [project thread](https://claude.ai/code/project/chan_014LE6qsPXZ5YJ26HjaADSTZ?thread=cmsg_014LE6qsPXZ5YJ26HjaADSTZ5FViA2Ac9eywAJFW3BtEf9)_

> 🤖 Opened by Vaspyyy's AI agent (Claude Code, Claude Opus 5.5), not by Vaspyyy personally.

## What and why

Fixes #5862

Grass, leaf and water colours come from biome files and the colormaps, fixed while the game runs, so a seasons mod can't turn the leaves gold in autumn or pale the grass in winter. This adds:

- `native_biome_colors(biome, colors)` (client hook): changes a biome's `BiomeColors` (`grass`, `foliage`, `dry_foliage`, `water`, 0xRRGGBB), starting from the biome's own.
- `native_refresh_biome_colors()` (client helper): asks the hook for every biome (`Biomes::with_colors`). When the colours differ from those in use it swaps the mesh workers' biome table and rebuilds the loaded sections and the far terrain (`Streaming::reload_resources`, `clear_lod`), as changing Biome Blend does. Nothing changes (no rebuild) when no mod changes a colour.

The hook is only asked on that call, so the mesher's per-block colour lookup costs nothing extra. The dark forest's grass modifier still applies on top; the swamp's noise colours replace grass as before. Block-break and leaf particles keep the biome's own colours for now (they read the world's table). A season's shader mood already works through `native_shader_params` (`frame.mods`, #5411), so this PR doesn't touch the shader frame.

The hook comes as an API fragment (`tools/native-hook-fragments/biome_colors.json`).

## Kind

- [x] feature: something new for players (listed under New in the release notes)

## Vanilla reference

Horizon-only (vanilla's colours are `BiomeSpecialEffects` and `GrassColor`/`FoliageColor`, fixed per resource reload). The rebuild follows `LevelRenderer.allChanged`.

## Other projects' code

none

## Self-check

**pr_ready** (`tools/pr_ready.py --quick --base upstream/main` on vm, against upstream/main at bd8c53bfa, 4 min 58 s)

| Check | Result |
| --- | --- |
| Scope | ✅ 6 files, +203 −0 |
| Cargo.lock | ✅ unchanged |
| Public tree | ✅ |
| Python tests | ✅ 515 passed (44 s) |
| rustfmt | ✅ (11 s) |
| clippy | ✅ (1 min 56 s) |
| tests: horizon-client, horizon-client-render | ✅ 2505 passed (2 min 4 s) |
| doc tests | ✅ 0 passed (3 s) |
| content root tests | ➖ content root untouched |
| tokens.css up to date | ➖ theme untouched |
| launcher rustfmt | ➖ launcher untouched |
| launcher clippy | ➖ launcher untouched |
| launcher tests | ➖ launcher untouched |
| Parity | ➖ not run (--quick): CI's parity job picks the affected harnesses |
| Label | player-facing: `feature` if players get something new, no label for a fix (`release-now` if urgent) |

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01XnU6QRKvdg3SMVP1onGqS3
