use super::*;

/// Fluid each pipe tile holds.
pub const PIPE_CAP: f32 = 100.0;

#[derive(Clone, Default)]
pub struct FluidNet {
    pub fluid: Option<Fluid>,
    pub amount: f32,
    pub cap: f32,
    pipes: Vec<usize>,
}

/// Connected pipe groups. Each group behaves as one shared tank.
#[derive(Default)]
pub struct FluidNets {
    pub dirty: bool,
    pub nets: Vec<FluidNet>,
}

impl World {
    /// Writes network contents back into the individual pipes, e.g. before saving or re-grouping.
    pub fn sync_pipes(&mut self) {
        let nets = std::mem::take(&mut self.fluids.nets);
        for net in &nets {
            let share = net.amount / net.pipes.len().max(1) as f32;
            for &id in &net.pipes {
                if let Some(Entity { state: State::Pipe { fluid, amount }, .. }) = &mut self.entities[id]
                {
                    *fluid = net.fluid;
                    *amount = share;
                }
            }
        }
        self.fluids.nets = nets;
    }

    pub(super) fn recompute_fluids(&mut self) {
        self.sync_pipes();
        self.fluids.dirty = false;
        self.fluids.nets.clear();
        for e in self.entities.iter_mut().flatten() {
            e.fnet = None;
        }
        for start in 0..self.entities.len() {
            match &self.entities[start] {
                Some(e) if e.kind == Kind::Pipe && e.fnet.is_none() => {}
                _ => continue,
            }
            let net_id = self.fluids.nets.len();
            let mut net = FluidNet::default();
            let mut stack = vec![start];
            self.entity_mut(start).fnet = Some(net_id);
            // Amounts per fluid, so a merge of two different fluids keeps the larger.
            let mut totals: Vec<(Fluid, f32)> = Vec::new();
            while let Some(id) = stack.pop() {
                let e = self.entity(id);
                let pos = e.pos;
                if let State::Pipe { fluid: Some(f), amount } = e.state {
                    match totals.iter_mut().find(|(g, _)| *g == f) {
                        Some(t) => t.1 += amount,
                        None => totals.push((f, amount)),
                    }
                }
                net.pipes.push(id);
                for d in Dir::ALL {
                    if let Some(n) = self.entity_at(pos + d.vec())
                        && self.entity(n).kind == Kind::Pipe
                        && self.entity(n).fnet.is_none()
                    {
                        self.entity_mut(n).fnet = Some(net_id);
                        stack.push(n);
                    }
                }
            }
            net.cap = net.pipes.len() as f32 * PIPE_CAP;
            if let Some(&(f, a)) = totals.iter().max_by(|a, b| a.1.total_cmp(&b.1)) {
                net.fluid = Some(f);
                net.amount = a.min(net.cap);
            }
            self.fluids.nets.push(net);
        }
    }

    /// The pipe network occupying `tile`, if any.
    pub fn pipe_net(&self, tile: IVec2) -> Option<usize> {
        self.entity_ref_at(tile).filter(|e| e.kind == Kind::Pipe).and_then(|e| e.fnet)
    }

    /// Adds fluid to a network; returns how much it took.
    fn fluid_push(&mut self, net: usize, fluid: Fluid, amount: f32) -> f32 {
        let n = &mut self.fluids.nets[net];
        if n.fluid.is_some_and(|f| f != fluid) {
            return 0.0;
        }
        let moved = amount.min(n.cap - n.amount).max(0.0);
        if moved > 0.0 {
            n.fluid = Some(fluid);
            n.amount += moved;
        }
        moved
    }

    /// Removes up to `max` of `fluid` from a network; returns how much it gave.
    fn fluid_pull(&mut self, net: usize, fluid: Fluid, max: f32) -> f32 {
        let n = &mut self.fluids.nets[net];
        if n.fluid != Some(fluid) {
            return 0.0;
        }
        let moved = max.min(n.amount).max(0.0);
        n.amount -= moved;
        if n.amount < 0.01 {
            n.amount = 0.0;
            n.fluid = None;
        }
        moved
    }

    /// Fills a crafter's fluid tank from the pipe at its input port (behind it).
    pub(super) fn pull_fluid_input(&mut self, id: usize) {
        let e = self.entity(id);
        let State::Crafter(c) = &e.state else {
            return;
        };
        let Some((fluid, need)) = c.recipe.and_then(|r| RECIPES[r].fluid_in) else {
            return;
        };
        let Some(net) = self.pipe_net(e.back()) else {
            return;
        };
        let room = need * 2.0 - c.fluid_in;
        if room <= 0.0 {
            return;
        }
        let got = self.fluid_pull(net, fluid, room);
        if let State::Crafter(c) = &mut self.entity_mut(id).state {
            c.fluid_in += got;
        }
    }

    /// Empties a building's output tank into the pipe at its front.
    pub(super) fn push_fluid_output(&mut self, id: usize) {
        let e = self.entity(id);
        let (fluid, amount) = match &e.state {
            State::Wellhead { amount } => (Fluid::Tar, *amount),
            State::Crafter(c) => match c.recipe.and_then(|r| RECIPES[r].fluid_out) {
                Some((f, _)) => (f, c.fluid_out),
                None => return,
            },
            _ => return,
        };
        if amount <= 0.0 {
            return;
        }
        let Some(net) = self.pipe_net(e.front()) else {
            return;
        };
        let moved = self.fluid_push(net, fluid, amount);
        match &mut self.entity_mut(id).state {
            State::Wellhead { amount } => *amount -= moved,
            State::Crafter(c) => c.fluid_out -= moved,
            _ => {}
        }
    }

    /// Whether a pipe at `pipe` connects to the building at `other` through one of its ports.
    pub fn pipe_port_link(&self, pipe: IVec2, other: IVec2) -> bool {
        let Some(e) = self.entity_ref_at(other) else {
            return false;
        };
        (e.kind.has_fluid_in() && e.back() == pipe) || (e.kind.has_fluid_out() && e.front() == pipe)
    }
}
