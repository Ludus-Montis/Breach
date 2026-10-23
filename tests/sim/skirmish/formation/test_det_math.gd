extends GdUnitTestSuite
## DetMath and DetPow (Decision 93: a battle replays bit for bit on every machine): their
## results on a fixed set of inputs are pinned by hash, so this suite fails on any platform
## where one of them gives other bits; they stay within 1e-12 of the engine's own
## functions over a dense sweep; and their edge cases are C's.

const DetMath = preload("res://sim/skirmish/formation/det_math.gd")
const DetPow = preload("res://sim/skirmish/formation/det_pow.gd")

const COUNT := 2048
const BOUND := 1e-12

## The first 16 hex digits of each function's sha256 over _pinned's inputs, from Linux
## x86_64 (Godot 4.7.2 official). A mismatch elsewhere means the platform differs.
const PINNED := {
	"sin": "1ec73b2883f079aa",
	"cos": "3518183c5448fbd9",
	"asin": "b7baa1d83417acc9",
	"acos": "00eecd0882020ec3",
	"atan2": "b48641339de7e6b0",
	"pow": "988f784787e017d8",
	"ease": "976274fc75f40d1c",
	"exp": "af1cbb1bed652543",
	"log": "7044ba6a6224cc26",
	"rotated": "1d7f4d6b18da3ae4",
}

var _state := 2024


func test_the_bits_are_pinned() -> void:
	var got := _pinned()
	for name in PINNED:
		assert_str(got[name]).override_failure_message("%s: %s" % [name, got[name]]).is_equal(
			PINNED[name]
		)


func test_sin_and_cos_are_within_bound_of_the_engine() -> void:
	var worst := [0.0, 0.0]
	for i in range(-60000, 60001):
		var x := i * 0.001 + (0.0 if i % 5 else i * 0.13)  # to +-60 and on to +-7,800
		worst[0] = maxf(worst[0], _off(DetMath.sin(x), sin(x)))
		worst[1] = maxf(worst[1], _off(DetMath.cos(x), cos(x)))
	for k in range(-64, 65):  # the multiples of pi/2 and their neighbours
		for x in [k * PI / 2.0, k * PI / 2.0 + 1e-9, k * PI / 4.0, deg_to_rad(k * 22.5)]:
			worst[0] = maxf(worst[0], _off(DetMath.sin(x), sin(x)))
			worst[1] = maxf(worst[1], _off(DetMath.cos(x), cos(x)))
	print("test_det_math (x 1e-15): sin %.3f, cos %.3f" % _scaled(worst))
	assert_float(worst.max()).is_less_equal(BOUND)


func test_the_inverse_functions_are_within_bound_of_the_engine() -> void:
	var worst := [0.0, 0.0, 0.0]
	for i in range(-100000, 100001):
		var u := i / 100000.0  # -1 to 1, both included
		worst[0] = maxf(worst[0], _off(DetMath.asin(u), asin(u)))
		worst[1] = maxf(worst[1], _off(DetMath.acos(u), acos(u)))
	for i in range(0, 3600):  # round the circle, at radii from 1e-6 to 1e6
		var angle := i * TAU / 3600.0
		var radius := pow(10.0, (i % 13) - 6.0)
		var y := radius * sin(angle)
		var x := radius * cos(angle)
		worst[2] = maxf(worst[2], _off(DetMath.atan2(y, x), atan2(y, x)))
		worst[2] = maxf(worst[2], _off(DetMath.atan2(y, x * 1e-9), atan2(y, x * 1e-9)))
	print("test_det_math (x 1e-15): asin %.3f, acos %.3f, atan2 %.3f" % _scaled(worst))
	assert_float(worst.max()).is_less_equal(BOUND)


func test_pow_exp_log_and_ease_are_within_bound_of_the_engine() -> void:
	var worst := [0.0, 0.0, 0.0, 0.0]
	for i in range(1, 40001):
		var x := i * 0.025  # to 1,000
		var y := (i % 401) * 0.05 - 10.0  # -10 to 10
		worst[0] = maxf(worst[0], _relative(DetPow.pow(x, y), pow(x, y)))
		worst[1] = maxf(worst[1], _relative(DetPow.exp(y * 7.0), exp(y * 7.0)))
		worst[2] = maxf(worst[2], _off(DetPow.log(x * 1e-3), log(x * 1e-3)))
		var t := (i % 1001) / 1000.0
		worst[3] = maxf(worst[3], _off(DetPow.ease(t, y * 0.5), ease(t, y * 0.5)))
	print("test_det_math (x 1e-15): pow %.3f, exp %.3f, log %.3f, ease %.3f" % _scaled(worst))
	assert_float(worst.max()).is_less_equal(BOUND)


func test_rotated_is_within_a_float32_step_of_the_engine() -> void:
	for i in range(0, 720):
		var v := Vector2(3.0 - i * 0.01, 0.5 + i * 0.002)
		var angle := i * 0.0175 - 6.3
		var ours := DetMath.rotated(v, angle)
		assert_float(ours.distance_to(v.rotated(angle))).is_less_equal(v.length() * 1e-6)
		assert_float(ours.length()).is_equal_approx(v.length(), v.length() * 1e-6)


