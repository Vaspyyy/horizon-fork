// Ironfall's rules with no game in them: shell types, flight, craters,
// debris, the shock wave and impact messages. The same file is `crate::game::ironfall_ballistics` in the
// game and a module of this crate for its tests.

/// Gravity on a shell, in blocks a tick squared (an arrow's is 0.05).
pub const GRAVITY: f64 = 0.05;
/// Air drag a tick (an arrow keeps 0.99 of its speed).
pub const DRAG: f64 = 0.995;
/// Steps a tick when a shell's flight is tested against blocks, so a fast
/// shell can't pass through a wall between two ticks.
pub const SUBSTEPS: usize = 8;
/// Shells in the air at once, across all levels.
pub const MAX_SHELLS: usize = 48;
/// Ticks a shell flies before it is dropped (a minute).
pub const MAX_AGE: u32 = 1200;

/// The guns' shells, from light and quick to heavy and far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellKind {
    /// A light mortar round: short range, small crater.
    Mortar,
    /// A field gun shell: middle range, a crater you can hide in.
    Field,
    /// A heavy howitzer shell: long range, a crater that eats a hill.
    Howitzer,
}

impl ShellKind {
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "mortar" => Some(Self::Mortar),
            "field" => Some(Self::Field),
            "howitzer" => Some(Self::Howitzer),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Mortar => "mortar",
            Self::Field => "field",
            Self::Howitzer => "howitzer",
        }
    }
    /// Muzzle speed, blocks a tick.
    pub fn speed(self) -> f64 {
        match self {
            Self::Mortar => 1.8,
            Self::Field => 3.0,
            Self::Howitzer => 4.2,
        }
    }
    /// Crater radius, in blocks.
    pub fn radius(self) -> f64 {
        match self {
            Self::Mortar => 2.5,
            Self::Field => 4.0,
            Self::Howitzer => 6.5,
        }
    }
}

/// A shell in the air.
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    pub kind: ShellKind,
    pub dimension: String,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub age: u32,
}

/// The unit vector a player looks along: yaw 0 faces south (+z), 90 west
/// (-x); pitch is positive looking down.
pub fn look(yaw: f32, pitch: f32) -> [f64; 3] {
    let (y, p) = (f64::from(yaw).to_radians(), f64::from(pitch).to_radians());
    [-y.sin() * p.cos(), -p.sin(), y.cos() * p.cos()]
}

impl Shell {
    /// A shell fired from `eye` along the look of `yaw` and `pitch`.
    pub fn fired(kind: ShellKind, dimension: &str, eye: [f64; 3], yaw: f32, pitch: f32) -> Self {
        let dir = look(yaw, pitch);
        let speed = kind.speed();
        Self {
            kind,
            dimension: dimension.to_owned(),
            // Clear of the gunner's head.
            position: [
                eye[0] + dir[0] * 1.5,
                eye[1] + dir[1] * 1.5,
                eye[2] + dir[2] * 1.5,
            ],
            velocity: dir.map(|d| d * speed),
            age: 0,
        }
    }
    /// One tick of flight in `SUBSTEPS` steps. `solid` says whether a block
    /// stops the shell; the result is where it hit, if it did.
    pub fn step(&mut self, solid: &mut impl FnMut([i32; 3]) -> bool) -> Option<[f64; 3]> {
        self.age += 1;
        let n = SUBSTEPS as f64;
        for _ in 0..SUBSTEPS {
            for k in 0..3 {
                self.position[k] += self.velocity[k] / n;
            }
            let cell = self.position.map(|v| v.floor() as i32);
            if solid(cell) {
                return Some(self.position);
            }
        }
        self.velocity[1] -= GRAVITY;
        self.velocity = self.velocity.map(|v| v * DRAG);
        None
    }
}

/// A cheap, repeatable noise in 0..1 for a cell (a crater's ragged edge
/// shouldn't change between two runs of the same impact).
pub fn cell_noise(x: i32, y: i32, z: i32, seed: u64) -> f64 {
    let mut h = seed ^ 0x9E37_79B9_7F4A_7C15;
    for v in [x, y, z] {
        h ^= u64::from(v as u32);
        h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        h ^= h >> 31;
    }
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// What happens to a cell around an impact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cut {
    /// Blown away.
    Air,
    /// Left in place, scorched on top.
    Scorch,
}

