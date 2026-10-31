// Weather Glass: rain drops, running streaks, snow and frost on glass, drawn
// at 64 pixels per block, from the weather where the player stands.
override drops: bool = true;
override streaks: bool = true;
override frost: bool = true;
override density: f32 = 1.0;

const GLASS: u32 = 1u;   // the manifest's first [[tags]]
const PIXELS: f32 = 64.0;

fn hash(v: vec3<u32>) -> u32 {
    var h = v.x * 0x8da6b343u ^ v.y * 0xd8163841u ^ v.z * 0xcb1ab31fu;
    h ^= h >> 16u;
    h *= 0x7feb352du;
    h ^= h >> 15u;
    h *= 0x846ca68bu;
    h ^= h >> 16u;
    return h;
}

fn rand(v: vec3<i32>, salt: u32) -> f32 {
    return f32(hash(bitcast<vec3<u32>>(v) + vec3<u32>(salt)) >> 8u) / 16777216.0;
}

// What the weather adds at pixel `p` (64 per block, y up) of a glass face whose
// plane is `face`: rgb, and how much of it covers the glass.
struct Wet {
    color: vec3<f32>,
    cover: f32,
}

fn drop_at(p: vec2<i32>, face: i32, amount: f32) -> Wet {
    // One chance of a drop per 6x6 pixel cell, seen from the cells around it.
    var out = Wet(vec3<f32>(0.0), 0.0);
    let cell = vec2<i32>(floor(vec2<f32>(p) / 6.0));
    for (var dy = -1; dy <= 1; dy++) {
        for (var dx = -1; dx <= 1; dx++) {
            let c = cell + vec2<i32>(dx, dy);
            let key = vec3<i32>(c, face);
            if rand(key, 1u) > 0.22 * amount * density { continue; }
            let center = vec2<f32>(c) * 6.0 + vec2<f32>(rand(key, 2u), rand(key, 3u)) * 6.0;
            let radius = 0.8 + rand(key, 4u) * 1.9;
            let d = vec2<f32>(p) + 0.5 - center;
            if length(d) > radius { continue; }
            // A bead: dark rim below, a highlight up and to the left.
            let lit = select(0.62, 0.95, d.x < -0.3 * radius && d.y > 0.3 * radius);
            let rim = select(lit, 0.4, d.y < -0.55 * radius);
            out = Wet(vec3<f32>(rim * 0.85, rim * 0.92, rim), 0.85);
        }
    }
    return out;
}

fn streak_at(p: vec2<i32>, face: i32, amount: f32, time: f32) -> Wet {
    // Every few pixel columns may carry a drop running down; its trail fades above it.
    let column = vec3<i32>(p.x, face, 7);
    if rand(column, 5u) > 0.12 * amount * density { return Wet(vec3<f32>(0.0), 0.0); }
    let speed = 20.0 + rand(column, 6u) * 40.0;   // pixels a second
    let period = 160.0 + rand(column, 7u) * 200.0;
    let head = period - (time * speed + rand(column, 8u) * period) % period;
    let above = (f32(p.y) % period + period) % period - head;
    if above < -1.5 || above > 28.0 { return Wet(vec3<f32>(0.0), 0.0); }
    let fade = select(1.0 - above / 28.0, 1.0, above < 1.5);
    return Wet(vec3<f32>(0.78, 0.86, 0.95), 0.75 * fade);
}

fn frost_at(edge: f32, p: vec2<i32>, face: i32, amount: f32) -> Wet {
    // Frost grows in from the block's edges, ragged, with a few flakes past it.
    let reach = amount * 10.0 + rand(vec3<i32>(p / 4, face), 9u) * 8.0 * amount;
    let speck = rand(vec3<i32>(p, face), 10u);
    if edge < reach && speck < 1.2 - 1.4 * edge / max(reach, 1.0) {
        let white = 0.82 + speck * 0.18;
        return Wet(vec3<f32>(white * 0.92, white * 0.96, white), 0.9);
    }
    if speck > 1.0 - 0.004 * amount * density {
        return Wet(vec3<f32>(0.95, 0.97, 1.0), 0.95);
    }
    return Wet(vec3<f32>(0.0), 0.0);
}

fn material(input: MaterialInput) -> Surface {
    let lit = vec4<f32>(input.lightmap, 1.0);
    let vanilla = input.albedo * input.tint * lit;
    // The face's normal, from how the point moves across the screen (taken before any branch).
    let normal = normalize(cross(dpdx(input.position), dpdy(input.position)));
    if (input.tags & GLASS) == 0u || frame.dimension != 0 || abs(normal.y) > 0.5 {
        return Surface(vanilla);
    }
    let world = input.position + frame.camera_position;
    let x_facing = abs(normal.x) > abs(normal.z);
    let along = select(world.x, world.z, x_facing);
    let plane = select(world.z, world.x, x_facing);
    let face = i32(floor(plane * 4.0)) * 2 + i32(x_facing);
    let p = vec2<i32>(floor(vec2<f32>(along, world.y) * PIXELS));
    // Weather only reaches glass under the open sky.
    let outside = smoothstep(0.6, 0.95, input.light.y);
    let snowing = frame.precipitation == 2u;
    let wet = select(frame.wetness, 0.0, snowing) * outside;
    let cold = select(0.0, frame.wetness, snowing) * outside;
    var add = Wet(vec3<f32>(0.0), 0.0);
    if frost && cold > 0.02 {
        let inside = fract(vec2<f32>(along, world.y));
        let edge = min(min(inside.x, 1.0 - inside.x), min(inside.y, 1.0 - inside.y)) * PIXELS;
        add = frost_at(edge, p, face, cold);
    }
    if streaks && wet > 0.02 && add.cover == 0.0 {
        add = streak_at(p, face, wet, frame.time);
    }
    if drops && wet > 0.02 && add.cover == 0.0 {
        add = drop_at(p, face, wet);
    }
    if add.cover == 0.0 {
        return Surface(vanilla);
    }
    // Clear glass is cut out below half alpha: a covered pixel stays, the glass shows around it.
    let color = vec4<f32>(add.color, 1.0) * lit;
    let alpha = max(vanilla.a, add.cover);
    return Surface(vec4<f32>(mix(vanilla.rgb, color.rgb, add.cover), alpha));
}
