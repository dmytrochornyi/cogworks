use super::logistics::Src;
use super::*;

/// A world with no resources, trees, buildings or enemies, and the player out of the way.
fn blank() -> World {
    let mut w = World::new(7);
    w.res.fill(None);
    w.trees.clear();
    for id in 0..w.entities.len() {
        w.delete(id);
    }
    w.player.pos = vec2(250.0, 250.0);
    w.peaceful = true;
    w
}

fn run(w: &mut World, seconds: f32) {
    for _ in 0..(seconds * 60.0) as usize {
        w.update(1.0 / 60.0);
    }
}

fn build(w: &mut World, kind: Kind, pos: IVec2, dir: Dir) -> usize {
    w.player.inv.add(kind.item().unwrap(), 1);
    w.place(kind, pos, dir).unwrap();
    w.entity_at(pos).unwrap()
}

fn lanes(w: &World, pos: IVec2) -> [usize; 2] {
    match &w.entity(w.entity_at(pos).unwrap()).state {
        State::Belt(b) => [b.lanes[0].len(), b.lanes[1].len()],
        _ => panic!("not a belt"),
    }
}

fn chest_count(w: &World, pos: IVec2, item: Item) -> u32 {
    match &w.entity(w.entity_at(pos).unwrap()).state {
        State::Chest { stock } => stock.get(item),
        _ => panic!("not a chest"),
    }
}

/// A coal generator with a pole covering `area_center`.
fn power_up(w: &mut World, at: IVec2) {
    let g = build(w, Kind::Generator, at, Dir::N);
    if let State::Generator { fuel, .. } = &mut w.entity_mut(g).state {
        fuel.add(Item::Coal, 10);
    }
    build(w, Kind::Pole, at + ivec2(2, 0), Dir::N);
}

/// Sets a pipe's stored fluid and flags the fluid nets for recomputation.
fn set_pipe(w: &mut World, id: usize, fluid: Option<Fluid>) {
    if let State::Pipe { fluid: f, amount } = &mut w.entity_mut(id).state {
        *f = fluid;
        *amount = if fluid.is_some() { 100.0 } else { 0.0 };
    }
    w.fluids.dirty = true;
}

/// Total fluid held across every live pipe.
fn pipe_total(w: &World) -> f32 {
    w.entities
        .iter()
        .flatten()
        .filter_map(|e| match &e.state {
            State::Pipe { amount, .. } => Some(*amount),
            _ => None,
        })
        .sum()
}

#[test]
fn belts_carry_items_into_a_chest() {
    let mut w = blank();
    for x in 10..13 {
        build(&mut w, Kind::Belt, ivec2(x, 10), Dir::E);
    }
    build(&mut w, Kind::Chest, ivec2(13, 10), Dir::N);
    for lane in 0..2 {
        assert!(w.give(ivec2(10, 10), Item::Coal, Src { tile: ivec2(9, 10), lane: Some(lane) }));
    }
    run(&mut w, 3.0);
    assert_eq!(chest_count(&w, ivec2(13, 10), Item::Coal), 2);
}

#[test]
fn curves_keep_lanes_and_side_loading_picks_the_near_lane() {
    let mut w = blank();
    let a = build(&mut w, Kind::Belt, ivec2(10, 10), Dir::E);
    let b = build(&mut w, Kind::Belt, ivec2(11, 10), Dir::S);
    assert_eq!(w.curve_side(b), Some(ivec2(-1, 0)));
    if let State::Belt(s) = &mut w.entity_mut(a).state {
        s.lanes[0].push((Item::Coal, 0.9));
    }
    run(&mut w, 0.2);
    assert_eq!(lanes(&w, ivec2(11, 10)), [1, 0], "curve keeps the left lane");

    // With a feeder behind it, the same belt is now side-loaded from its right.
    let mut w = blank();
    let a = build(&mut w, Kind::Belt, ivec2(10, 10), Dir::E);
    build(&mut w, Kind::Belt, ivec2(11, 10), Dir::S);
    build(&mut w, Kind::Belt, ivec2(11, 9), Dir::S);
    if let State::Belt(s) = &mut w.entity_mut(a).state {
        s.lanes[0].push((Item::Coal, 0.9));
    }
    run(&mut w, 0.2);
    assert_eq!(lanes(&w, ivec2(11, 10)), [0, 1], "side-load goes to the near (right) lane");
}

