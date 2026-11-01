// Ironfall's server half: commands fire shells, each level's tick flies them
// (a shell model eases along on every client), and an impact sets off a real
// blast, cuts a crater, throws debris and smoke, and tells every player in
// the level where it was, so their client half (ironfall_client) times the
// boom, the camera shake and the pack's cloud to the distance.
use super::Game;
use super::ironfall_ballistics::{self as rules, Command, Cut, Impact, Shell, ShellKind};
use super::native_entities::NativeDisplay;
use super::native_explosions::NativeBlast;
use super::native_falling::NativeFalling;
use horizon_client::particles::{ParticleOptions, ParticleType};
use horizon_client::text::Text;
use horizon_world::nbt::{Compound, Tag};
use std::sync::Mutex;

pub const ID: &str = "ironfall";

/// A shell in the air and the display entity players see it as, once it
/// has one.
struct Flying {
    shell: Shell,
    model: Option<u32>,
}

/// Shells in the air, and shells of a barrage still waiting to fire (ticks
/// left, the shell).
struct State {
    shells: Vec<Flying>,
    queued: Vec<(u32, Shell)>,
    impacts: u64,
    /// Display entities to remove: (dimension, id).
    gone: Vec<(String, u32)>,
}

static STATE: Mutex<State> = Mutex::new(State {
    shells: Vec::new(),
    queued: Vec::new(),
    impacts: 0,
    gone: Vec::new(),
});

/// The item a shell is drawn as, and how big, by kind.
fn model(kind: ShellKind) -> (&'static str, f32) {
    match kind {
        ShellKind::Mortar => ("minecraft:fire_charge", 0.45),
        ShellKind::Field => ("minecraft:fire_charge", 0.65),
        ShellKind::Howitzer => ("minecraft:heavy_core", 0.8),
    }
}

/// At most this many debris blocks a blast throws.
fn debris_cap(kind: ShellKind) -> usize {
    match kind {
        ShellKind::Mortar => 10,
        ShellKind::Field => 24,
        ShellKind::Howitzer => 48,
    }
}

fn state() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Blocks a crater never takes.
const KEEP: [&str; 6] = [
    "minecraft:bedrock",
    "minecraft:barrier",
    "minecraft:end_portal_frame",
    "minecraft:reinforced_deepslate",
    "minecraft:command_block",
    "minecraft:structure_block",
];

pub fn register(commands: &mut Vec<super::native_server::NativeCommand>) {
    use super::native_server::NativeCommand;
    commands.push(NativeCommand::op("ironfall fire <shell:word>", 2));
    commands.push(NativeCommand::op(
        "ironfall barrage <shell:word> <count:integer>",
        2,
    ));
    commands.push(NativeCommand::op("ironfall clear", 2));
}

pub fn perform(game: &mut Game<'_>, command: &str, ran: &mut bool) {
    let Some(parsed) = rules::parse(command) else {
        return;
    };
    *ran = true;
    if !game.native_on(ID) {
        game.native_feedback(Text::literal("Ironfall is turned off in Options > Mods"));
        return;
    }
    let command = match parsed {
        Ok(c) => c,
        Err(usage) => {
            game.native_feedback(Text::literal(usage));
            return;
        }
    };
    let Some(p) = game.native_player() else {
        return;
    };
    let eye = [p.x, p.y + 1.62, p.z];
    let mut s = state();
    match command {
        Command::Clear => {
            let n = s.shells.len() + s.queued.len();
            let models: Vec<(String, u32)> = s
                .shells
                .drain(..)
                .filter_map(|f| f.model.map(|m| (f.shell.dimension, m)))
                .collect();
            s.gone.extend(models);
            s.queued.clear();
            drop(s);
            game.native_feedback(Text::literal(format!("Dropped {n} shells")));
        }
        Command::Fire(kind) => {
            if s.shells.len() >= rules::MAX_SHELLS {
                drop(s);
                game.native_feedback(Text::literal("Too many shells in the air"));
                return;
            }
            s.shells.push(Flying {
                shell: Shell::fired(kind, p.dimension, eye, p.yaw, p.pitch),
                model: None,
            });
            drop(s);
            game.native_feedback(Text::literal(format!("Fired a {} shell", kind.name())));
        }
        Command::Barrage(kind, count) => {
            for i in 0..count {
                // A fan of shells: a little left and right, a little long and short.
                let spread = |k: u32, salt: i32| {
                    (rules::cell_noise(k as i32, salt, 0, s.impacts) as f32 - 0.5) * 2.0
                };
                let yaw = p.yaw + spread(i, 1) * 4.0;
                let pitch = p.pitch + spread(i, 2) * 2.5;
                s.queued
                    .push((i * 6, Shell::fired(kind, p.dimension, eye, yaw, pitch)));
            }
            drop(s);
            game.native_feedback(Text::literal(format!(
                "Barrage of {count} {} shells",
                kind.name()
            )));
        }
    }
}

