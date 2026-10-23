//! What a pathfinder can see to plan by (PathSight, spec 30): a shape, not a circle - its
//! reach ahead, to either side and behind, eased between them by the angle off its facing
//! (straight ahead the ahead reach, square to it the side reach, straight back the behind
//! reach, linearly between). A unit type sets the three as shares of its detection range.
//! Within that shape, on ground that hides (`hides`), it sees a point while the obscurance
//! along its sight line stays within its budget (`Ground::sight_clear`). Symmetric left and
//! right, so ground mirrored is seen mirrored.

use crate::det_math;
use crate::ground::Ground;
use crate::maths::V2;

#[derive(Clone, Copy, Debug)]
pub struct Sight {
    /// Where it looks from (cells) and which way (a unit vector).
    pub origin: V2,
    pub facing: V2,
    /// How far (cells) it sees straight ahead, square to either side and straight back.
    pub ahead: f64,
    pub side: f64,
    pub behind: f64,
    /// Whether the ground it looks over hides anything, and how much obscurance a sight
    /// line may sum before it sees no further.
    pub hides: bool,
    pub budget: f64,
}

impl Sight {
    /// How far it sees in the direction `way` (a unit vector).
    pub fn reach(&self, way: V2) -> f64 {
        let cosine = (way.dot(self.facing) as f64).clamp(-1.0, 1.0);
        let turned = det_math::acos(cosine) / (std::f64::consts::PI / 2.0); // 0 ahead to 2 behind
        if turned <= 1.0 {
            lerp(self.ahead, self.side, turned)
        } else {
            lerp(self.side, self.behind, turned - 1.0)
        }
    }

    /// Whether it sees `point` over `ground` (None: nothing hides anything).
    pub fn sees(&self, ground: Option<&Ground>, point: V2) -> bool {
        let offset = point.sub(self.origin);
        let length = offset.length();
        let way = V2::new(offset.x / length, offset.y / length);
        if length as f64 > 0.000001 && length as f64 > self.reach(way) {
            return false;
        }
        match ground {
            Some(ground) if self.hides => ground.sight_clear(self.origin, point, self.budget),
            _ => true,
        }
    }
}

/// Godot's `lerpf`.
fn lerp(from: f64, to: f64, weight: f64) -> f64 {
    from + (to - from) * weight
}
