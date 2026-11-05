//! SkirmishSlotPool (Decision 40): the player's formation slots, split across a battle's lanes
//! as they choose, each lane capped by its own limit (its width by MAX_RANKS).

/// The pool: its total, and by lane its cap and the slots assigned to it.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotPool {
    pub total: i32,
    caps: Vec<i32>,
    assigned: Vec<i32>,
}

impl SlotPool {
    pub fn new(total: i32, caps: Vec<i32>) -> SlotPool {
        let assigned = vec![0; caps.len()];
        SlotPool { total, caps, assigned }
    }

    pub fn assigned(&self, lane: usize) -> i32 {
        self.assigned.get(lane).copied().unwrap_or(0)
    }

    pub fn free(&self) -> i32 {
        self.total - self.assigned.iter().sum::<i32>()
    }

    /// A lane's share set to `requested`, clamped to its cap and to what the other lanes
    /// leave free; what it got.
    pub fn assign(&mut self, lane: usize, requested: i32) -> i32 {
        let Some(&cap) = self.caps.get(lane) else { return 0 };
        let most = cap.min(self.assigned(lane) + self.free());
        self.assigned[lane] = requested.clamp(0, most.max(0));
        self.assigned[lane]
    }
}
