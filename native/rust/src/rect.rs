//! Godot's `Rect2` with `real_t` = f32, its operations as Godot's C++ has them, so a box
//! worked out here is bit for bit GDScript's (spec 39 step 4b: the scrum's squad boxes).

use crate::maths::V2;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Rect {
    pub position: V2,
    pub size: V2,
}

/// `Vector2::clamp`: each axis `CLAMP(v, low, high)`, `v < low ? low : (v > high ? high : v)`.
pub fn clamp(v: V2, low: V2, high: V2) -> V2 {
    let one = |v: f32, low: f32, high: f32| {
        if v < low {
            low
        } else if v > high {
            high
        } else {
            v
        }
    };
    V2::new(one(v.x, low.x, high.x), one(v.y, low.y, high.y))
}

impl Rect {
    /// `Rect2(position, size)`.
    pub fn new(position: V2, size: V2) -> Rect {
        Rect { position, size }
    }

    /// `Rect2.end`: `position + size`.
    pub fn end(self) -> V2 {
        self.position.add(self.size)
    }

    /// `Rect2.expand(to)`: grown to take in the point.
    pub fn expand(self, to: V2) -> Rect {
        let mut begin = self.position;
        let mut end = self.end();
        if to.x < begin.x {
            begin.x = to.x;
        }
        if to.y < begin.y {
            begin.y = to.y;
        }
        if to.x > end.x {
            end.x = to.x;
        }
        if to.y > end.y {
            end.y = to.y;
        }
        Rect { position: begin, size: end.sub(begin) }
    }

    /// `Rect2.get_center()`: `position + size * 0.5`.
    pub fn center(self) -> V2 {
        self.position.add(self.size.scale(0.5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_grows_to_take_in_points() {
        let one = Rect::new(V2::new(1.0, 1.0), V2::ZERO).expand(V2::new(3.0, -1.0));
        assert_eq!(one, Rect::new(V2::new(1.0, -1.0), V2::new(2.0, 2.0)));
        assert_eq!(one.center(), V2::new(2.0, 0.0));
        assert_eq!(clamp(V2::new(5.0, -3.0), one.position, one.end()), V2::new(3.0, -1.0));
    }
}
