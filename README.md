# Ironfall

An artillery native mod for Horizon. You call in shells that arc across the sky trailing smoke. Where they land, a real explosion goes off, a ragged crater opens, and debris flies out and rains down around it. Players far away see the flash first, then hear the boom and feel their camera shake when the shock wave reaches them. With the bundled shader pack on, a mushroom cloud rises and a ring of dust rolls outward.

Version 0.2 needs the six artillery hooks merged on 2026-11-01 (hooks 84 once main's pending hook numbers are stamped). It was built against main at `f77b139`.

## Commands

Operators of level 2 and up can run these:

| Command | What it does |
| --- | --- |
| `/ironfall fire <mortar\|field\|howitzer>` | Fires one shell where you look. The higher you aim, the farther it flies. |
| `/ironfall barrage <shell> <count>` | Fires 1 to 16 shells, one every 6 ticks, in a loose fan. |
| `/ironfall clear` | Removes every shell that is in the air or waiting to fire. |

The ranges below assume level ground and a 45° aim.

| Shell | Muzzle speed | Crater radius | Range |
| --- | --- | --- | --- |
| mortar | 1.8 blocks/tick | 2.5 | about 60 blocks |
| field | 3.0 blocks/tick | 4.0 | about 140 blocks |
| howitzer | 4.2 blocks/tick | 6.5 | about 255 blocks |

A shell that flies into an unloaded chunk is dropped without exploding.

## Layout

- `mixins/ironfall/ironfall_ballistics.rs`: the rules for flight, craters, debris, the shock wave, camera shake strength, impact messages and command parsing. It contains no game code, and `cargo test` covers it (10 tests).
- `mixins/ironfall/ironfall_glue.rs`: the server half. It flies the shells (each one is an `item_display` that eases along on clients), then on impact calls `native_explode` (with `NativeBlast::Keep`, so the crater keeps Ironfall's shape), cuts the crater, throws debris with `native_spawn_falling_block` and tells each player in the level about the impact on the `ironfall:impact` mod message channel.
- `mixins/ironfall/ironfall_client.rs`: the client half. It hears the impact, plays the boom with `native_play_local_sound` when the wave arrives (34 blocks a second), shakes the camera through `native_camera_shake`, and gives the pack the four newest blasts through `native_shader_params`.
- `mixins/ironfall/horizon-mixin.toml`: the bundle definition: three modules and seven HEAD injections.
- `shaders/native-ironfall/`: the shader pack (`ironfall_skies`) that draws the cloud and the dust ring from `frame.mods`.

## What players see

- The shell flies as a small item model (a fire charge, or a heavy core for the howitzer) pointed along its arc, with a smoke and flame trail.
- Close to the impact you hear vanilla's explosion right away. More than 24 blocks away, a deeper boom and a rumble wait for the shock wave, which travels at 34 blocks a second. At 200 blocks that's about 6 seconds.
- The camera shakes when the wave arrives. The shake reaches 40 times the crater radius, and it follows the Distortion Effects setting, so 0% turns it off.
- Debris (10, 24 or 48 blocks by shell) is thrown out of the crater. It lands as blocks and drops nothing.
- The blast's damage and knockback are vanilla's, so armor and Blast Protection work.

## Status

- The rules' tests pass (`cargo test`).
- v0.2 compiles inside Horizon main at `f77b139` (with the pending hook numbers stamped locally), with clippy clean. This was checked with the injections applied by hand; horizon-mixins itself was not run.
- Nobody has played it in game yet. It needs a Nightly built after 2026-11-01 that has the artillery hooks.
- The files have no `//!` comments, which horizon-mixins rejects.