#[test]
fn tunnels_pair_and_carry_items_underground() {
    let mut w = blank();
    build(&mut w, Kind::Tunnel, ivec2(10, 10), Dir::E);
    let exit = build(&mut w, Kind::Tunnel, ivec2(14, 10), Dir::E);
    match &w.entity(exit).state {
        State::Belt(b) => assert!(b.exit && b.partner == Some(ivec2(10, 10))),
        _ => unreachable!(),
    }
    build(&mut w, Kind::Wall, ivec2(12, 10), Dir::N);
    build(&mut w, Kind::Chest, ivec2(15, 10), Dir::N);
    assert!(w.give(ivec2(10, 10), Item::Stone, Src { tile: ivec2(9, 10), lane: Some(0) }));
    run(&mut w, 5.0);
    assert_eq!(chest_count(&w, ivec2(15, 10), Item::Stone), 1);

    // Removing the exit leaves the entrance unpaired again.
    w.deconstruct(ivec2(14, 10));
    let entrance = w.entity_at(ivec2(10, 10)).unwrap();
    match &w.entity(entrance).state {
        State::Belt(b) => assert!(b.partner.is_none() && b.len == 1.0),
        _ => unreachable!(),
    }
}

#[test]
fn splitter_alternates_between_outputs() {
    let mut w = blank();
    build(&mut w, Kind::Belt, ivec2(10, 11), Dir::N);
    build(&mut w, Kind::Splitter, ivec2(10, 10), Dir::N);
    build(&mut w, Kind::Chest, ivec2(10, 9), Dir::N);
    build(&mut w, Kind::Chest, ivec2(11, 9), Dir::N);
    for _ in 0..6 {
        while !w.give(ivec2(10, 11), Item::Gear, Src { tile: ivec2(10, 12), lane: Some(1) }) {
            run(&mut w, 0.1);
        }
    }
    run(&mut w, 3.0);
    assert_eq!(chest_count(&w, ivec2(10, 9), Item::Gear), 3);
    assert_eq!(chest_count(&w, ivec2(11, 9), Item::Gear), 3);
}

#[test]
fn arms_need_power() {
    let mut w = blank();
    let src = build(&mut w, Kind::Chest, ivec2(10, 10), Dir::N);
    build(&mut w, Kind::Arm, ivec2(11, 10), Dir::E);
    build(&mut w, Kind::Chest, ivec2(12, 10), Dir::N);
    if let State::Chest { stock } = &mut w.entity_mut(src).state {
        stock.add(Item::Gear, 5);
    }
    run(&mut w, 3.0);
    assert_eq!(chest_count(&w, ivec2(12, 10), Item::Gear), 0, "no power, no movement");

    power_up(&mut w, ivec2(10, 12));
    run(&mut w, 8.0);
    assert_eq!(chest_count(&w, ivec2(12, 10), Item::Gear), 5);
}

#[test]
fn overloaded_network_slows_down() {
    let mut w = blank();
    power_up(&mut w, ivec2(10, 20));
    build(&mut w, Kind::Pole, ivec2(6, 18), Dir::N);
    // 6 smelters x 120 kW = 720 kW against one 600 kW generator.
    for k in 0..6 {
        let s = build(&mut w, Kind::Smelter, ivec2(4 + k * 2, 16), Dir::N);
        if let State::Crafter(c) = &mut w.entity_mut(s).state {
            c.recipe = Some(recipe_id("Iron Plate"));
            c.input.add(Item::IronOre, 2);
        }
    }
    run(&mut w, 0.5);
    let net = &w.power.nets[0];
    assert!((net.sat - 600.0 / 720.0).abs() < 0.01, "sat was {}", net.sat);
}

#[test]
fn ore_to_plates_production_line() {
    let mut w = blank();
    for y in 10..12 {
        for x in 10..12 {
            w.res[idx(ivec2(x, y))] = Some(Resource::Ore(Item::IronOre));
        }
    }
    power_up(&mut w, ivec2(12, 12));
    // Drill -> belt -> arm -> smelter -> arm -> crate
    build(&mut w, Kind::Drill, ivec2(10, 10), Dir::E);
    build(&mut w, Kind::Belt, ivec2(12, 10), Dir::E);
    build(&mut w, Kind::Arm, ivec2(13, 10), Dir::E);
    build(&mut w, Kind::Smelter, ivec2(14, 10), Dir::N);
    build(&mut w, Kind::Arm, ivec2(16, 10), Dir::E);
    build(&mut w, Kind::Chest, ivec2(17, 10), Dir::N);
    run(&mut w, 30.0);
    let plates = chest_count(&w, ivec2(17, 10), Item::IronPlate);
    assert!(plates >= 12, "only {plates} plates");
    assert!(w.pollution_at(vec2(11.0, 11.0)) > 0.0);
}

