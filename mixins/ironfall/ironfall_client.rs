// Ironfall's client half: the server tells it of each impact in this level
// (ironfall_glue, on `ironfall:impact`). It plays the boom when the shock
// wave reaches the player, shakes the camera then, and hands the pack in
// shaders/ where and how long ago each blast was, for its cloud and dust ring.
use super::Game;
use super::ironfall_ballistics::{self as rules, Impact, ShellKind};
use super::native_hooks::NativeShake;
use std::sync::Mutex;

/// The pack reads four blasts, two slots each.
const PACK_BLASTS: usize = 4;
/// Ticks a blast is kept: the pack's clouds last 70 seconds.
const LIFE: u64 = 70 * 20;

struct Blast {
    at: [f64; 3],
    kind: ShellKind,
    dimension: String,
    /// The client tick it hit on, and the tick its wave reaches the player.
    born: u64,
    arrives: u64,
    distance: f64,
}

struct Client {
    tick: u64,
    blasts: Vec<Blast>,
}

static CLIENT: Mutex<Client> = Mutex::new(Client {
    tick: 0,
    blasts: Vec::new(),
});

fn client() -> std::sync::MutexGuard<'static, Client> {
    CLIENT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The window's game, once a tick: count ticks, forget old blasts and the
/// blasts of a level the player left.
pub fn tick(game: &Game<'_>) {
    if !game.native_window() {
        return;
    }
    let here = game.native_player().map(|p| p.dimension.to_owned());
    let mut c = client();
    c.tick += 1;
    let now = c.tick;
    c.blasts
        .retain(|b| now.saturating_sub(b.born) < LIFE && here.as_deref() == Some(&*b.dimension));
}

/// A message from the server: an impact, when it is Ironfall's.
pub fn from_server(game: &mut Game<'_>, channel: &str, data: &[u8]) {
    if channel != rules::IMPACT_CHANNEL || !game.native_on(super::ironfall_glue::ID) {
        return;
    }
    let (Some(impact), Some(p)) = (Impact::decode(data), game.native_player()) else {
        return;
    };
    let eye = [p.x, p.y + 1.62, p.z];
    let d = [
        impact.at[0] - eye[0],
        impact.at[1] - eye[1],
        impact.at[2] - eye[2],
    ];
    let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    let delay = rules::wave_delay(distance);
    let r = impact.kind.radius();
    // Close by, the blast's own sound (native_explode) is the boom; farther
    // off, a deeper one that waits for the wave, and a rumble after it. The
    // volume carries it: vanilla hears a sound out to 16 blocks times it.
    let reach = (distance / 16.0 + 1.5) as f32;
    if distance > 24.0 {
        let pitch = (0.75 - r as f32 * 0.04).max(0.45);
        game.native_play_local_sound(
            "minecraft:entity.generic.explode",
            impact.at,
            reach,
            pitch,
            delay,
        );
    }
    game.native_play_local_sound(
        "minecraft:entity.lightning_bolt.thunder",
        impact.at,
        reach.max(r as f32),
        0.5,
        (delay + 4).min(rules::MAX_DELAY),
    );
    let mut c = client();
    let now = c.tick;
    c.blasts.push(Blast {
        at: impact.at,
        kind: impact.kind,
        dimension: p.dimension.to_owned(),
        born: now,
        arrives: now + delay as u64,
        distance,
    });
    let extra = c.blasts.len().saturating_sub(16);
    c.blasts.drain(..extra);
}

/// The camera shake of every blast whose wave has just reached the player.
pub fn camera_shake(game: &Game<'_>, partial: f32, shake: &mut NativeShake) {
    if !game.native_on(super::ironfall_glue::ID) {
        return;
    }
    let c = client();
    let now = c.tick as f64 + f64::from(partial);
    for b in &c.blasts {
        let t = now - b.arrives as f64;
        let s = rules::shake_strength(b.kind.radius(), b.distance, t) as f32;
        if s <= 0.0 {
            continue;
        }
        // Quick, uneven wobbles on each axis.
        let t = t as f32;
        shake.pitch += s * 4.0 * (t * 2.7).sin();
        shake.yaw += s * 2.5 * (t * 3.3 + 1.0).sin();
        shake.roll += s * 3.0 * (t * 2.1 + 2.0).sin();
        shake.offset[1] += s * 0.12 * (t * 4.1).sin();
    }
}

/// The pack's values: the newest four blasts, as shaders/cloud.wgsl reads
/// them.
pub fn shader_params(game: &Game<'_>, partial: f32, params: &mut [[f32; 4]; 8]) {
    if !game.native_on(super::ironfall_glue::ID) {
        return;
    }
    let c = client();
    let now = c.tick as f64 + f64::from(partial);
    for (i, b) in c.blasts.iter().rev().take(PACK_BLASTS).enumerate() {
        let age = ((now - b.born as f64) / 20.0) as f32;
        let fade = (1.0 - age / (LIFE as f32 / 20.0)).clamp(0.0, 1.0);
        params[i * 2] = [b.at[0] as f32, b.at[1] as f32, b.at[2] as f32, age];
        params[i * 2 + 1] = [b.kind.radius() as f32, fade, 0.0, 0.0];
    }
}