func test_the_edge_cases_are_c_s() -> void:
	# Made at run time: GDScript pools a function's constants, and -0.0 == 0.0 would merge.
	var minus_zero := -(float(_state) * 0.0)
	assert_float(DetMath.asin(1.0)).is_equal(PI / 2.0)
	assert_float(DetMath.asin(-1.0)).is_equal(-PI / 2.0)
	assert_float(DetMath.asin(1.5)).is_equal(PI / 2.0)  # beyond 1 counts as 1, as Godot's
	assert_float(DetMath.acos(1.0)).is_equal(0.0)
	assert_float(DetMath.acos(-1.0)).is_equal(PI)
	assert_float(DetMath.acos(-1.5)).is_equal(PI)
	assert_float(DetMath.acos(0.0)).is_equal(PI / 2.0)
	assert_float(DetMath.atan2(0.0, 1.0)).is_equal(0.0)
	assert_float(DetMath.atan2(0.0, -1.0)).is_equal(PI)
	assert_float(DetMath.atan2(minus_zero, -1.0)).is_equal(-PI)
	assert_float(DetMath.atan2(0.0, 0.0)).is_equal(0.0)
	assert_float(DetMath.atan2(0.0, minus_zero)).is_equal(PI)
	assert_float(DetMath.atan2(2.0, 0.0)).is_equal(PI / 2.0)
	assert_float(DetMath.atan2(-2.0, 0.0)).is_equal(-PI / 2.0)
	assert_float(DetMath.atan2(1.0, 1.0)).is_equal(PI / 4.0)
	assert_float(DetMath.atan2(-1.0, -1.0)).is_equal(atan2(-1.0, -1.0))
	assert_float(DetMath.atan2(1.0, -INF)).is_equal(PI)
	assert_float(DetMath.atan2(INF, INF)).is_equal(PI / 4.0)
	assert_float(DetPow.pow(0.0, 2.0)).is_equal(0.0)
	assert_float(DetPow.pow(0.0, -1.0)).is_equal(INF)
	assert_float(DetPow.pow(0.0, 0.0)).is_equal(1.0)
	assert_float(DetPow.pow(7.5, 0.0)).is_equal(1.0)
	assert_float(DetPow.pow(1.0, 1e300)).is_equal(1.0)
	assert_float(DetPow.pow(-2.0, 3.0)).is_equal(-8.0)
	assert_float(DetPow.pow(-2.0, 2.0)).is_equal(4.0)
	assert_bool(is_nan(DetPow.pow(-2.0, 0.5))).is_true()
	assert_float(DetPow.pow(2.0, 10.0)).is_equal(1024.0)
	assert_float(DetPow.pow(10.0, 400.0)).is_equal(INF)
	assert_float(DetPow.pow(10.0, -400.0)).is_equal(0.0)
	assert_float(DetPow.ease(0.3, 0.0)).is_equal(0.0)
	assert_float(DetPow.ease(1.0, 2.0)).is_equal(1.0)
	assert_float(DetMath.sin(0.0)).is_equal(0.0)
	assert_float(DetMath.cos(0.0)).is_equal(1.0)
	assert_float(DetMath.sin(PI / 2.0)).is_equal(1.0)


## Each function's sha256 (16 hex digits) over fixed inputs, from a fixed LCG.
func _pinned() -> Dictionary:
	_state = 2024
	var xs := _inputs(-60.0, 60.0)
	var units := _inputs(-1.0, 1.0)
	var positive := _inputs(0.0, 50.0)
	var pairs := range(COUNT).map(func(i): return DetMath.atan2(xs[i], xs[COUNT - 1 - i]))
	var turned := []
	for i in range(COUNT):
		var v := DetMath.rotated(Vector2(xs[i], xs[COUNT - 1 - i]), xs[i] * 0.37)
		turned.append_array([v.x, v.y])
	return {
		"sin": _digest(xs.map(func(x): return DetMath.sin(x))),
		"cos": _digest(xs.map(func(x): return DetMath.cos(x))),
		"asin": _digest(units.map(func(x): return DetMath.asin(x))),
		"acos": _digest(units.map(func(x): return DetMath.acos(x))),
		"atan2": _digest(pairs),
		"pow": _digest(range(COUNT).map(func(i): return DetPow.pow(positive[i], _power(xs, i)))),
		"ease": _digest(units.map(func(u): return DetPow.ease(absf(u), u * 4.0 - 1.0))),
		"exp": _digest(xs.map(func(x): return DetPow.exp(x * 5.0))),
		"log": _digest(positive.map(func(x): return DetPow.log(x))),
		"rotated": _digest(turned),
	}


## A power for pow's pins: whole (by squaring) every third input, else any.
func _power(xs: Array, i: int) -> float:
	return float(i % 129 - 64) if i % 3 == 0 else xs[i] / 6.0


func _inputs(low: float, high: float) -> Array:
	var out := []
	for _i in range(COUNT):
		_state = (_state * 1103515245 + 12345) % 2147483648
		out.append(low + (high - low) * (_state / 2147483648.0))
	return out


func _digest(values: Array) -> String:
	var hashing := HashingContext.new()
	hashing.start(HashingContext.HASH_SHA256)
	hashing.update(PackedFloat64Array(values).to_byte_array())
	return hashing.finish().hex_encode().substr(0, 16)


## The error measure: absolute up to 1, relative beyond.
func _off(ours: float, engine: float) -> float:
	return absf(ours - engine) / maxf(1.0, absf(engine))


func _relative(ours: float, engine: float) -> float:
	if ours == engine:
		return 0.0
	return absf(ours - engine) / absf(engine)


func _scaled(worst: Array) -> Array:
	return worst.map(func(w): return w * 1e15)