#[test]
fn tar_is_pumped_and_refined() {
    let mut w = blank();
    w.res[idx(ivec2(10, 10))] = Some(Resource::Tar);
    power_up(&mut w, ivec2(10, 13));
    build(&mut w, Kind::Wellhead, ivec2(10, 10), Dir::E);
    // Wellhead's front port is at (12, 10).
    for x in 12..15 {
        build(&mut w, Kind::Pipe, ivec2(x, 10), Dir::N);
    }
    // Refinery facing east: back port at (14, 11)... so route a pipe there.
    build(&mut w, Kind::Pipe, ivec2(14, 11), Dir::N);
    let r = build(&mut w, Kind::Refinery, ivec2(15, 10), Dir::E);
    assert_eq!(w.entity(r).back(), ivec2(14, 11));
    let out = w.entity(r).front();
    build(&mut w, Kind::Pipe, out, Dir::N);
    w.set_recipe(r, recipe_id("Naphtha"));
    run(&mut w, 20.0);
    let net = w.pipe_net(out).unwrap();
    let n = &w.fluids.nets[net];
    assert_eq!(n.fluid, Some(Fluid::Naphtha));
    assert!(n.amount > 10.0, "only {} naphtha", n.amount);
}

#[test]
fn labs_complete_research() {
    let mut w = blank();
    power_up(&mut w, ivec2(10, 16));
    let lab = build(&mut w, Kind::Lab, ivec2(10, 12), Dir::N);
    if let State::Lab { input, .. } = &mut w.entity_mut(lab).state {
        input.add(Item::SurveyData, 4);
    }
    w.set_research(0);
    w.research.progress = TECHS[0].units - 2;
    run(&mut w, 25.0);
    assert!(w.research.done[0]);
    assert!(w.recipe_unlocked(recipe_id("Splitter")));
}

#[test]
fn hand_crafting_makes_intermediates() {
    let mut w = blank();
    w.player.inv = Stock::default();
    w.player.inv.add(Item::IronPlate, 3);
    w.queue_craft(recipe_id("Conveyor"), 1).unwrap();
    assert_eq!(w.player.queue.len(), 2, "gear first, then the belt");
    run(&mut w, 2.0);
    assert_eq!(w.player.inv.get(Item::Belt), 2);
    assert_eq!(w.player.inv.get(Item::IronPlate), 0);
    assert!(w.queue_craft(recipe_id("Conveyor"), 1).is_err());
}

#[test]
fn mites_bite_walls_and_sentries_shoot_back() {
    let mut w = blank();
    w.peaceful = false;
    let wall = build(&mut w, Kind::Wall, ivec2(20, 10), Dir::N);
    w.enemies.push(Enemy {
        pos: vec2(18.0, 10.5),
        hp: 30.0,
        brute: false,
        target: vec2(20.5, 10.5),
        home: vec2(0.0, 0.0),
        cooldown: 0.0,
        patience: 60.0,
        phase: 0.0,
    });
    run(&mut w, 3.0);
    assert!(w.entity(wall).hp < Kind::Wall.max_hp());

    let s = build(&mut w, Kind::Sentry, ivec2(20, 14), Dir::N);
    if let State::Sentry { ammo, .. } = &mut w.entity_mut(s).state {
        ammo.add(Item::SlugClip, 2);
    }
    run(&mut w, 3.0);
    assert!(w.enemies.is_empty());
}

#[test]
fn save_round_trip() {
    let mut w = World::new(3);
    w.player.inv.add(Item::Belt, 1);
    let p = w.spawn.as_ivec2() + ivec2(3, 3);
    w.place(Kind::Belt, p, Dir::E).unwrap();
    run(&mut w, 1.0);
    w.sync_pipes();
    let json = serde_json::to_string(&w).unwrap();
    let mut back: World = serde_json::from_str(&json).unwrap();
    back.after_load();
    assert_eq!(back.entity_ref_at(p).map(|e| e.kind), Some(Kind::Belt));
    assert_eq!(back.player.inv, w.player.inv);
    run(&mut back, 1.0);
}

