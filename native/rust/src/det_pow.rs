//! `DetPow.pow` (sim/skirmish/formation/det_pow.gd), operation for operation in the same
//! order, so it gives the same bits as the GDScript on every platform (Decision 93): only
//! `+`, `-`, `*`, `/`, `sqrt`, `floor` and comparisons. exp and log follow fdlibm. pow takes
//! a whole power up to 64 by squaring, a half power by sqrt, and any other as exp(y log x).

const LN2_HI: f64 = 0xB17217F7u64 as f64 / (1u64 << 32) as f64;
const LN2_LO: f64 = 0xD1CF79ABC9E3Bu64 as f64 / (1u64 << 62) as f64 / (1u64 << 22) as f64;
const LG1: f64 = 0x15555555555593u64 as f64 / (1u64 << 53) as f64;
const LG2: f64 = 0x666666665FE81u64 as f64 / (1u64 << 52) as f64;
const LG3: f64 = 0x12492494229359u64 as f64 / (1u64 << 54) as f64;
const LG4: f64 = 0x1C71C51D8E78AFu64 as f64 / (1u64 << 55) as f64;
const LG5: f64 = 0xBA3324B6581EFu64 as f64 / (1u64 << 54) as f64;
const LG6: f64 = 0x139A09D078C69Fu64 as f64 / (1u64 << 55) as f64;
const LG7: f64 = 0x4BC44B7CF9491u64 as f64 / (1u64 << 53) as f64;
const INVLN2: f64 = 0xB8AA3B295C17Fu64 as f64 / (1u64 << 51) as f64;
const P1: f64 = 0xAAAAAAAAAAA9Fu64 as f64 / (1u64 << 54) as f64;
const P2: f64 = -(0x16C16C16BEBD93u64 as f64) / (1u64 << 61) as f64;
const P3: f64 = 0x4559AABC9778Bu64 as f64 / (1u64 << 62) as f64 / (1u64 << 2) as f64;
const P4: f64 = -(0x1BBD41C5D26BF1u64 as f64) / (1u64 << 62) as f64 / (1u64 << 10) as f64;
const P5: f64 = 0x16376972BEA4Du64 as f64 / (1u64 << 62) as f64 / (1u64 << 11) as f64;
const OVER: f64 = 0x162E42FEFA39EFu64 as f64 / (1u64 << 43) as f64;
const UNDER: f64 = -(0x174910D52D3051u64 as f64) / (1u64 << 43) as f64;
const SQRT2: f64 = 0x16A09E667F3BCDu64 as f64 / (1u64 << 52) as f64;
const TWO64: f64 = (1u64 << 62) as f64 * 4.0;

/// x to the power y, with C's special cases (0, 1, infinities, negative bases).
pub fn pow(x: f64, y: f64) -> f64 {
    if y == 0.0 || x == 1.0 {
        return 1.0;
    }
    if x.is_nan() || y.is_nan() {
        return x + y;
    }
    if y.is_infinite() {
        if x.abs() == 1.0 {
            return 1.0;
        }
        return if (x.abs() > 1.0) == (y > 0.0) { f64::INFINITY } else { 0.0 };
    }
    if x == 0.0 || x == f64::INFINITY {
        return if (x == 0.0) == (y < 0.0) { f64::INFINITY } else { 0.0 };
    }
    if y == 0.5 && x > 0.0 {
        return x.sqrt();
    }
    if y.floor() == y && y.abs() <= 64.0 {
        return whole_power(x, y);
    }
    if x < 0.0 {
        if y.floor() != y {
            return f64::NAN;
        }
        let whole = exp(y * log(-x));
        let odd = y.abs() < TWO64 && y - 2.0 * (y * 0.5).floor() == 1.0;
        return if odd { -whole } else { whole };
    }
    exp(y * log(x))
}

/// e to the x (fdlibm exp): x less k ln 2, a rational fit of the rest, scaled by 2^k.
fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x > OVER {
        return f64::INFINITY;
    }
    if x < UNDER {
        return 0.0;
    }
    let k = (x * INVLN2 + 0.5).floor();
    let hi = x - k * LN2_HI;
    let lo = k * LN2_LO;
    let r = hi - lo;
    let t = r * r;
    let c = r - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    let y = 1.0 - ((lo - (r * c) / (2.0 - c)) - hi);
    scaled(y, k)
}

/// The natural log (fdlibm log).
fn log(mut x: f64) -> f64 {
    if x.is_nan() || x == f64::INFINITY {
        return x;
    }
    if x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    let mut k = 0.0;
    while x >= TWO64 {
        x /= TWO64;
        k += 64.0;
    }
    while x < 1.0 / TWO64 {
        x *= TWO64;
        k -= 64.0;
    }
    while x >= 2.0 {
        x *= 0.5;
        k += 1.0;
    }
    while x < 1.0 {
        x *= 2.0;
        k -= 1.0;
    }
    if x > SQRT2 {
        x *= 0.5;
        k += 1.0;
    }
    let f = x - 1.0;
    let s = f / (2.0 + f);
    let z = s * s;
    let w = z * z;
    let r = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7))) + w * (LG2 + w * (LG4 + w * LG6));
    let hfsq = 0.5 * f * f;
    k * LN2_HI - ((hfsq - (s * (hfsq + r) + k * LN2_LO)) - f)
}

/// x to a whole power y, |y| <= 64, by repeated squaring.
fn whole_power(mut x: f64, y: f64) -> f64 {
    let mut n = y.abs();
    let mut result = 1.0;
    while n > 0.0 {
        let half = (n * 0.5).floor();
        if n - 2.0 * half == 1.0 {
            result *= x;
        }
        n = half;
        if n > 0.0 {
            x *= x;
        }
    }
    if y < 0.0 {
        1.0 / result
    } else {
        result
    }
}

/// y times 2^k, k whole.
fn scaled(mut y: f64, mut k: f64) -> f64 {
    while k >= 64.0 {
        y *= TWO64;
        k -= 64.0;
    }
    while k <= -64.0 {
        y /= TWO64;
        k += 64.0;
    }
    while k >= 1.0 {
        y *= 2.0;
        k -= 1.0;
    }
    while k <= -1.0 {
        y *= 0.5;
        k += 1.0;
    }
    y
}

#[cfg(test)]
mod tests {
    use super::pow;

    #[test]
    fn whole_and_half_powers_are_exact_and_the_rest_near_the_true_value() {
        assert_eq!(pow(3.0, 4.0), 81.0);
        assert_eq!(pow(2.0, -2.0), 0.25);
        assert_eq!(pow(9.0, 0.5), 3.0);
        assert_eq!(pow(-2.0, 3.0), -8.0);
        assert!((pow(2.0, 0.75) - 1.681_792_830_507_429).abs() < 1e-15);
        assert!(pow(-2.0, 0.5).is_nan());
        assert_eq!(pow(0.0, -1.0), f64::INFINITY);
    }
}
