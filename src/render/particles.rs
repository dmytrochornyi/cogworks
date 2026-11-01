use std::f32::consts::TAU;

use macroquad::prelude::*;

use crate::theme;
use crate::world::Kind;

use super::WIND;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum PKind {
    Steam,
    Firefly,
    Mote,
    Spark,
    Butterfly,
    Petal,
    BirdShadow,
}

/// Particle caps for the Task 8b "life" pass. Butterflies and petals drift by
/// day, steam rises from active machines, and each kind is hard-capped so the
/// world never fills with particles. The cap guarantees are unit-tested.
pub const MAX_BUTTERFLIES: usize = 6;
pub const MAX_PETALS: usize = 12;
pub const MAX_STEAM: usize = 40;

/// Pastel wing colours for butterflies, picked deterministically from the seed.
const BUTTERFLY_COLORS: [u32; 4] = [theme::FLOWER_PINK, theme::FLOWER_BUTTER, theme::FLOWER_LILAC, theme::SKY];

pub(super) fn butterfly_color(seed: f32) -> Color {
    let i = (seed * 3.0).rem_euclid(BUTTERFLY_COLORS.len() as f32) as usize;
    theme::color(BUTTERFLY_COLORS[i])
}

pub(super) struct Particle {
    pub(super) pos: Vec2,
    vel: Vec2,
    pub(super) life: f32,
    pub(super) max: f32,
    pub(super) size: f32,
    pub(super) kind: PKind,
    pub(super) seed: f32,
}

/// xorshift64* in 0..1, shared by the renderer and the pure spawn function so
/// the particle system stays deterministic and testable.
fn rand01(rng: &mut u64) -> f32 {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 7;
    *rng ^= *rng << 17;
    (*rng >> 40) as f32 / (1u64 << 24) as f32
}

/// Everything the per-frame spawner needs, free of any window state so the cap
/// guarantees can be unit-tested. `vents` lists the active machines' steam
/// sources (world position, puffs/second), `furnaces` their smelter furnace
/// mouths and `shots` the muzzle positions of shots fired this frame.
pub(super) struct SpawnEnv<'a> {
    pub(super) dt: f32,
    pub(super) night: f32,
    pub(super) cam_pos: Vec2,
    pub(super) half: Vec2,
    pub(super) vents: &'a [(Vec2, f32)],
    pub(super) furnaces: &'a [Vec2],
    pub(super) shots: &'a [Vec2],
}

/// Steam puff rate (puffs/second) per active machine, roughly matching how
/// busy each machine reads.
pub(super) fn vent_rate(kind: Kind) -> f32 {
    match kind {
        Kind::Smelter => 3.0,
        Kind::Generator => 2.0,
        Kind::Refinery => 1.5,
        Kind::Assembler => 2.0,
        Kind::Lab => 1.2,
        Kind::Mixer => 1.5,
        Kind::Drill => 1.2,
        Kind::Wellhead => 1.0,
        Kind::GrandEngine => 2.5,
        _ => 1.0,
    }
}

fn kind_count(particles: &[Particle], kind: PKind) -> usize {
    particles.iter().filter(|p| p.kind == kind).count()
}

/// Advances every particle and drops the dead ones. Pure, so the cap test can
/// drive a whole minute of frames without a window.
pub(super) fn advance_particles(particles: &mut Vec<Particle>, dt: f32) {
    for p in particles.iter_mut() {
        p.life -= dt;
        p.pos += p.vel * dt;
        match p.kind {
            PKind::Steam => {
                p.vel += WIND * dt * 0.5 + vec2(0.0, -0.25) * dt;
                p.vel.x *= 1.0 - dt * 0.4;
            }
            PKind::Firefly | PKind::Mote => {
                p.vel = Vec2::from_angle(p.seed + p.life * 0.9) * if p.kind == PKind::Mote { 0.25 } else { 0.5 };
            }
            PKind::Spark => p.vel *= 0.9,
            PKind::Butterfly => p.vel = Vec2::from_angle(p.seed + p.life * 0.5) * 0.5,
            PKind::Petal => {
                p.vel.x = (p.seed + p.life * 0.7).sin() * 0.3;
                p.vel.y = -0.15;
            }
            PKind::BirdShadow => {}
        }
    }
    particles.retain(|p| p.life > 0.0);
}