#[test]
fn deconstructing_tunnel_exit_refunds_underground_items() {
    let mut w = blank();
    build(&mut w, Kind::Tunnel, ivec2(10, 10), Dir::E);
    build(&mut w, Kind::Tunnel, ivec2(14, 10), Dir::E);
    // One item is in flight underground, another still sits at the entrance's surface.
    let entrance = w.entity_at(ivec2(10, 10)).unwrap();
    if let State::Belt(b) = &mut w.entity_mut(entrance).state {
        b.lanes[0].push((Item::Coal, 2.5));
        b.lanes[1].push((Item::Stone, 0.5));
    }
    let coal_before = w.player.inv.get(Item::Coal);
    let stone_before = w.player.inv.get(Item::Stone);
    w.deconstruct(ivec2(14, 10));
    assert_eq!(w.player.inv.get(Item::Coal), coal_before + 1, "underground coal refunded");
    assert_eq!(w.player.inv.get(Item::Stone), stone_before, "surface stone stays put");
    match &w.entity(entrance).state {
        State::Belt(b) => {
            assert!(b.partner.is_none() && b.len == 1.0);
            let n: usize = b.lanes.iter().map(Vec::len).sum();
            assert_eq!(n, 1, "only the surface item remains on the unpaired entrance");
        }
        _ => unreachable!(),
    }
}

#[test]
fn deleting_pipe_keeps_network_fluid() {
    let mut w = blank();
    for x in 0..10 {
        build(&mut w, Kind::Pipe, ivec2(10 + x, 10), Dir::N);
    }
    for x in 0..10 {
        let id = w.entity_at(ivec2(10 + x, 10)).unwrap();
        set_pipe(&mut w, id, Some(Fluid::Tar));
    }
    w.update(1.0 / 60.0);
    let net = w.pipe_net(ivec2(10, 10)).unwrap();
    assert_eq!(w.fluids.nets[net].amount, 1000.0, "10 pipes hold 1000 tar");

    // Removing one pipe costs exactly that pipe's share, nothing more.
    let mid = w.entity_at(ivec2(15, 10)).unwrap();
    w.delete(mid);
    assert_eq!(
        w.fluids.nets[net].amount,
        900.0,
        "the network keeps the other nine shares when one pipe is removed"
    );
    // And after the next sync the surviving pipes agree with the network.
    w.update(1.0 / 60.0);
    assert!(
        (pipe_total(&w) - 900.0).abs() < 0.01,
        "remaining {} tar, want 900",
        pipe_total(&w)
    );
}

#[test]
fn changing_recipe_keeps_matching_fluid() {
    // Mixer: Polymer -> Fuel Cell, both Naphtha-in, keeps its fluid tank.
    let mut w = blank();
    let m = build(&mut w, Kind::Mixer, ivec2(10, 10), Dir::N);
    w.set_recipe(m, recipe_id("Polymer"));
    if let State::Crafter(c) = &mut w.entity_mut(m).state {
        c.fluid_in = 20.0;
    }
    w.set_recipe(m, recipe_id("Fuel Cell"));
    match &w.entity(m).state {
        State::Crafter(c) => {
            assert_eq!(c.fluid_in, 20.0, "Naphtha input kept across a Naphtha-in change");
            assert!(w.events.is_empty(), "no fluid was emptied: {:?}", w.events);
        }
        _ => unreachable!(),
    }

    // Refinery: Naphtha (Tar-in, Naphtha-out) -> a Naphtha-in recipe discards both tanks.
    let mut w = blank();
    let r = build(&mut w, Kind::Refinery, ivec2(10, 10), Dir::N);
    w.set_recipe(r, recipe_id("Naphtha"));
    if let State::Crafter(c) = &mut w.entity_mut(r).state {
        c.fluid_in = 30.0;
        c.fluid_out = 25.0;
    }
    w.set_recipe(r, recipe_id("Polymer"));
    match &w.entity(r).state {
        State::Crafter(c) => {
            assert_eq!(c.fluid_in, 0.0, "mismatched input fluid is discarded");
            assert_eq!(c.fluid_out, 0.0, "mismatched output fluid is discarded");
        }
        _ => unreachable!(),
    }
    assert!(w.events.iter().any(|e| e == "Emptied 30 Tar from Refinery"), "events: {:?}", w.events);
    assert!(w.events.iter().any(|e| e == "Emptied 25 Naphtha from Refinery"), "events: {:?}", w.events);
}

