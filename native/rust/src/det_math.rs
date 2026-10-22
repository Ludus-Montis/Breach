//! `DetMath.sin`, `cos`, `asin`, `acos` and `atan2` (sim/skirmish/formation/det_math.gd),
//! operation for operation in the same order, so they give the same bits as the GDScript
//! and the same bits on every platform (Decision 93). The C library's are not correctly
//! rounded and differ between glibc, the Windows CRT and Apple's libm; these use only
//! `+`, `-`, `*`, `/`, `sqrt` and `floor`, which IEEE 754 fixes exactly. fdlibm's
//! Cody-Waite reduction by pi/2 in three exact parts, then its minimax kernels; fdlibm's
//! atan, with asin and acos through it. Rust never fuses `a * b + c`.

use std::f64::consts::PI;

const S1: f64 = f64::from_bits(0xbfc5555555555549); // -1.66666666666666324348e-01
const S2: f64 = f64::from_bits(0x3f8111111110f8a6); // 8.33333333332248946124e-03
const S3: f64 = f64::from_bits(0xbf2a01a019c161d5); // -1.98412698298579493134e-04
const S4: f64 = f64::from_bits(0x3ec71de357b1fe7d); // 2.75573137070700676789e-06
const S5: f64 = f64::from_bits(0xbe5ae5e68a2b9ceb); // -2.50507602534068634195e-08
const S6: f64 = f64::from_bits(0x3de5d93a5acfd57c); // 1.58969099521155010221e-10
const C1: f64 = f64::from_bits(0x3fa555555555554c); // 4.16666666666666019037e-02
const C2: f64 = f64::from_bits(0xbf56c16c16c15177); // -1.38888888888741095749e-03
const C3: f64 = f64::from_bits(0x3efa01a019cb1590); // 2.48015872894767294178e-05
const C4: f64 = f64::from_bits(0xbe927e4f809c52ad); // -2.75573143513906633035e-07
const C5: f64 = f64::from_bits(0x3e21ee9ebdb4b1c4); // 2.08757232129817482790e-09
const C6: f64 = f64::from_bits(0xbda8fae9be8838d4); // -1.13596475577881948265e-11
const PIO2_1: f64 = f64::from_bits(0x3ff921fb54400000); // 1.57079632673412561417e+00
const PIO2_2: f64 = f64::from_bits(0x3dd0b4611a600000); // 6.07710050630396597660e-11
const PIO2_3: f64 = f64::from_bits(0x3ba3198a2e000000); // 2.02226624871116645580e-21
const INVPIO2: f64 = f64::from_bits(0x3fe45f306dc9c883); // 6.36619772367581382433e-01

// atan's (DetMath._AH*, _AL*, _T*, _PI_LO, _TINY, _HUGE), the bits Godot gives them.
const ATAN_HI: [f64; 4] = [
    f64::from_bits(0x3fddac670561bb4f), // 4.63647609000806093515e-01
    f64::from_bits(0x3fe921fb54442d18), // 7.85398163397448278999e-01
    f64::from_bits(0x3fef730bd281f69b), // 9.82793723247329054082e-01
    PI / 2.0,
];
const ATAN_LO: [f64; 4] = [
    f64::from_bits(0x3c7a2b7f222f65e2), // 2.26987774529616870924e-17
    f64::from_bits(0x3c81a62633145c07), // 3.06161699786838301793e-17
    f64::from_bits(0x3c7007887af0cbbd), // 1.39033110312309984516e-17
    f64::from_bits(0x3c91a62633145c07), // 6.12323399573676603587e-17
];
const T0: f64 = f64::from_bits(0x3fd555555555550d); // 3.33333333333329318027e-01
const T1: f64 = f64::from_bits(0xbfc999999998ebc4); // -1.99999999998764832476e-01
const T2: f64 = f64::from_bits(0x3fc24924920083ff); // 1.42857142725034663711e-01
const T3: f64 = f64::from_bits(0xbfbc71c6fe231671); // -1.11111104054623557880e-01
const T4: f64 = f64::from_bits(0x3fb745cdc54c206e); // 9.09088713343650656196e-02
const T5: f64 = f64::from_bits(0xbfb3b0f2af749a6d); // -7.69187620504482999495e-02
const T6: f64 = f64::from_bits(0x3fb10d66a0d03d51); // 6.66107313738753120669e-02
const T7: f64 = f64::from_bits(0xbfadde2d52defd9a); // -5.83357013379057348645e-02
const T8: f64 = f64::from_bits(0x3fa97b4b24760deb); // 4.97687799461593236017e-02
const T9: f64 = f64::from_bits(0xbfa2b4442c6a6c2f); // -3.65315727442169155270e-02
const T10: f64 = f64::from_bits(0x3f90ad3ae322da11); // 1.62858201153657823623e-02
const PI_LO: f64 = f64::from_bits(0x3ca1a62633145c07); // 1.2246467991473531772e-16
const TINY: f64 = f64::from_bits(0x3e40000000000000); // 2^-27
const HUGE: f64 = f64::from_bits(0x4410000000000000); // 2^66

