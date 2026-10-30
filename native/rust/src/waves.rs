//! The player's waves and the domain's builders in the crate (spec 39 step 8c-viii), as
//! GDScript's WaveTemplate, WavePresets.line, FormationProduction and DomainProduction keep
//! them: a lane's wave painted unit by unit (`Template`), which of its places are filled and
//! whether it was announced full (`Wave`), and the builders, each building one unit at a time
//! for a lane or the reserve, with the reserve filling the lanes first (`Builders`). Sending a
//! wave into the battle is `field_play`'s.

#[cfg(test)]
pub(crate) mod tests;

/// WaveTemplate.MAX_RANKS.
pub const MAX_RANKS: i32 = 4;

/// A unit painted on a template: its definition, its footprint (depth, width) and its
/// front-most rank and left-most column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Painted {
    pub id: &'static str,
    pub foot: (i32, i32),
    pub at: (i32, i32),
}

/// WaveTemplate: a grid `max_width` columns by MAX_RANKS ranks, painted with no more cells
/// than `slot_limit`.
#[derive(Clone, Debug, PartialEq)]
pub struct Template {
    pub max_width: i32,
    pub slot_limit: i32,
    painted: Vec<Painted>,
}

impl Template {
    pub fn new(width: i32, slots: i32) -> Template {
        Template { max_width: width.clamp(1, 8), slot_limit: slots.max(0), painted: Vec::new() }
    }

    /// WavePresets.line: `count` of `id` (its footprint `foot`), as wide as allowed, then
    /// deeper.
    pub fn line(id: &'static str, foot: (i32, i32), count: i32, width: i32) -> Template {
        let mut template = Template::new(width, count);
        for index in 0..count {
            let wide = template.max_width;
            template.paint(id, foot, (index / wide, index % wide));
        }
        template
    }

    /// Paints `id` anchored at `cell`, replacing the units it overlaps, pulled back inside the
    /// grid; false, with nothing changed, if `cell` is off the grid or the slots run out.
    pub fn paint(&mut self, id: &'static str, foot: (i32, i32), cell: (i32, i32)) -> bool {
        let (depth, width) = foot;
        if cell.0 < 0 || cell.1 < 0 || cell.0 >= MAX_RANKS || cell.1 >= self.max_width {
            return false;
        }
        if depth > MAX_RANKS || width > self.max_width {
            return false;
        }
        let at = (cell.0.min(MAX_RANKS - depth), cell.1.min(self.max_width - width));
        let laid = Painted { id, foot, at };
        let mut kept: Vec<Painted> =
            self.painted.iter().copied().filter(|p| !overlaps(p, &laid)).collect();
        if cells(&kept) + depth * width > self.slot_limit {
            return false;
        }
        kept.push(laid);
        self.painted = kept;
        true
    }

    /// The painted units front-first (rank, then column): the build and fold order.
    pub fn ordered(&self) -> Vec<Painted> {
        let mut sorted = self.painted.clone();
        sorted.sort_by_key(|p| p.at);
        sorted
    }

    /// The squad's width and the units in order, their columns shifted to start at 0.
    pub fn layout(&self) -> (i32, Vec<Painted>) {
        if self.painted.is_empty() {
            return (1, Vec::new());
        }
        let first = self.painted.iter().map(|p| p.at.1).min().unwrap_or(0).min(self.max_width);
        let last = self.painted.iter().map(|p| p.at.1 + p.foot.1).max().unwrap_or(0).max(0);
        let shifted =
            self.ordered().into_iter().map(|p| Painted { at: (p.at.0, p.at.1 - first), ..p });
        (last - first, shifted.collect())
    }
}

fn overlaps(p: &Painted, by: &Painted) -> bool {
    p.at.0 < by.at.0 + by.foot.0
        && by.at.0 < p.at.0 + p.foot.0
        && p.at.1 < by.at.1 + by.foot.1
        && by.at.1 < p.at.1 + p.foot.1
}

