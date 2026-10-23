//! Godot's maths as GDScript sees it, spelled out so results are bit for bit Godot's own:
//! `Vector2` in f32 (`real_t`), GDScript `float` in f64, a `Vector2 * float` narrowing the
//! float to f32 first, `snappedf` and `UnitMotion.vector` as the engine computes them, and
//! no fused multiply-add (Rust never contracts `a * b + c` on its own, and the library
//! targets baseline x86-64, which has no FMA instructions to choose).

use crate::det_math;

/// Godot's `Vector2` with `real_t` = f32, its operators as Godot's C++ has them.
#[derive(Clone, Copy, PartialEq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const ZERO: V2 = V2 { x: 0.0, y: 0.0 };

    #[inline]
    pub fn new(x: f32, y: f32) -> V2 {
        V2 { x, y }
    }

    #[inline]
    pub fn add(self, o: V2) -> V2 {
        V2 { x: self.x + o.x, y: self.y + o.y }
    }

    #[inline]
    pub fn sub(self, o: V2) -> V2 {
        V2 { x: self.x - o.x, y: self.y - o.y }
    }

    /// `Vector2 * float`: the GDScript float (f64) is narrowed to real_t first.
    #[inline]
    pub fn scale(self, s: f64) -> V2 {
        let s = s as f32;
        V2 { x: self.x * s, y: self.y * s }
    }

    /// `Vector2::length`: `Math::sqrt(x * x + y * y)` in real_t.
    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// `Vector2::distance_to`: `Math::sqrt((x - o.x) * (x - o.x) + (y - o.y) * (y - o.y))`.
    #[inline]
    pub fn distance_to(self, o: V2) -> f32 {
        let dx = self.x - o.x;
        let dy = self.y - o.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// `Vector2::dot`: `x * o.x + y * o.y`.
    #[inline]
    pub fn dot(self, o: V2) -> f32 {
        self.x * o.x + self.y * o.y
    }

    /// `Vector2::normalized`.
    #[inline]
    pub fn normalized(self) -> V2 {
        let l = self.x * self.x + self.y * self.y;
        if l != 0.0 {
            let l = l.sqrt();
            V2 { x: self.x / l, y: self.y / l }
        } else {
            self
        }
    }

    /// `Vector2::orthogonal`: `(y, -x)`.
    #[inline]
    pub fn orthogonal(self) -> V2 {
        V2 { x: self.y, y: -self.x }
    }

    /// `Vector2::is_equal_approx`: each axis within CMP_EPSILON (scaled by its own value)
    /// in real_t.
    #[inline]
    pub fn is_equal_approx(self, o: V2) -> bool {
        fn near(a: f32, b: f32) -> bool {
            if a == b {
                return true;
            }
            let tolerance = (CMP_EPSILON as f32 * a.abs()).max(CMP_EPSILON as f32);
            (a - b).abs() < tolerance
        }
        near(self.x, o.x) && near(self.y, o.y)
    }

    /// `Vector2::lerp`: the weight narrowed to real_t, `from + (to - from) * weight` per axis.
    #[inline]
    pub fn lerp(self, to: V2, weight: f64) -> V2 {
        let w = weight as f32;
        V2 { x: self.x + (to.x - self.x) * w, y: self.y + (to.y - self.y) * w }
    }

    /// `Vector2::move_toward`: `to` if within `delta` (or nearer than CMP_EPSILON), else
    /// `delta` along the way, the GDScript float narrowed to real_t.
    #[inline]
    pub fn move_toward(self, to: V2, delta: f64) -> V2 {
        const CMP_EPSILON: f32 = 0.00001;
        let delta = delta as f32;
        let way = to.sub(self);
        let length = way.length();
        if length <= delta || length < CMP_EPSILON {
            to
        } else {
            self.add(V2::new(way.x / length * delta, way.y / length * delta))
        }
    }
}

/// Godot's `CMP_EPSILON`, the tolerance of `is_equal_approx` and `is_zero_approx`.
const CMP_EPSILON: f64 = 0.00001;

/// Godot's `clampf`.
#[inline]
pub fn clampf(v: f64, low: f64, high: f64) -> f64 {
    if v < low {
        low
    } else if v > high {
        high
    } else {
        v
    }
}

/// Godot's `maxf`.
#[inline]
pub fn maxf(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

/// Godot's `minf`.
#[inline]
pub fn minf(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

/// Godot's `signf`: 1, -1 or 0 (and 0 for NaN).
#[inline]
pub fn signf(v: f64) -> f64 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Godot's `lerpf`: `from + (to - from) * weight`.
#[inline]
pub fn lerpf(from: f64, to: f64, weight: f64) -> f64 {
    from + (to - from) * weight
}

/// Godot's `is_equal_approx` for floats: equal, or within CMP_EPSILON scaled by `a` (at
/// least CMP_EPSILON).
#[inline]
pub fn is_equal_approx(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    let tolerance = (CMP_EPSILON * a.abs()).max(CMP_EPSILON);
    (a - b).abs() < tolerance
}

/// Godot's `is_zero_approx` for floats.
#[inline]
pub fn is_zero_approx(v: f64) -> bool {
    v.abs() < CMP_EPSILON
}

/// GDScript's `snappedf(value, step)`: `Math::floor(value / step + 0.5) * step`.
#[inline]
pub fn snapped(value: f64, step: f64) -> f64 {
    (value / step + 0.5).floor() * step
}

/// `UnitMotion.vector(bearing)`: the unit vector of a bearing (degrees clockwise from
/// north), exact on the quarter bearings. `deg_to_rad` is `bearing * (PI / 180)`; `sin` and
/// `cos` are DetMath's (`det_math.rs`), the same bits on every platform.
#[inline]
pub fn bearing_vector(bearing: f64) -> V2 {
    const EPSILON: f64 = 0.000001;
    let angle = bearing * (std::f64::consts::PI / 180.0);
    let x = det_math::sin(angle);
    let y = -det_math::cos(angle);
    V2 {
        x: if x.abs() > EPSILON { x as f32 } else { 0.0 },
        y: if y.abs() > EPSILON { y as f32 } else { 0.0 },
    }
}