/// `DetMath.sin`: sin(x), x in radians.
pub fn sin(x: f64) -> f64 {
    let k = (x * INVPIO2 + 0.5).floor();
    let r = reduced(x, k);
    let quarter = k - 4.0 * (k * 0.25).floor();
    if quarter == 0.0 {
        sin_kernel(r)
    } else if quarter == 1.0 {
        cos_kernel(r)
    } else if quarter == 2.0 {
        -sin_kernel(r)
    } else {
        -cos_kernel(r)
    }
}

/// `DetMath.cos`: cos(x), x in radians.
pub fn cos(x: f64) -> f64 {
    let k = (x * INVPIO2 + 0.5).floor();
    let r = reduced(x, k);
    let quarter = k - 4.0 * (k * 0.25).floor();
    if quarter == 0.0 {
        cos_kernel(r)
    } else if quarter == 1.0 {
        -sin_kernel(r)
    } else if quarter == 2.0 {
        -cos_kernel(r)
    } else {
        sin_kernel(r)
    }
}

/// `DetMath.asin`: asin(x) in [-pi/2, pi/2]; beyond [-1, 1] counts as -1 or 1.
pub fn asin(x: f64) -> f64 {
    if x <= -1.0 {
        return -PI / 2.0;
    }
    if x >= 1.0 {
        return PI / 2.0;
    }
    if x == 0.0 {
        return x;
    }
    let t = x / ((1.0 - x) * (1.0 + x)).sqrt();
    if t > 0.0 {
        atan(t)
    } else {
        -atan(-t)
    }
}

/// `DetMath.acos`: acos(x) in [0, pi]; beyond [-1, 1] counts as -1 or 1.
pub fn acos(x: f64) -> f64 {
    if x <= -1.0 {
        return PI;
    }
    if x >= 1.0 {
        return 0.0;
    }
    if x == 0.0 {
        return PI / 2.0;
    }
    let s = ((1.0 - x) * (1.0 + x)).sqrt();
    if x > 0.0 {
        return atan(s / x);
    }
    PI - (atan(s / -x) - PI_LO)
}

/// `DetMath.atan2`: atan2(y, x) in [-pi, pi], with C's quadrants, signed zeros and
/// infinities.
pub fn atan2(y: f64, x: f64) -> f64 {
    #[allow(clippy::eq_op)]
    let finite = x - x == 0.0 && y - y == 0.0;
    if y == 0.0 || x == 0.0 || !finite {
        return atan2_edge(y, x);
    }
    let ratio = if (y > 0.0) == (x > 0.0) { y / x } else { -(y / x) };
    let mut z = atan(ratio);
    if x < 0.0 {
        z = PI - (z - PI_LO);
    }
    if y < 0.0 {
        -z
    } else {
        z
    }
}