#[test]
fn lab_returns_vials_when_research_stops() {
    // Completing the research elsewhere returns a mid-unit lab's consumed vials.
    let mut w = blank();
    let a = build(&mut w, Kind::Lab, ivec2(10, 10), Dir::N);
    w.set_research(0);
    if let State::Lab { input, progress } = &mut w.entity_mut(a).state {
        input.add(Item::SurveyData, 1);
        *progress = 0.5;
    }
    w.research.progress = TECHS[0].units - 1;
    w.add_research(1);
    assert!(w.research.done[0]);
    match &w.entity(a).state {
        State::Lab { input, progress } => {
            assert_eq!(input.get(Item::SurveyData), 2, "consumed vial returned on completion");
            assert_eq!(*progress, 0.0, "lab reset after the research finished");
        }
        _ => unreachable!(),
    }

    // Switching research also returns a mid-unit lab's consumed vials.
    let mut w = blank();
    let a = build(&mut w, Kind::Lab, ivec2(10, 10), Dir::N);
    w.set_research(0);
    if let State::Lab { input, progress } = &mut w.entity_mut(a).state {
        input.add(Item::SurveyData, 1);
        *progress = 0.5;
    }
    w.set_research(1);
    assert_eq!(w.research.current, Some(1));
    match &w.entity(a).state {
        State::Lab { input, progress } => {
            assert_eq!(input.get(Item::SurveyData), 2, "consumed vial returned on switch");
            assert_eq!(*progress, 0.0, "lab reset for the new research");
        }
        _ => unreachable!(),
    }
}

#[test]
fn tunnel_pairs_in_either_order() {
    let mut w = blank();
    // Exit placed first, then the entrance further back.
    let exit = build(&mut w, Kind::Tunnel, ivec2(14, 10), Dir::E);
    let entrance = build(&mut w, Kind::Tunnel, ivec2(10, 10), Dir::E);
    match &w.entity(exit).state {
        State::Belt(b) => assert!(b.exit && b.partner == Some(ivec2(10, 10)), "exit pairs to the entrance behind it"),
        _ => unreachable!(),
    }
    match &w.entity(entrance).state {
        State::Belt(b) => assert!(!b.exit && b.partner == Some(ivec2(14, 10)), "entrance pairs to the exit ahead"),
        _ => unreachable!(),
    }
}

#[test]
fn pipes_refuse_to_mix_fluids() {
    let mut w = blank();
    let a = build(&mut w, Kind::Pipe, ivec2(10, 10), Dir::N);
    let b = build(&mut w, Kind::Pipe, ivec2(12, 10), Dir::N);
    set_pipe(&mut w, a, Some(Fluid::Tar));
    set_pipe(&mut w, b, Some(Fluid::Naphtha));
    w.update(1.0 / 60.0);

    match w.check_place(Kind::Pipe, ivec2(11, 10), Dir::N) {
        Err(e) => assert!(e.contains("Can't mix Tar and Naphtha"), "got {e:?}"),
        Ok(()) => panic!("mixing Tar and Naphtha must be rejected"),
    }

    // Same fluid on both sides is fine.
    let nb = w.pipe_net(ivec2(12, 10)).unwrap();
    w.fluids.nets[nb].fluid = Some(Fluid::Tar);
    assert!(w.check_place(Kind::Pipe, ivec2(11, 10), Dir::N).is_ok(), "matching fluids may connect");

    // An empty network never counts as a fluid.
    w.fluids.nets[nb].fluid = None;
    assert!(w.check_place(Kind::Pipe, ivec2(11, 10), Dir::N).is_ok(), "empty nets may connect");
}