/// The cells a crater of `radius` at `center` changes: a bowl, deeper than a
/// sphere would cut below and shallower above, its edge roughened by noise,
/// and a ring of scorched ground around it.
pub fn crater(center: [f64; 3], radius: f64, seed: u64) -> Vec<([i32; 3], Cut)> {
    let mut out = Vec::new();
    let r = radius.ceil() as i32 + 2;
    let c = center.map(|v| v.floor() as i32);
    for dx in -r..=r {
        for dz in -r..=r {
            for dy in -r..=r {
                let p = [c[0] + dx, c[1] + dy, c[2] + dz];
                let horizontal = f64::from(dx * dx + dz * dz).sqrt();
                // A bowl: 0.7 of the radius deep, the full radius wide, and
                // up to the radius above the impact (what stood there).
                let vertical = if dy < 0 {
                    f64::from(-dy) / 0.7
                } else {
                    f64::from(dy) * 0.8
                };
                let d = (horizontal * horizontal + vertical * vertical).sqrt();
                let edge = radius * (0.85 + 0.3 * cell_noise(p[0], p[1], p[2], seed));
                if d <= edge {
                    out.push((p, Cut::Air));
                } else if dy <= 0
                    && dy >= -2
                    && horizontal <= radius * 1.6
                    && cell_noise(p[0], p[1] + 7, p[2], seed) < 0.45
                {
                    out.push((p, Cut::Scorch));
                }
            }
        }
    }
    out
}

/// How fast a blast's sound and shock wave travel, blocks a tick (34 blocks a
/// second: slower than real sound, so the wait is felt at Minecraft's
/// distances). The pack's dust ring rolls out at the same speed.
pub const WAVE_SPEED: f64 = 1.7;

/// The longest a boom waits, in ticks (the hook's own limit is 1200).
pub const MAX_DELAY: i32 = 1200;

/// Ticks until the wave of a blast `distance` blocks away arrives.
pub fn wave_delay(distance: f64) -> i32 {
    if !distance.is_finite() || distance <= 0.0 {
        return 0;
    }
    ((distance / WAVE_SPEED).round() as i32).min(MAX_DELAY)
}

/// How far a blast of crater `radius` is felt in the camera, in blocks.
pub fn shake_reach(radius: f64) -> f64 {
    radius * 40.0
}

/// How hard the camera shakes, 0 to 1, `ticks` after the wave of a blast of
/// crater `radius` reached a player `distance` blocks away: a jolt that dies
/// away over about two seconds, weaker the farther off.
pub fn shake_strength(radius: f64, distance: f64, ticks: f64) -> f64 {
    let reach = shake_reach(radius);
    if !(0.0..40.0).contains(&ticks) || distance >= reach {
        return 0.0;
    }
    let near = 1.0 - distance / reach;
    let size = (radius / 6.5).min(1.0);
    size * near * near * (-ticks / 9.0).exp()
}

/// Whether the cell `p` of a crater flies out as debris instead of
/// vanishing: about one cell in `share`, only from the crater's upper half.
pub fn is_debris(p: [i32; 3], center: [f64; 3], seed: u64, share: f64) -> bool {
    f64::from(p[1]) >= center[1].floor() - 1.0 && cell_noise(p[0], p[1] + 13, p[2], seed) < share
}

/// The velocity, blocks a tick, a debris block at `p` is thrown with: out
/// from the centre and up, harder for a bigger blast, with some scatter.
pub fn debris_velocity(p: [i32; 3], center: [f64; 3], radius: f64, seed: u64) -> [f64; 3] {
    let d = [
        f64::from(p[0]) + 0.5 - center[0],
        f64::from(p[2]) + 0.5 - center[2],
    ];
    let len = (d[0] * d[0] + d[1] * d[1]).sqrt().max(0.3);
    let jitter = |salt: i32| cell_noise(p[0], p[1] + salt, p[2], seed);
    let out = 0.25 + radius * 0.06 * (0.5 + jitter(29));
    let up = 0.45 + radius * 0.07 * (0.5 + jitter(31));
    [d[0] / len * out, up, d[1] / len * out]
}