/// After the level `dimension` ticks: fly its shells, and blow up the ones
/// that hit.
pub fn level_tick(game: &mut Game<'_>, dimension: &str) {
    let gone: Vec<(String, u32)> = {
        let mut s = state();
        let (here, elsewhere) = s.gone.drain(..).partition(|g| g.0 == dimension);
        s.gone = elsewhere;
        here
    };
    for (_, id) in gone {
        game.native_remove_entity(dimension, id);
    }
    if !game.native_on(ID) {
        return;
    }
    let mut hits: Vec<(ShellKind, [f64; 3])> = Vec::new();
    // Shells still flying: (index, position, velocity, model).
    let mut flying: Vec<(usize, [f64; 3], [f64; 3], Option<u32>)> = Vec::new();
    let mut landed_models: Vec<u32> = Vec::new();
    {
        let mut s = state();
        // A barrage's next shells.
        let mut ready = Vec::new();
        s.queued.retain_mut(|(wait, shell)| {
            if shell.dimension != dimension {
                return true;
            }
            if *wait == 0 {
                ready.push(shell.clone());
                false
            } else {
                *wait -= 1;
                true
            }
        });
        for shell in ready {
            if s.shells.len() < rules::MAX_SHELLS {
                s.shells.push(Flying { shell, model: None });
            }
        }
        let mut i = 0;
        while i < s.shells.len() {
            if s.shells[i].shell.dimension != dimension {
                i += 1;
                continue;
            }
            let mut left_world = false;
            let hit = s.shells[i].shell.step(&mut |p| match stops(game, p) {
                Some(stop) => stop,
                None => {
                    left_world = true;
                    true
                }
            });
            let f = &s.shells[i];
            if let Some(at) = hit {
                if !left_world {
                    hits.push((f.shell.kind, at));
                }
                landed_models.extend(f.model);
                s.shells.swap_remove(i);
            } else if f.shell.age > rules::MAX_AGE {
                landed_models.extend(f.model);
                s.shells.swap_remove(i);
            } else {
                flying.push((i, f.shell.position, f.shell.velocity, f.model));
                i += 1;
            }
        }
        s.impacts += hits.len() as u64;
    }
    for id in landed_models {
        game.native_remove_entity(dimension, id);
    }
    for (i, at, velocity, model) in flying {
        smoke_trail(game, at);
        let (yaw, pitch) = rules::heading(velocity);
        match model {
            Some(id) => {
                game.native_move_entity(dimension, id, at, yaw, pitch);
            }
            None => {
                let kind = state().shells[i].shell.kind;
                if let Some(id) = summon_model(game, dimension, kind, at, yaw, pitch) {
                    let mut s = state();
                    match s.shells.get_mut(i) {
                        Some(f) => f.model = Some(id),
                        None => s.gone.push((dimension.to_owned(), id)),
                    }
                }
            }
        }
    }
    let seed = state().impacts;
    for (k, (kind, at)) in hits.into_iter().enumerate() {
        impact(
            game,
            dimension,
            kind,
            at,
            seed.wrapping_mul(31).wrapping_add(k as u64),
        );
    }
}

/// The display entity a shell is seen as: an item that eases from tick to
/// tick on every client, pointed along its flight.
fn summon_model(
    game: &mut Game<'_>,
    dimension: &str,
    kind: ShellKind,
    at: [f64; 3],
    yaw: f32,
    pitch: f32,
) -> Option<u32> {
    let (item, scale) = model(kind);
    let mut stack = Compound::new();
    stack.insert("id".into(), Tag::String(item.into()));
    stack.insert("count".into(), Tag::Int(1));
    let mut nbt = Compound::new();
    nbt.insert("item".into(), Tag::Compound(stack));
    nbt.insert("item_display".into(), Tag::String("fixed".into()));
    nbt.insert(
        "Rotation".into(),
        Tag::List(vec![Tag::Float(yaw.to_bits()), Tag::Float(pitch.to_bits())]),
    );
    let id = game.native_summon(dimension, "minecraft:item_display", at, nbt)?;
    game.native_set_display(
        dimension,
        id,
        &NativeDisplay {
            scale: [scale; 3],
            teleport: 1,
            ..NativeDisplay::default()
        },
    );
    Some(id)
}

/// Whether the block at `p` stops a shell; None where the chunk isn't
/// loaded (the shell is dropped there) or below the world.
fn stops(game: &Game<'_>, p: [i32; 3]) -> Option<bool> {
    if !game.world.chunks.contains_key(&(p[0] >> 4, p[2] >> 4)) || p[1] < -64 {
        return None;
    }
    let state = game.world.block(p[0], p[1], p[2]);
    let facts = game.res.facts();
    if facts.is_air(state) {
        return Some(false);
    }
    let f = facts.get(state);
    // Grass, flowers and water let it through; anything solid, and leaves, stop it.
    Some(f.solid_render || f.name.ends_with("_leaves"))
}

