use super::*;

/// Generator output at full load, in kW.
pub const GENERATOR_KW: f32 = 600.0;
pub const SOLAR_KW: f32 = 40.0;
/// Tiles a pole powers in each direction beyond its own tile.
pub const POLE_SUPPLY: i32 = 3;
/// Maximum wire length between poles, in tiles.
pub const WIRE_REACH: f32 = 8.0;
/// Each pole wires itself to at most this many nearby poles.
const MAX_WIRES: usize = 4;

#[derive(Default, Clone)]
pub struct Net {
    /// kW requested last tick.
    pub demand: f32,
    /// kW available last tick.
    pub supply: f32,
    /// Fraction of demand met, 0..=1.
    pub sat: f32,
}

#[derive(Default)]
pub struct Power {
    pub dirty: bool,
    pub nets: Vec<Net>,
    /// Wires between pole centres, for drawing.
    pub wires: Vec<(Vec2, Vec2)>,
}

impl World {
    pub(super) fn recompute_power(&mut self) {
        self.power.dirty = false;
        let poles: Vec<(usize, IVec2)> = self
            .entities
            .iter()
            .enumerate()
            .filter_map(|(id, e)| e.as_ref().filter(|e| e.kind == Kind::Pole).map(|e| (id, e.pos)))
            .collect();

        let mut parent: Vec<usize> = (0..poles.len()).collect();
        fn find(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        let mut wired = std::collections::BTreeSet::new();
        for a in 0..poles.len() {
            let mut near: Vec<(f32, usize)> = (0..poles.len())
                .filter(|&b| b != a)
                .map(|b| (poles[a].1.as_vec2().distance(poles[b].1.as_vec2()), b))
                .filter(|(d, _)| *d <= WIRE_REACH)
                .collect();
            near.sort_by(|x, y| x.0.total_cmp(&y.0));
            for &(_, b) in near.iter().take(MAX_WIRES) {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent[ra] = rb;
                }
                wired.insert((a.min(b), a.max(b)));
            }
        }

        let half = Vec2::splat(0.5);
        self.power.wires = wired
            .into_iter()
            .map(|(a, b)| (poles[a].1.as_vec2() + half, poles[b].1.as_vec2() + half))
            .collect();

        let mut net_of_root = vec![usize::MAX; poles.len()];
        let mut count = 0;
        let mut pole_net = vec![0; poles.len()];
        for (k, net) in pole_net.iter_mut().enumerate() {
            let r = find(&mut parent, k);
            if net_of_root[r] == usize::MAX {
                net_of_root[r] = count;
                count += 1;
            }
            *net = net_of_root[r];
        }
        self.power.nets = vec![Net { sat: 1.0, ..Default::default() }; count];

        // Which network supplies each tile (first pole wins where areas overlap).
        let mut supply = vec![None; (MAP_W * MAP_H) as usize];
        for (k, &(_, p)) in poles.iter().enumerate() {
            for y in -POLE_SUPPLY..=POLE_SUPPLY {
                for x in -POLE_SUPPLY..=POLE_SUPPLY {
                    let t = p + ivec2(x, y);
                    if in_bounds(t) && supply[idx(t)].is_none() {
                        supply[idx(t)] = Some(pole_net[k]);
                    }
                }
            }
        }
        for e in self.entities.iter_mut().flatten() {
            e.net = e.tiles().find_map(|t| supply[idx(t)]);
        }
    }

    /// Speed multiplier from the entity's power supply. Unpowered-by-design entities get 1.
    pub fn power_factor(&self, id: usize) -> f32 {
        let e = self.entity(id);
        if e.kind.draw_kw() <= 0.0 {
            return 1.0;
        }
        e.net.and_then(|n| self.power.nets.get(n)).map_or(0.0, |n| n.sat)
    }

    pub(super) fn update_power(&mut self, dt: f32) {
        let n = self.power.nets.len();
        let mut demand = vec![0.0f32; n];
        let mut solar = vec![0.0f32; n];
        let mut gens: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (id, e) in self.entities.iter().enumerate() {
            let Some(e) = e else { continue };
            let Some(net) = e.net else { continue };
            match &e.state {
                State::Generator { fuel, energy, .. } => {
                    if *energy > 0.0 || fuel.total() > 0 {
                        gens[net].push(id);
                    }
                }
                State::Solar => solar[net] += SOLAR_KW,
                _ if e.active => demand[net] += e.kind.draw_kw(),
                _ => {}
            }
        }
        for net in 0..n {
            let supply = solar[net] + gens[net].len() as f32 * GENERATOR_KW;
            let d = demand[net];
            let sat = if d <= 0.0 {
                if supply > 0.0 { 1.0 } else { 0.0 }
            } else {
                (supply / d).min(1.0)
            };
            self.power.nets[net] = Net { demand: d, supply, sat };

            // Solar covers what it can; generators share the rest.
            let from_gens = (d.min(supply) - solar[net]).max(0.0);
            let per_gen = if gens[net].is_empty() { 0.0 } else { from_gens / gens[net].len() as f32 };
            for &g in &gens[net] {
                let e = self.entity_mut(g);
                let State::Generator { fuel, energy, load } = &mut e.state else {
                    continue;
                };
                *load = per_gen / GENERATOR_KW;
                *energy -= per_gen * 1000.0 * dt;
                while *energy <= 0.0 {
                    let Some(item) = [Item::FuelCell, Item::Coal]
                        .into_iter()
                        .find(|&i| fuel.get(i) > 0)
                    else {
                        *energy = 0.0;
                        break;
                    };
                    fuel.take(item, 1);
                    *energy += item.fuel_value().unwrap_or(0.0);
                }
                e.active = *load > 0.0;
            }
        }
        // Generators outside any network, or with nothing to burn, sit idle.
        for e in self.entities.iter_mut().flatten() {
            if let State::Generator { load, energy, fuel } = &mut e.state
                && (e.net.is_none() || (*energy <= 0.0 && fuel.total() == 0))
            {
                *load = 0.0;
                e.active = false;
            }
        }
    }
}