/// The yaw and pitch, in degrees, of something flying along `velocity`, in
/// the game's convention (yaw 0 faces +z, pitch positive looking down).
pub fn heading(velocity: [f64; 3]) -> (f32, f32) {
    let flat = (velocity[0] * velocity[0] + velocity[2] * velocity[2]).sqrt();
    let yaw = (-velocity[0]).atan2(velocity[2]).to_degrees();
    let pitch = (-velocity[1]).atan2(flat).to_degrees();
    (yaw as f32, pitch as f32)
}

/// The channel the server tells each client of an impact on.
pub const IMPACT_CHANNEL: &str = "ironfall:impact";

/// What a client hears of an impact: where, and which shell.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Impact {
    pub at: [f64; 3],
    pub kind: ShellKind,
}

impl ShellKind {
    fn byte(self) -> u8 {
        match self {
            Self::Mortar => 0,
            Self::Field => 1,
            Self::Howitzer => 2,
        }
    }
    fn from_byte(b: u8) -> Option<Self> {
        match b {
            0 => Some(Self::Mortar),
            1 => Some(Self::Field),
            2 => Some(Self::Howitzer),
            _ => None,
        }
    }
}

impl Impact {
    /// 25 bytes: the shell, then x, y and z as big-endian f64s.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![self.kind.byte()];
        for v in self.at {
            out.extend_from_slice(&v.to_be_bytes());
        }
        out
    }
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() != 25 {
            return None;
        }
        let kind = ShellKind::from_byte(data[0])?;
        let mut at = [0.0; 3];
        for (k, v) in at.iter_mut().enumerate() {
            let b: [u8; 8] = data[1 + k * 8..9 + k * 8].try_into().ok()?;
            *v = f64::from_be_bytes(b);
        }
        at.iter()
            .all(|v| v.is_finite())
            .then_some(Self { at, kind })
    }
}

/// A parsed `/ironfall` command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    /// `ironfall fire <shell>`: one shell where the player looks.
    Fire(ShellKind),
    /// `ironfall barrage <shell> <count>`: a spread of shells, one every few ticks.
    Barrage(ShellKind, u32),
    /// `ironfall clear`: every shell in the air is dropped.
    Clear,
}