fn cells(painted: &[Painted]) -> i32 {
    painted.iter().map(|p| p.foot.0 * p.foot.1).sum()
}

/// FormationProduction's wave: its template, one flag a place (in `ordered` order) for
/// filled, whether it was announced full, whether it departs when full, and the route its
/// squads take (a key into the field's routes).
#[derive(Clone, Debug, PartialEq)]
pub struct Wave {
    pub template: Template,
    pub filled: Vec<bool>,
    pub announced: bool,
    pub auto: bool,
    pub route: usize,
}

impl Wave {
    /// A wave on `template` with nothing filled (set_template on an empty wave).
    pub fn new(template: Template, route: usize) -> Wave {
        let filled = vec![false; template.ordered().len()];
        Wave { template, filled, announced: false, auto: false, route }
    }

    /// Its unfilled places of `id` (wanted()'s count for that type).
    pub fn wanting(&self, id: &str) -> i64 {
        let places = self.template.ordered();
        places.iter().zip(&self.filled).filter(|(p, &f)| !f && p.id == id).count() as i64
    }

    /// Fills its front-most unfilled place of `id`; false if there is none.
    pub fn fill(&mut self, id: &str) -> bool {
        let places = self.template.ordered();
        let open = places.iter().zip(&self.filled).position(|(p, &f)| !f && p.id == id);
        open.map(|k| self.filled[k] = true).is_some()
    }

    pub fn built(&self) -> i64 {
        self.filled.iter().filter(|&&f| f).count() as i64
    }

    pub fn full(&self) -> bool {
        !self.filled.is_empty() && self.filled.iter().all(|&f| f)
    }
}

/// What a builder works for: nothing, the reserve, or a lane (an index into the lanes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Claim {
    Idle,
    Reserve,
    Lane(usize),
}

/// One builder: ticks into its build, how far through it is, and what it builds for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Builder {
    pub ticks: i64,
    pub progress: f64,
    pub claim: Claim,
}

const IDLE: Builder = Builder { ticks: 0, progress: 0.0, claim: Claim::Idle };

/// What the builders and the waves do in a tick, as GDScript's production events tell it:
/// "from_reserve", "built" (into a lane) and "to_reserve" (lane None), then "wave_full" and
/// "departed" (each with the units built).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Produced {
    FromReserve { tick: i64, lane: usize },
    Built { tick: i64, lane: usize },
    ToReserve { tick: i64 },
    WaveFull { tick: i64, built: i64 },
    Departed { tick: i64, units: i64 },
}

/// DomainProduction: builders by unit type (in the order first set), each type's build
/// time, the reserve, its cap, the sharing rule (round robin, or `lane_order` first) and
/// whose turn it is.
#[derive(Clone, Debug, PartialEq)]
pub struct Builders {
    pub kinds: Vec<(&'static str, f64, Vec<Builder>)>,
    pub reserve: Vec<&'static str>,
    pub reserve_cap: usize,
    pub round_robin: bool,
    pub lane_order: Vec<usize>,
    turn: usize,
}

impl Default for Builders {
    fn default() -> Builders {
        Builders {
            kinds: Vec::new(),
            reserve: Vec::new(),
            reserve_cap: 6,
            round_robin: false,
            lane_order: Vec::new(),
            turn: 0,
        }
    }
}

impl Builders {
    /// Adds idle builders of `id` (taking `seconds` a build), or removes builders keeping
    /// those furthest along.
    pub fn set(&mut self, id: &'static str, seconds: f64, count: usize) {
        let at = self.kinds.iter().position(|k| k.0 == id).unwrap_or_else(|| {
            self.kinds.push((id, seconds, Vec::new()));
            self.kinds.len() - 1
        });
        let list = &mut self.kinds[at].2;
        list.resize(list.len().max(count), IDLE);
        if list.len() > count {
            list.sort_by_key(|b| std::cmp::Reverse(b.ticks));
            list.truncate(count);
        }
    }