/// Spawns this frame's particles, keeping every kind under its cap. Pure.
pub(super) fn spawn_particles(rng: &mut u64, particles: &mut Vec<Particle>, env: &SpawnEnv, bird_timer: &mut f32) {
    let day = 1.0 - env.night;

    // Steam rises from active machines' vents, capped.
    if kind_count(particles, PKind::Steam) < MAX_STEAM {
        for &(vent, rate) in env.vents {
            if kind_count(particles, PKind::Steam) >= MAX_STEAM {
                break;
            }
            if rand01(rng) < rate * env.dt {
                let (r1, r2) = (rand01(rng), rand01(rng));
                particles.push(Particle {
                    pos: vent,
                    vel: vec2(r1 * 0.3 - 0.15, -0.4 - r2 * 0.3),
                    life: 2.0 + r1,
                    max: 3.0,
                    size: 0.4 + r1 * 0.25,
                    kind: PKind::Steam,
                    seed: r2,
                });
            }
        }
    }

    // An occasional spark from a smelter's glowing furnace mouth.
    for &mouth in env.furnaces {
        if rand01(rng) < 0.5 * env.dt {
            let (r1, r2) = (rand01(rng), rand01(rng));
            particles.push(Particle {
                pos: mouth,
                vel: Vec2::from_angle(r1 * TAU) * (0.4 + r2 * 0.6),
                life: 0.3,
                max: 0.3,
                size: 0.05,
                kind: PKind::Spark,
                seed: r1,
            });
        }
    }

    // Fireflies at night, golden dust by day, around the camera.
    for (kind, want) in [(PKind::Firefly, (env.night * 20.0) as usize), (PKind::Mote, ((1.0 - env.night) * 22.0) as usize)] {
        if kind_count(particles, kind) < want {
            let (r1, r2, r3) = (rand01(rng), rand01(rng), rand01(rng));
            let pos = env.cam_pos + vec2(r1 * 2.0 - 1.0, r2 * 2.0 - 1.0) * env.half;
            particles.push(Particle { pos, vel: Vec2::ZERO, life: 6.0 + r3 * 6.0, max: 12.0, size: 0.08, kind, seed: r3 * TAU });
        }
    }

    // Butterflies wander near the camera by day.
    let want_bf = (day * MAX_BUTTERFLIES as f32) as usize;
    if kind_count(particles, PKind::Butterfly) < want_bf {
        let (r1, r2, r3) = (rand01(rng), rand01(rng), rand01(rng));
        let pos = env.cam_pos + vec2(r1 * 2.0 - 1.0, r2 * 2.0 - 1.0) * env.half;
        particles.push(Particle { pos, vel: Vec2::ZERO, life: 14.0 + r3 * 12.0, max: 26.0, size: 0.4 + r2 * 0.2, kind: PKind::Butterfly, seed: r3 * TAU });
    }

    // Drifting petals by day.
    let want_pet = (day * MAX_PETALS as f32) as usize;
    if kind_count(particles, PKind::Petal) < want_pet {
        let (r1, r2, r3) = (rand01(rng), rand01(rng), rand01(rng));
        let pos = env.cam_pos + vec2(r1 * 2.0 - 1.0, r2 * 2.0 - 1.0) * env.half;
        particles.push(Particle { pos, vel: Vec2::ZERO, life: 10.0 + r3 * 8.0, max: 18.0, size: 0.22 + r2 * 0.1, kind: PKind::Petal, seed: r3 * TAU });
    }

    // Muzzle sparks from shots.
    for &to in env.shots {
        for _ in 0..2 {
            let (r1, r2) = (rand01(rng), rand01(rng));
            particles.push(Particle {
                pos: to,
                vel: Vec2::from_angle(r1 * TAU) * (1.0 + r2 * 2.0),
                life: 0.25,
                max: 0.25,
                size: 0.06,
                kind: PKind::Spark,
                seed: r1,
            });
        }
    }

    // A soft bird shadow crosses the view now and then, by day only.
    *bird_timer -= env.dt;
    if env.night < 0.3 && *bird_timer <= 0.0 && kind_count(particles, PKind::BirdShadow) == 0 {
        let (r1, r2) = (rand01(rng), rand01(rng));
        let from_left = r2 < 0.5;
        let speed = 2.5;
        let width = env.half.x * 2.0 + 6.0;
        let x = if from_left { env.cam_pos.x - env.half.x - 3.0 } else { env.cam_pos.x + env.half.x + 3.0 };
        let y = env.cam_pos.y + (r1 * 2.0 - 1.0) * env.half.y * 0.7;
        particles.push(Particle {
            pos: vec2(x, y),
            vel: vec2(if from_left { speed } else { -speed }, 0.0),
            life: width / speed,
            max: width / speed,
            size: 3.5,
            kind: PKind::BirdShadow,
            seed: if from_left { 0.0 } else { 1.0 },
        });
        *bird_timer = 20.0 + rand01(rng) * 20.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_stay_capped() {
        // 30 active machines: 30 steam vents and 30 smelter furnace mouths.
        let vents: Vec<(Vec2, f32)> = (0..30).map(|i| (vec2(i as f32, 0.0), 3.0)).collect();
        let furnaces: Vec<Vec2> = (0..30).map(|i| vec2(i as f32, 0.0)).collect();
        let mut rng = 0x9e37_79b9_u64;
        let dt = 1.0 / 60.0;
        for night in [0.0f32, 1.0] {
            let mut particles: Vec<Particle> = Vec::new();
            let mut bird = 30.0f32;
            for _ in 0..(60.0 / dt) as usize {
                let env = SpawnEnv {
                    dt,
                    night,
                    cam_pos: Vec2::ZERO,
                    half: vec2(20.0, 12.0),
                    vents: &vents,
                    furnaces: &furnaces,
                    shots: &[],
                };
                spawn_particles(&mut rng, &mut particles, &env, &mut bird);
                advance_particles(&mut particles, dt);
            }
            let count = |k: PKind| particles.iter().filter(|p| p.kind == k).count();
            assert!(count(PKind::Steam) <= MAX_STEAM, "steam exceeded cap at night={night}");
            assert!(count(PKind::Butterfly) <= MAX_BUTTERFLIES, "butterflies exceeded cap at night={night}");
            assert!(count(PKind::Petal) <= MAX_PETALS, "petals exceeded cap at night={night}");
            assert!(count(PKind::Firefly) <= 20, "fireflies exceeded cap at night={night}");
            assert!(count(PKind::Mote) <= 22, "motes exceeded cap at night={night}");
            assert!(count(PKind::BirdShadow) <= 1, "bird shadows exceeded cap at night={night}");
        }
    }
}