fn smoke_trail(game: &mut Game<'_>, at: [f64; 3]) {
    game.with_level(|_, level, _| {
        level.native_send_particles(
            &ParticleOptions::simple(ParticleType::CampfireSignalSmoke),
            None,
            at,
            [0.05, 0.05, 0.05],
            0.005,
            2,
        );
        level.native_send_particles(
            &ParticleOptions::simple(ParticleType::Flame),
            None,
            at,
            [0.0; 3],
            0.0,
            0,
        );
    });
}

fn impact(game: &mut Game<'_>, dimension: &str, kind: ShellKind, at: [f64; 3], seed: u64) {
    let radius = kind.radius();
    // The blast itself: vanilla's damage, knockback, sound and flash, with no
    // block broken by it, so the crater stays Ironfall's own shape.
    game.native_explode(dimension, at, radius as f32, false, NativeBlast::Keep);
    carve(game, dimension, kind, at, seed);
    blast_effects(game, at, radius);
    tell_players(game, dimension, kind, at);
}

/// Every player in the level hears of the impact: their client half plays
/// the boom when the wave reaches them, shakes their camera and draws the
/// cloud.
fn tell_players(game: &mut Game<'_>, dimension: &str, kind: ShellKind, at: [f64; 3]) {
    let data = Impact { at, kind }.encode();
    for p in game.native_server_players() {
        if p.dimension == dimension {
            let _ = game.native_net_send_to_player(p.uuid, rules::IMPACT_CHANNEL, &data);
        }
    }
}

fn carve(game: &mut Game<'_>, dimension: &str, kind: ShellKind, at: [f64; 3], seed: u64) {
    let radius = kind.radius();
    let mut thrown = 0;
    let registry = game.pack.assets.registry;
    let (Some(air), Some(scorch)) = (
        registry.default_state("minecraft:air"),
        registry.default_state("minecraft:coarse_dirt"),
    ) else {
        return;
    };
    for (p, cut) in rules::crater(at, radius, seed) {
        if !game.world.chunks.contains_key(&(p[0] >> 4, p[2] >> 4))
            || game.world.block_entities.contains_key(&(p[0], p[1], p[2]))
        {
            continue;
        }
        let state = game.world.block(p[0], p[1], p[2]);
        let facts = game.res.facts();
        if facts.is_air(state) {
            continue;
        }
        let f = facts.get(state);
        let name = f.name.clone();
        let solid = f.solid_render;
        if KEEP.contains(&&*name) {
            continue;
        }
        match cut {
            Cut::Air => {
                game.native_set_block(dimension, p, air, 3);
                // Some of the crater flies out as debris and lands around it.
                if solid && thrown < debris_cap(kind) && rules::is_debris(p, at, seed, 0.18) {
                    let from = [
                        f64::from(p[0]) + 0.5,
                        f64::from(p[1]),
                        f64::from(p[2]) + 0.5,
                    ];
                    let velocity = rules::debris_velocity(p, at, radius, seed);
                    let falling = NativeFalling {
                        drop_item: false,
                        hurts: false,
                    };
                    if game.native_spawn_falling_block(dimension, state, from, velocity, falling) {
                        thrown += 1;
                    }
                }
            }
            Cut::Scorch => {
                let above = game.world.block(p[0], p[1] + 1, p[2]);
                let soil = matches!(
                    &*name,
                    "minecraft:grass_block"
                        | "minecraft:dirt"
                        | "minecraft:podzol"
                        | "minecraft:mycelium"
                );
                if soil && game.res.facts().is_air(above) {
                    game.native_set_block(dimension, p, scorch, 3);
                }
            }
        }
    }
}

fn blast_effects(game: &mut Game<'_>, at: [f64; 3], radius: f64) {
    let r = radius;
    game.with_level(|_, level, _| {
        let mut send = |ty: ParticleType, spread: [f64; 3], speed: f64, count: i32| {
            level.native_send_particles(
                &ParticleOptions::simple(ty),
                None,
                at,
                spread,
                speed,
                count,
            );
        };
        // The blast's own flash comes with native_explode; this is the smoke
        // and the embers. The pack (shaders/) draws the cloud and dust ring
        // for players who have it on.
        send(
            ParticleType::LargeSmoke,
            [r * 0.7, r * 0.5, r * 0.7],
            0.08,
            (r * 12.0) as i32,
        );
        send(
            ParticleType::CampfireSignalSmoke,
            [r * 0.5, 0.5, r * 0.5],
            0.03,
            (r * 8.0) as i32,
        );
        send(
            ParticleType::Lava,
            [r * 0.4, 0.3, r * 0.4],
            0.0,
            (r * 3.0) as i32,
        );
    });
}