    /// The sharing rule: a lane first (`lane_order` that lane), or None to share in turn.
    pub fn prefer(&mut self, lane: Option<usize>) {
        self.round_robin = lane.is_none();
        self.lane_order = lane.into_iter().collect();
    }

    /// Units put in the reserve; the cap doesn't apply.
    pub fn bank(&mut self, id: &'static str) {
        self.reserve.push(id);
    }

    /// One tick: the reserve fills what it can, then every builder works.
    pub fn step(&mut self, lanes: &mut [Wave], seconds: f64, tick: i64, out: &mut Vec<Produced>) {
        let mut index = 0;
        while index < self.reserve.len() {
            let Some(lane) = self.pick(self.reserve[index], lanes, false) else {
                index += 1;
                continue;
            };
            lanes[lane].fill(self.reserve[index]);
            self.reserve.remove(index);
            out.push(Produced::FromReserve { tick, lane });
        }
        for k in 0..self.kinds.len() {
            for b in 0..self.kinds[k].2.len() {
                self.advance((k, b), lanes, (seconds, tick), out);
            }
        }
    }

    fn advance(
        &mut self,
        (k, b): (usize, usize),
        lanes: &mut [Wave],
        (seconds, tick): (f64, i64),
        out: &mut Vec<Produced>,
    ) {
        let (id, build_seconds) = (self.kinds[k].0, self.kinds[k].1);
        if self.kinds[k].2[b].claim == Claim::Idle {
            let claim = self.claim(id, lanes);
            self.kinds[k].2[b].claim = claim;
            if claim == Claim::Idle {
                return;
            }
        }
        let builder = &mut self.kinds[k].2[b];
        builder.ticks += 1;
        let needed = ((build_seconds / seconds).round() as i64).max(1);
        builder.progress = builder.ticks as f64 / needed as f64;
        if builder.ticks < needed {
            return;
        }
        let claimed = builder.claim;
        *builder = IDLE;
        if let Claim::Lane(lane) = claimed {
            if lane < lanes.len() && lanes[lane].fill(id) {
                out.push(Produced::Built { tick, lane });
                return;
            }
        }
        if let Some(other) = self.pick(id, lanes, false) {
            lanes[other].fill(id);
            out.push(Produced::Built { tick, lane: other });
        } else {
            self.reserve.push(id);
            out.push(Produced::ToReserve { tick });
        }
    }

    /// The lane a new build of `id` should serve, or the reserve if it has room, or idle.
    fn claim(&mut self, id: &'static str, lanes: &[Wave]) -> Claim {
        if let Some(lane) = self.pick(id, lanes, true) {
            return Claim::Lane(lane);
        }
        if self.reserve.len() + self.claims(Claim::Reserve, None) < self.reserve_cap {
            return Claim::Reserve;
        }
        Claim::Idle
    }

    /// The first lane, by the sharing rule, with an unfilled place of `id` (not already
    /// claimed by a build, if `respect_claims`).
    fn pick(&mut self, id: &str, lanes: &[Wave], respect_claims: bool) -> Option<usize> {
        let mut in_order: Vec<usize> =
            self.lane_order.iter().copied().filter(|&k| k < lanes.len()).collect();
        in_order.extend((0..lanes.len()).filter(|k| !self.lane_order.contains(k)));
        let mut order = in_order.clone();
        if self.round_robin && !in_order.is_empty() {
            order.rotate_left(self.turn % in_order.len());
        }
        for key in order {
            let mut open = lanes[key].wanting(id);
            if respect_claims {
                open -= self.claims(Claim::Lane(key), Some(id)) as i64;
            }
            if open > 0 {
                self.turn = in_order.iter().position(|&k| k == key).unwrap_or(0) + 1;
                return Some(key);
            }
        }
        None
    }

    /// The builders (of `id`, if given) working for `claim`.
    fn claims(&self, claim: Claim, id: Option<&str>) -> usize {
        let kinds = self.kinds.iter().filter(|k| id.is_none_or(|id| k.0 == id));
        kinds.map(|k| k.2.iter().filter(|b| b.claim == claim).count()).sum()
    }
}