/// atan2 where a side is zero, infinite or NaN, as C has it (DetMath._atan2_edge).
fn atan2_edge(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return x + y;
    }
    let below = negative(y);
    if y == 0.0 {
        if !negative(x) {
            return y;
        }
        return if below { -PI } else { PI };
    }
    if x == 0.0 || !x.is_infinite() {
        return if below { -PI / 2.0 } else { PI / 2.0 };
    }
    let mut z = if y.is_infinite() { PI * 0.25 } else { 0.0 };
    if x < 0.0 {
        z = PI - (z - PI_LO);
    }
    if below {
        -z
    } else {
        z
    }
}

/// True if `v` is below zero or is -0.0.
#[inline]
fn negative(v: f64) -> bool {
    v < 0.0 || (v == 0.0 && 1.0 / v < 0.0)
}

/// atan(t) for t >= 0 (fdlibm atan): t brought near 0, 0.5, 1, 1.5 or infinity first.
fn atan(t: f64) -> f64 {
    if t >= HUGE {
        return PI / 2.0;
    }
    let mut t = t;
    let at: usize;
    if t < 0.4375 {
        if t < TINY {
            return t;
        }
        let z = t * t;
        let w = z * z;
        let (s1, s2) = atan_sums(z, w);
        return t - t * (s1 + s2);
    } else if t < 0.6875 {
        at = 0;
        t = (2.0 * t - 1.0) / (2.0 + t);
    } else if t < 1.1875 {
        at = 1;
        t = (t - 1.0) / (t + 1.0);
    } else if t < 2.4375 {
        at = 2;
        t = (t - 1.5) / (1.0 + 1.5 * t);
    } else {
        at = 3;
        t = -1.0 / t;
    }
    let z = t * t;
    let w = z * z;
    let (s1, s2) = atan_sums(z, w);
    ATAN_HI[at] - ((t * (s1 + s2) - ATAN_LO[at]) - t)
}

/// atan's two polynomials, in z = t^2 and w = t^4.
#[inline]
fn atan_sums(z: f64, w: f64) -> (f64, f64) {
    let s1 = z * (T0 + w * (T2 + w * (T4 + w * (T6 + w * (T8 + w * T10)))));
    let s2 = w * (T1 + w * (T3 + w * (T5 + w * (T7 + w * T9))));
    (s1, s2)
}

/// x less k times pi/2, the three parts of pi/2 taken off in turn (each product exact).
#[inline]
fn reduced(x: f64, k: f64) -> f64 {
    ((x - k * PIO2_1) - k * PIO2_2) - k * PIO2_3
}

/// sin on [-pi/4, pi/4] (fdlibm __kernel_sin).
#[inline]
fn sin_kernel(x: f64) -> f64 {
    let z = x * x;
    let v = z * x;
    let r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
    x + v * (S1 + z * r)
}

