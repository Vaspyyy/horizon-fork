// Ironfall's blasts, drawn over the world. The mod hands the pack up to four
// blasts in frame.mods (native_shader_params), two slots each:
//   mods[2i]     = (world x, y, z of the impact, seconds since it)
//   mods[2i + 1] = (crater radius in blocks, strength 0..1, unused, unused)
// A blast with strength 0 draws nothing. Each one grows a stem and a cap that
// rise and spread for a minute, and a ring of dust that rolls outward along
// the ground at the speed the mod times its boom to.

override steps: i32 = 24;

const DUST_SPEED: f32 = 34.0;   // blocks a second, the mod's shock wave speed
const LIFE: f32 = 70.0;         // seconds until a cloud has thinned away

fn hash3(p: vec3<f32>) -> f32 {
    let q = fract(p * 0.1031);
    let r = q + dot(q, q.yzx + 33.33);
    return fract((r.x + r.y) * r.z);
}

fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(hash3(i), hash3(i + vec3(1.0, 0.0, 0.0)), u.x);
    let b = mix(hash3(i + vec3(0.0, 1.0, 0.0)), hash3(i + vec3(1.0, 1.0, 0.0)), u.x);
    let c = mix(hash3(i + vec3(0.0, 0.0, 1.0)), hash3(i + vec3(1.0, 0.0, 1.0)), u.x);
    let d = mix(hash3(i + vec3(0.0, 1.0, 1.0)), hash3(i + vec3(1.0, 1.0, 1.0)), u.x);
    return mix(mix(a, b, u.y), mix(c, d, u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    return noise3(p) * 0.55 + noise3(p * 2.03) * 0.3 + noise3(p * 4.01) * 0.15;
}

// The cloud's density at `p`, relative to the impact, `age` seconds in.
fn density(p: vec3<f32>, radius: f32, age: f32) -> f32 {
    let grow = 1.0 - exp(-age * 0.35);
    let height = radius * 9.0 * grow;
    let cap_r = radius * (1.2 + 2.6 * grow);
    // The stem: a column that narrows as it rises.
    let h = clamp(p.y / max(height, 0.001), 0.0, 1.0);
    let stem_r = radius * mix(1.1, 0.45, h);
    let stem = (1.0 - smoothstep(stem_r * 0.6, stem_r, length(p.xz))) * step(0.0, p.y) * step(p.y, height);
    // The cap: a flattened ball at the top, rolling.
    let c = p - vec3(0.0, height, 0.0);
    let cap = 1.0 - smoothstep(cap_r * 0.55, cap_r, length(c * vec3(1.0, 1.8, 1.0)));
    // The dust ring along the ground.
    let ring_r = age * DUST_SPEED;
    let ring_w = radius * 1.5 + age * 0.8;
    let ring_h = radius * 1.2 * exp(-age * 0.05);
    let ring = (1.0 - smoothstep(0.0, ring_w, abs(length(p.xz) - ring_r)))
        * (1.0 - smoothstep(0.0, ring_h, abs(p.y))) * exp(-age * 0.08);
    let fluff = fbm(p * (0.9 / radius) + vec3(0.0, -age * 0.25, 0.0));
    let shape = max(max(stem, cap), ring * 0.8);
    let fade = 1.0 - smoothstep(LIFE * 0.4, LIFE, age);
    return clamp(shape * 1.4 - fluff * 0.9, 0.0, 1.0) * fade;
}

// Where the ray `o + t d` (|d| = 1) enters and leaves a sphere at `c`.
fn sphere(o: vec3<f32>, d: vec3<f32>, c: vec3<f32>, r: f32) -> vec2<f32> {
    let oc = o - c;
    let b = dot(oc, d);
    let k = dot(oc, oc) - r * r;
    let disc = b * b - k;
    if disc < 0.0 {
        return vec2(1.0, 0.0);
    }
    let s = sqrt(disc);
    return vec2(max(-b - s, 0.0), -b + s);
}

fn post(input: PostInput) -> vec4<f32> {
    var color = input.color;
    // The ray: from the camera (camera-relative origin) through this pixel.
    let dir = normalize(view_position(input.uv, 1.0));
    var scene = 1.0e6;
    if input.depth > 0.0 {
        scene = length(view_position(input.uv, input.depth));
    }
    let sun = normalize(frame.sun_direction);
    let day = clamp(sun.y * 2.0 + 0.4, 0.15, 1.0);
    for (var i = 0; i < 4; i++) {
        let a = frame.mods[i * 2];
        let b = frame.mods[i * 2 + 1];
        let radius = b.x;
        let strength = b.y;
        let age = a.w;
        if strength <= 0.0 || radius <= 0.0 || age < 0.0 || age > LIFE {
            continue;
        }
        let center = a.xyz - frame.camera_position;
        let reach = max(radius * 14.0, age * DUST_SPEED + radius * 3.0);
        let span = sphere(vec3(0.0), dir, center + vec3(0.0, radius * 4.0, 0.0), reach);
        let t0 = span.x;
        let t1 = min(span.y, scene);
        if t1 <= t0 {
            continue;
        }
        let dt = (t1 - t0) / f32(steps);
        var light = vec3(0.0);
        var trans = 1.0;
        for (var s = 0; s < steps; s++) {
            let t = t0 + (f32(s) + 0.5) * dt;
            let p = dir * t - center;
            let dens = density(p, radius, age) * strength;
            if dens > 0.001 {
                // Lit from above and from the sun, darker low and inside.
                let lit = clamp(0.35 + 0.65 * (p.y / (radius * 9.0)) + 0.25 * dot(sun, normalize(p + vec3(0.001))), 0.15, 1.0);
                let tint = mix(vec3(0.36, 0.31, 0.27), vec3(0.86, 0.83, 0.79), lit) * day;
                // A hot glow at the root in the first seconds.
                let glow = vec3(1.0, 0.45, 0.12) * exp(-age * 0.6) * exp(-max(p.y, 0.0) / radius);
                let absorb = 1.0 - exp(-dens * dt * 0.35);
                light += trans * absorb * (tint + glow * 3.0);
                trans *= 1.0 - absorb;
                if trans < 0.02 {
                    break;
                }
            }
        }
        color = vec4(color.rgb * trans + light, color.a);
    }
    return color;
}
