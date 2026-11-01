# Native mods: draw a mob with the mod's own model (native_mob_submit)

<!-- claude-projects-attribution: {"github_login":"Vaspyyy"} -->
_Requested by **Ransom** · [project thread](https://claude.ai/code/project/chan_014LE6qsPXZ5YJ26HjaADSTZ?thread=cmsg_014LE6qsPXZ5YJ26HjaADSTZ5FViA2Ac9eywAJFW3BtEf9)_

> 🤖 Opened by Vaspyyy's AI agent (Claude Code, Claude Opus 5.5), not by Vaspyyy personally.

## What and why

Fixes #5861

Client mods can draw their block entities (`native_block_entity_submit`, #5562) but not mobs, so a mod's creature (a villager-like settler, a bird) always wears a vanilla model. This adds the client hook `native_mob_submit(mob, draw, drawn)` (`EntityRenderer.submit`; NeoForge's `RenderLivingEvent.Pre` when cancelled):

- It is asked once a frame for every mob in view but players, after the usual distance and frustum checks.
- `mob` (`NativeMobRender`) is the frame's `LivingEntityRenderState`: id, kind, custom name, baby, partial tick, age, body yaw, head yaw (from the body) and pitch, the walk cycle (`walkAnimation.position(partial)` and `speed(partial)`), hurt and death time, on ground.
- `draw` is the same `BlockEntityDraw` block entities get, posed at the mob's feet and turned with its body (+z ahead, +x left, +y up), so `native_draw_cube`, `native_draw_quad`, `native_draw_block` and `native_draw_item` work unchanged. Two small constructors on `BlockEntityDraw` make that frame.
- Setting `drawn` leaves out the vanilla model, armour, held items and custom head; the name tag, shadow, flames and glowing outline still draw, and occlusion culling applies as for any mob. A hook that draws nothing and leaves `drawn` false changes nothing.

A mod's own creature is then an ordinary mob (`native_spawn_mob`, API 37) that keeps vanilla's movement, sounds and saving, with a model and animations of its own. This is the drawing half of "mod entity types with their own model" in #5556; it doesn't add entity types (#5212's area).

The hook comes as an API fragment (`tools/native-hook-fragments/mob_submit.json`).

## Kind

- [x] feature: something new for players (listed under New in the release notes)

## Vanilla reference

`EntityRenderer.submit`, `LivingEntityRenderer.extractRenderState` (`bodyRot` with `Mth.rotLerp`, `yRot` as head minus body, `xRot`, `walkAnimationPos`/`walkAnimationSpeed` from `WalkAnimationState.position(partial)`/`speed(partial)`, `ageInTicks`). NeoForge's `RenderLivingEvent.Pre` for the contract.

## Other projects' code

none

## Self-check

**pr_ready** (`tools/pr_ready.py --quick --base upstream/main` on vm, against upstream/main at bd8c53bfa, 3 min 54 s)

| Check | Result |
| --- | --- |
| Scope | ✅ 7 files, +253 −0 |
| Cargo.lock | ✅ unchanged |
| Public tree | ✅ |
| Python tests | ✅ 515 passed (41 s) |
| rustfmt | ✅ (10 s) |
| clippy | ✅ (1 min 35 s) |
| tests: horizon-client | ✅ 1670 passed (1 min 26 s) |
| doc tests | ✅ 0 passed (2 s) |
| content root tests | ➖ content root untouched |
| tokens.css up to date | ➖ theme untouched |
| launcher rustfmt | ➖ launcher untouched |
| launcher clippy | ➖ launcher untouched |
| launcher tests | ➖ launcher untouched |
| Parity | ➖ not run (--quick): CI's parity job picks the affected harnesses |
| Label | player-facing: `feature` if players get something new, no label for a fix (`release-now` if urgent) |

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01XnU6QRKvdg3SMVP1onGqS3