/// cos on [-pi/4, pi/4] (fdlibm __kernel_cos).
#[inline]
fn cos_kernel(x: f64) -> f64 {
    let z = x * x;
    let r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))));
    let hz = 0.5 * z;
    let w = 1.0 - hz;
    w + (((1.0 - w) - hz) + z * r)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Inputs and the bits DetMath.sin and DetMath.cos give for them, printed by Godot
    // 4.7.2: radians from 1e-9 to 12345.678 and the bearings' deg_to_rad of 0 to 359.
    const CASES: [(f64, u64, u64); 26] = [
        (f64::from_bits(0x0000000000000000), 0x0000000000000000, 0x3ff0000000000000),
        (f64::from_bits(0x3e112e0be826d695), 0x3e112e0be826d695, 0x3ff0000000000000),
        (f64::from_bits(0x3fe0000000000000), 0x3fdeaee8744b05f0, 0x3fec1528065b7d50),
        (f64::from_bits(0xbfe8000000000000), 0xbfe5cffc16bf8f0d, 0x3fe769fec655211f),
        (f64::from_bits(0x3ff0000000000000), 0x3feaed548f090cee, 0x3fe14a280fb5068c),
        (f64::from_bits(0x3fe921fb54442d18), 0x3fe6a09e667f3bcc, 0x3fe6a09e667f3bcd),
        (f64::from_bits(0x3ff921fb54442d18), 0x3ff0000000000000, 0x3c91a62633145c00),
        (f64::from_bits(0x400921fb54442d18), 0x3ca1a62633145c00, 0xbff0000000000000),
        (f64::from_bits(0x4000000000000000), 0x3fed18f6ead1b446, 0xbfdaa22657537205),
        (f64::from_bits(0x4008000000000000), 0x3fc210386db6d55b, 0xbfefae04be85e5d2),
        (f64::from_bits(0xc00c000000000000), 0x3fd6733b7eba621f, 0xbfedf77403c11a5f),
        (f64::from_bits(0x401921fb54442d18), 0xbcb1a62633145c00, 0x3ff0000000000000),
        (f64::from_bits(0x4024000000000000), 0xbfe1689ef5f34f53, 0xbfead9ac890c6b1f),
        (f64::from_bits(0xc059000000000000), 0x3fe03425b78c4db8, 0x3feb981dbf665fdf),
        (f64::from_bits(0x408f420000000000), 0x3fee1702343c0531, 0x3fd5c7d948a31cf2),
        (f64::from_bits(0x40c81cd6c8b43958), 0xbfe687d5890974a5, 0x3fe6b94c3bbe24b8),
        (f64::from_bits(0x0000000000000000), 0x0000000000000000, 0x3ff0000000000000),
        (f64::from_bits(0x3fe0c152382d7365), 0x3fdfffffffffffff, 0x3febb67ae8584cab),
        (f64::from_bits(0x3fe921fb54442d18), 0x3fe6a09e667f3bcc, 0x3fe6a09e667f3bcd),
        (f64::from_bits(0x3ff921fb54442d18), 0x3ff0000000000000, 0x3c91a62633145c00),
        (f64::from_bits(0x4002d97c7f3321d2), 0x3fe6a09e667f3bcd, 0xbfe6a09e667f3bcc),
        (f64::from_bits(0x400921fb54442d18), 0x3ca1a62633145c00, 0xbff0000000000000),
        (f64::from_bits(0x400f111dc8299b4c), 0xbfe59e6f5ae6a0a6, 0xbfe797c6a435ce85),
        (f64::from_bits(0x4012d97c7f3321d2), 0xbff0000000000000, 0xbcaa79394c9e8a00),
        (f64::from_bits(0x4019101c0da1da7b), 0xbf91df0b2b89dd2c, 0x3feffec097f5af8a),
        (f64::from_bits(0x40178fdb9effea47), 0xbfd87de2a6aea95f, 0x3fed906bcf328d47),
    ];

    // Inputs and the bits DetMath.acos and DetMath.asin give for them, printed by Godot
    // 4.7.2: each of atan's five ranges, tiny values, beyond [-1, 1], and signs.
    const INVERSE: [(u64, u64, u64); 14] = [
        (0x3fe0000000000000, 0x3ff0c152382d7365, 0x3fe0c152382d7366),
        (0xbfe0000000000000, 0x4000c152382d7366, 0xbfe0c152382d7366),
        (0x3fd3333333333333, 0x3ff441f5ecbeef58, 0x3fd380159e14f6ff),
        (0x3fefff2e48e8a71e, 0x3f8cf69d216bd74b, 0x3ff8e80e1a01556a),
        (0xbfefff2e48e8a71e, 0x40090504b722c141, 0xbff8e80e1a01556a),
        (0x3e112e0be826d695, 0x3ff921fb53ff74e9, 0x3e112e0be826d695),
        (0xbe112e0be826d695, 0x3ff921fb5488e548, 0xbe112e0be826d695),
        (0x3fdb851eb851eb85, 0x3ff20556df003711, 0x3fdc7291d50fd81e),
        (0x3fe5c28f5c28f5c3, 0x3fea564ac0e73a33, 0x3fe7edabe7a11ffe),
        (0x3ff2e147ae147ae1, 0x0000000000000000, 0x3ff921fb54442d18),
        (0x400370a3d70a3d71, 0x0000000000000000, 0x3ff921fb54442d18),
        (0x444b1ae4d6e2ef50, 0x0000000000000000, 0x3ff921fb54442d18),
        (0x3fe6a09e667f3bcd, 0x3fe921fb54442d18, 0x3fe921fb54442d19),
        (0xbfbf9add3739635f, 0x3ffb1cf4472f9bdb, 0xbfbfaf8f2eb6ec2c),
    ];

    // (y, x) and the bits DetMath.atan2 gives, printed by Godot 4.7.2: every quadrant,
    // atan's ranges, near-axis ratios, zeros and a huge ratio.
    const ATAN2: [(u64, u64, u64); 14] = [
        (0x3ff0000000000000, 0x3ff0000000000000, 0x3fe921fb54442d18),
        (0xbff0000000000000, 0xbff0000000000000, 0xc002d97c7f3321d2),
        (0x3fd3333333333333, 0xc000000000000000, 0x4007f10e1dc6b048),
        (0xc014000000000000, 0x3f847ae147ae147b, 0xbff919ca2e11f4a5),
        (0x3e7ad7f29abcaf48, 0x4008000000000000, 0x3e61e54c672874d9),
        (0x4000000000000000, 0x3e7ad7f29abcaf48, 0x3ff921fb46d833cb),
        (0xbfe3333333333333, 0x3fe999999999999a, 0xbfe4978fa3269ee0),
        (0x3fe3333333333333, 0xbfe999999999999a, 0x4003fc176b7a8560),
        (0x405ec00000000000, 0xbfe0000000000000, 0x3ff932a1cf4be67d),
        (0xbfd0000000000000, 0xbfe8000000000000, 0xc0068f095fdf593c),
        (0x4008000000000000, 0x4004000000000000, 0x3fec08aae496efa6),
        (0x0000000000000000, 0xbff0000000000000, 0x400921fb54442d18),
        (0xbff0000000000000, 0x0000000000000000, 0xbff921fb54442d18),
        (0x46293e5939a08cea, 0x3ff0000000000000, 0x3ff921fb54442d18),
    ];

    #[test]
    fn the_inverse_functions_match_the_gdscript() {
        for &(x, c, s) in INVERSE.iter() {
            let x = f64::from_bits(x);
            assert_eq!(acos(x).to_bits(), c, "acos({x})");
            assert_eq!(asin(x).to_bits(), s, "asin({x})");
        }
        for &(y, x, a) in ATAN2.iter() {
            let (y, x) = (f64::from_bits(y), f64::from_bits(x));
            assert_eq!(atan2(y, x).to_bits(), a, "atan2({y}, {x})");
        }
    }

    #[test]
    fn the_edge_cases_are_c_s() {
        assert_eq!(atan2(-0.0, -1.0), -PI);
        assert_eq!(atan2(0.0, -0.0), PI);
        assert_eq!(atan2(0.0, 0.0).to_bits(), 0);
        assert_eq!(atan2(1.0, f64::NEG_INFINITY), PI);
        assert_eq!(atan2(f64::INFINITY, f64::INFINITY), PI / 4.0);
        assert!(atan2(f64::NAN, 1.0).is_nan());
        assert_eq!(acos(-1.5), PI);
        assert_eq!(asin(1.5), PI / 2.0);
    }

    #[test]
    fn matches_the_gdscript() {
        for &(x, s, c) in CASES.iter() {
            assert_eq!(sin(x).to_bits(), s, "sin({x})");
            assert_eq!(cos(x).to_bits(), c, "cos({x})");
        }
    }
}