pub fn parse(command: &str) -> Option<Result<Command, String>> {
    let mut words = command.split_whitespace();
    if words.next() != Some("ironfall") {
        return None;
    }
    let usage = || {
        "Usage: /ironfall fire <mortar|field|howitzer>, /ironfall barrage <shell> <count>, /ironfall clear".to_owned()
    };
    Some(match words.next() {
        Some("fire") => words
            .next()
            .and_then(ShellKind::parse)
            .map(Command::Fire)
            .ok_or_else(usage),
        Some("barrage") => match (words.next().and_then(ShellKind::parse), words.next()) {
            (Some(kind), Some(n)) => match n.parse::<u32>() {
                Ok(n) if (1..=16).contains(&n) => Ok(Command::Barrage(kind, n)),
                _ => Err("A barrage is 1 to 16 shells".to_owned()),
            },
            _ => Err(usage()),
        },
        Some("clear") => Ok(Command::Clear),
        _ => Err(usage()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looking_south_and_level_fires_along_plus_z() {
        let d = look(0.0, 0.0);
        assert!(d[0].abs() < 1e-9 && d[1].abs() < 1e-9 && (d[2] - 1.0).abs() < 1e-9);
        let west = look(90.0, 0.0);
        assert!((west[0] + 1.0).abs() < 1e-9);
        let up = look(0.0, -90.0);
        assert!((up[1] - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_shell_arcs_down_and_lands_on_flat_ground() {
        let mut shell = Shell::fired(
            ShellKind::Field,
            "minecraft:overworld",
            [0.5, 65.6, 0.5],
            0.0,
            -45.0,
        );
        let mut ticks = 0;
        let hit = loop {
            ticks += 1;
            assert!(ticks < MAX_AGE, "never landed");
            if let Some(at) = shell.step(&mut |p| p[1] < 64) {
                break at;
            }
        };
        // Up and over, then down well south of the gun.
        assert!(hit[2] > 60.0, "landed at {hit:?}");
        assert!(hit[1] < 64.0 && hit[1] > 62.0);
        assert!(ticks > 40);
    }

    #[test]
    fn heavier_shells_fly_farther_and_dig_deeper() {
        let range = |kind| {
            let mut s = Shell::fired(kind, "o", [0.5, 65.6, 0.5], 0.0, -45.0);
            loop {
                if let Some(at) = s.step(&mut |p| p[1] < 64) {
                    return at[2];
                }
            }
        };
        assert!(range(ShellKind::Mortar) < range(ShellKind::Field));
        assert!(range(ShellKind::Field) < range(ShellKind::Howitzer));
        let air = |kind: ShellKind| {
            crater([0.5, 64.0, 0.5], kind.radius(), 1)
                .iter()
                .filter(|c| c.1 == Cut::Air)
                .count()
        };
        assert!(air(ShellKind::Mortar) < air(ShellKind::Field));
        assert!(air(ShellKind::Field) < air(ShellKind::Howitzer));
    }

    #[test]
    fn a_crater_is_a_bowl_with_a_scorched_rim() {
        let cells = crater([0.5, 64.0, 0.5], 4.0, 7);
        let at = |p: [i32; 3]| cells.iter().find(|c| c.0 == p).map(|c| c.1);
        assert_eq!(at([0, 64, 0]), Some(Cut::Air));
        assert_eq!(at([0, 62, 0]), Some(Cut::Air));
        assert_eq!(at([0, 56, 0]), None, "too deep");
        assert!(cells.iter().any(|c| c.1 == Cut::Scorch));
        // The same impact cuts the same crater.
        assert_eq!(cells, crater([0.5, 64.0, 0.5], 4.0, 7));
    }

    #[test]
    fn the_wave_takes_longer_from_farther() {
        assert_eq!(wave_delay(0.0), 0);
        assert_eq!(wave_delay(17.0), 10);
        assert!(wave_delay(200.0) > wave_delay(100.0));
        assert_eq!(wave_delay(1.0e9), MAX_DELAY);
        assert_eq!(wave_delay(f64::NAN), 0);
    }

    #[test]
    fn the_shake_dies_away_with_time_and_distance() {
        let r = ShellKind::Howitzer.radius();
        assert!(shake_strength(r, 10.0, 0.0) > shake_strength(r, 100.0, 0.0));
        assert!(shake_strength(r, 10.0, 0.0) > shake_strength(r, 10.0, 20.0));
        assert_eq!(shake_strength(r, 10.0, -1.0), 0.0);
        assert_eq!(shake_strength(r, shake_reach(r), 0.0), 0.0);
        assert!(shake_strength(r, 0.0, 0.0) <= 1.0);
    }

    #[test]
    fn debris_flies_out_and_up() {
        let c = [0.5, 64.0, 0.5];
        let v = debris_velocity([3, 64, 0], c, 4.0, 1);
        assert!(v[0] > 0.0 && v[1] > 0.0 && v[2].abs() < 1e-9);
        assert!(!is_debris([0, 58, 0], c, 1, 1.0), "too deep to fly");
        assert!(is_debris([0, 64, 0], c, 1, 1.0));
    }

    #[test]
    fn heading_follows_the_velocity() {
        let (yaw, pitch) = heading([0.0, 0.0, 1.0]);
        assert!(yaw.abs() < 1e-4 && pitch.abs() < 1e-4);
        let (yaw, _) = heading([-1.0, 0.0, 0.0]);
        assert!((yaw - 90.0).abs() < 1e-4);
        let (_, pitch) = heading([0.0, -1.0, 0.0]);
        assert!((pitch - 90.0).abs() < 1e-4);
    }

    #[test]
    fn an_impact_round_trips() {
        let i = Impact {
            at: [12.5, -3.0, 1.0e6],
            kind: ShellKind::Field,
        };
        assert_eq!(Impact::decode(&i.encode()), Some(i));
        assert_eq!(Impact::decode(&[9; 25]), None);
        assert_eq!(Impact::decode(&[0; 3]), None);
    }

    #[test]
    fn commands_parse() {
        assert_eq!(parse("tp 0 0 0"), None);
        assert_eq!(
            parse("ironfall fire field"),
            Some(Ok(Command::Fire(ShellKind::Field)))
        );
        assert_eq!(
            parse("ironfall barrage howitzer 6"),
            Some(Ok(Command::Barrage(ShellKind::Howitzer, 6)))
        );
        assert!(matches!(parse("ironfall barrage mortar 40"), Some(Err(_))));
        assert!(matches!(parse("ironfall fire nuke"), Some(Err(_))));
        assert_eq!(parse("ironfall clear"), Some(Ok(Command::Clear)));
    }
}
