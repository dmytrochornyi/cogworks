//! Trees: scenery that blocks building until the player chops it down for wood.

use super::generate::Rng;
use super::*;

/// Wood a freshly grown tree yields before it falls.
pub const TREE_WOOD: u8 = 4;
/// Tree kinds, matching the `tree_<kind>` sprites: oak, pine, birch, bush.
pub const TREE_KINDS: u8 = 4;

impl World {
    /// The kind and remaining wood of the tree on `p`, if one grows there.
    /// Trees are stored per tile as `1 + kind * 16 + wood`; 0 is no tree.
    pub fn tree_at(&self, p: IVec2) -> Option<(u8, u8)> {
        if !in_bounds(p) {
            return None;
        }
        let v = *self.trees.get(idx(p))?;
        (v > 0).then(|| ((v - 1) / 16, (v - 1) % 16))
    }

    pub fn has_tree(&self, p: IVec2) -> bool {
        self.tree_at(p).is_some()
    }

    fn set_tree(&mut self, p: IVec2, tree: Option<(u8, u8)>) {
        if self.trees.len() != (MAP_W * MAP_H) as usize {
            self.trees = vec![0; (MAP_W * MAP_H) as usize];
        }
        self.trees[idx(p)] = tree.map_or(0, |(kind, wood)| 1 + kind * 16 + wood);
    }

    /// One chop's worth of wood from the tree on `p`. The tree falls (and is
    /// removed) when its last wood is taken. Returns whether wood was taken.
    pub(super) fn chop(&mut self, p: IVec2) -> bool {
        let Some((kind, wood)) = self.tree_at(p) else {
            return false;
        };
        self.player.inv.add(Item::Wood, 1);
        self.fx.push(Fx::Mined { tile: p, item: Item::Wood });
        if wood <= 1 {
            self.set_tree(p, None);
            self.fx.push(Fx::Removed { pos: p, size: IVec2::ONE });
        } else {
            self.set_tree(p, Some((kind, wood - 1)));
        }
        true
    }

    /// Grows forests and scattered trees on open grass, away from ore, tar
    /// and the landing spot.
    pub(super) fn grow_trees(&mut self, rng: &mut Rng) {
        self.trees = vec![0; (MAP_W * MAP_H) as usize];
        let center = self.spawn;
        let plant = |w: &mut World, p: IVec2, kind: u8| {
            if in_bounds(p) && w.res[idx(p)].is_none() && p.as_vec2().distance(center) > 7.0 {
                w.set_tree(p, Some((kind, TREE_WOOD)));
            }
        };
        // Forests: pine stands, or mixed oak and birch woods with bushes.
        for _ in 0..110 {
            let c = vec2(rng.range(2.0, MAP_W as f32 - 2.0), rng.range(2.0, MAP_H as f32 - 2.0));
            if c.distance(center) < 12.0 {
                continue;
            }
            let r = rng.range(2.5, 7.0);
            let pines = rng.range(0.0, 1.0) < 0.4;
            let ri = r.ceil() as i32;
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    let d = vec2(dx as f32, dy as f32).length() / r;
                    // Dense in the middle, thinning toward the edge.
                    if d > 1.0 || rng.range(0.0, 1.0) > 0.55 * (1.0 - d * d) + 0.08 {
                        continue;
                    }
                    let roll = rng.range(0.0, 1.0);
                    let kind = if pines {
                        if roll < 0.85 { 1 } else { 3 }
                    } else if roll < 0.5 {
                        0
                    } else if roll < 0.8 {
                        2
                    } else {
                        3
                    };
                    plant(self, c.as_ivec2() + ivec2(dx, dy), kind);
                }
            }
        }
        // A sprinkling of lone trees and bushes across the meadows.
        for _ in 0..500 {
            let p = ivec2(rng.range(0.0, MAP_W as f32) as i32, rng.range(0.0, MAP_H as f32) as i32);
            let kind = (rng.next() % TREE_KINDS as u64) as u8;
            plant(self, p, kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trees_grow_off_ore_and_away_from_spawn() {
        let w = World::new(7);
        let mut count = 0;
        for y in 0..MAP_H {
            for x in 0..MAP_W {
                let p = ivec2(x, y);
                if w.has_tree(p) {
                    count += 1;
                    assert!(w.res_at(p).is_none(), "tree on a resource at {p}");
                    assert!(p.as_vec2().distance(w.spawn) > 7.0, "tree on the landing spot at {p}");
                }
            }
        }
        assert!(count > 800, "the map should have forests: {count} trees");
    }

    #[test]
    fn chopping_yields_wood_then_fells_the_tree() {
        let mut w = World::new(7);
        let p = (0..MAP_H)
            .flat_map(|y| (0..MAP_W).map(move |x| ivec2(x, y)))
            .find(|&p| w.has_tree(p))
            .unwrap();
        let before = w.player.inv.get(Item::Wood);
        for _ in 0..TREE_WOOD {
            assert!(w.chop(p));
        }
        assert_eq!(w.player.inv.get(Item::Wood), before + TREE_WOOD as u32);
        assert!(!w.has_tree(p), "the tree falls after its last wood");
        assert!(!w.chop(p));
    }

    #[test]
    fn trees_block_building() {
        let w = World::new(7);
        let p = (0..MAP_H)
            .flat_map(|y| (0..MAP_W).map(move |x| ivec2(x, y)))
            .find(|&p| w.has_tree(p) && w.entity_at(p).is_none())
            .unwrap();
        assert!(w.check_place(Kind::Chest, p, Dir::N).is_err());
    }

    #[test]
    fn old_saves_without_trees_have_none() {
        let mut w = World::new(7);
        w.trees.clear();
        assert!(!w.has_tree(ivec2(10, 10)));
    }
}