/// Writes a showcase save for eyeballing the renderer:
/// `DEMO_SAVE=/path/demo.json cargo test -- --ignored write_demo_save`
#[test]
#[ignore]
fn write_demo_save() {
    let mut w = blank();
    let o = w.spawn.as_ivec2();
    let at = |x: i32, y: i32| o + ivec2(x, y);
    for y in -7..0 {
        for x in -13..-10 {
            w.res[idx(at(x, y))] = Some(Resource::Ore(Item::IronOre));
        }
    }
    for y in 4..6 {
        for x in 8..10 {
            w.res[idx(at(x, y))] = Some(Resource::Tar);
        }
    }
    for (x, y) in [(-14, 6), (-15, 7), (-14, 7), (-13, 8)] {
        w.res[idx(at(x, y))] = Some(Resource::Ore(Item::CopperOre));
    }
    let fuel = |w: &mut World, id: usize| {
        if let State::Generator { fuel, .. } = &mut w.entity_mut(id).state {
            fuel.add(Item::Coal, 10);
        }
    };
    let g1 = build(&mut w, Kind::Generator, at(-5, -7), Dir::N);
    let g2 = build(&mut w, Kind::Generator, at(-7, -7), Dir::N);
    fuel(&mut w, g1);
    fuel(&mut w, g2);
    for (x, y) in [(-8, -4), (-8, 4), (-3, -2), (2, -2), (6, 3), (10, 7), (12, -2)] {
        build(&mut w, Kind::Pole, at(x, y), Dir::N);
    }
    build(&mut w, Kind::Drill, at(-12, -6), Dir::E);
    build(&mut w, Kind::Drill, at(-12, -3), Dir::E);
    for y in -6..0 {
        build(&mut w, Kind::Belt, at(-10, y), Dir::S);
    }
    for x in -10..-3 {
        build(&mut w, Kind::Belt, at(x, 0), Dir::E);
    }
    for x in [-9, -6] {
        build(&mut w, Kind::Arm, at(x, 1), Dir::S);
        build(&mut w, Kind::Smelter, at(x, 2), Dir::N);
        build(&mut w, Kind::Arm, at(x, 4), Dir::S);
        build(&mut w, Kind::Chest, at(x, 5), Dir::N);
    }
    // Belt showcase: tunnel under a wall, then a splitter.
    for x in -12..-6 {
        build(&mut w, Kind::Belt, at(x, 9), Dir::E);
    }
    build(&mut w, Kind::Tunnel, at(-6, 9), Dir::E);
    build(&mut w, Kind::Wall, at(-4, 9), Dir::N);
    build(&mut w, Kind::Tunnel, at(-2, 9), Dir::E);
    build(&mut w, Kind::Belt, at(-1, 9), Dir::E);
    build(&mut w, Kind::Splitter, at(0, 9), Dir::E);
    for x in 1..5 {
        build(&mut w, Kind::Belt, at(x, 9), Dir::E);
        build(&mut w, Kind::Belt, at(x, 10), Dir::E);
    }
    build(&mut w, Kind::Belt, at(5, 9), Dir::S);
    build(&mut w, Kind::Belt, at(5, 10), Dir::S);
    for k in 0..12 {
        let item = [Item::Gear, Item::Circuit, Item::Coal][k % 3];
        let tile = at(-12 + (k as i32 % 6), 9);
        let _ = w.give(tile, item, Src { tile: tile - ivec2(1, 0), lane: Some(k % 2) });
    }
    // Research and assembly.
    let lab = build(&mut w, Kind::Lab, at(4, -6), Dir::N);
    if let State::Lab { input, .. } = &mut w.entity_mut(lab).state {
        input.add(Item::SurveyData, 4);
    }
    w.set_research(2);
    let asm = build(&mut w, Kind::Assembler, at(4, -2), Dir::N);
    w.set_recipe(asm, recipe_id("Gear"));
    if let State::Crafter(c) = &mut w.entity_mut(asm).state {
        c.input.add(Item::IronPlate, 4);
    }
    // Fluids.
    build(&mut w, Kind::Wellhead, at(8, 4), Dir::E);
    for (x, y) in [(10, 4), (11, 4), (11, 5), (15, 5), (16, 5), (16, 6)] {
        build(&mut w, Kind::Pipe, at(x, y), Dir::N);
    }
    let r = build(&mut w, Kind::Refinery, at(12, 4), Dir::E);
    w.set_recipe(r, recipe_id("Naphtha"));
    // Defenses and the enemy.
    for y in -6..4 {
        build(&mut w, Kind::Wall, at(19, y), Dir::N);
    }
    let s = build(&mut w, Kind::Sentry, at(16, -3), Dir::N);
    if let State::Sentry { ammo, .. } = &mut w.entity_mut(s).state {
        ammo.add(Item::SlugClip, 5);
    }
    w.spawn_entity(Kind::Hive, at(27, -3), Dir::N);
    w.peaceful = false;
    for k in 0..6 {
        w.enemies.push(Enemy {
            pos: at(23, -4 + k).as_vec2(),
            hp: 30.0,
            brute: k == 2,
            target: at(19, -4 + k).as_vec2(),
            home: at(28, -2).as_vec2(),
            cooldown: 0.0,
            patience: 60.0,
            phase: k as f32,
        });
    }
    w.player.pos = at(0, 5).as_vec2() + Vec2::splat(0.5);
    w.player.inv.add(Item::SurveyData, 10);
    w.queue_craft(recipe_id("Conveyor"), 3).unwrap();
    run(&mut w, 6.0);
    let path = std::env::var("DEMO_SAVE").unwrap_or("demo_save.json".into());
    crate::save::save(&mut w, &path).unwrap();
}
