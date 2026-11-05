extends SceneTree
## Which maths primitives give different bits on this platform (Decision 93: a battle must
## replay bit for bit on every machine). Each line hashes one primitive's results over the
## same few thousand inputs; run it on two machines and diff the output - a differing line
## names a primitive GDScript can't rely on across platforms. Covers Godot's own maths:
## the transcendental ones (sin, cos, acos, asin, atan2, pow, ease, Vector2.rotated), which
## come from the platform's maths library, and, as controls, ones IEEE 754 fixes exactly
## (sqrt, distance_to, normalized, fposmod, snappedf), plus hash() and float formatting.
## The simulation's own maths are the crate's (`det_math.rs`, `det_pow.rs`, spec 39), built
## from IEEE 754's exact operations; since step 9d-iv no GDScript plays the tick.
##
##   godot --headless --path . --script res://tools/platform_probe.gd

const COUNT := 4096

var _state := 12345


func _init() -> void:
	print(
		(
			"platform_probe: %s, %s, %s"
			% [OS.get_name(), OS.get_processor_name(), Engine.get_version_info()["string"]]
		)
	)
	var xs := _inputs(-50.0, 50.0)
	var units := _inputs(-1.0, 1.0)
	var positive := _inputs(0.0, 4.0)
	_line("sin", xs.map(func(x): return sin(x)))
	_line("cos", xs.map(func(x): return cos(x)))
	_line("acos", units.map(func(x): return acos(x)))
	_line("asin", units.map(func(x): return asin(x)))
	var pairs := range(COUNT).map(func(i): return atan2(xs[i], xs[COUNT - 1 - i]))
	_line("atan2", pairs)
	_line("pow", range(COUNT).map(func(i): return pow(positive[i], units[i] * 3.0)))
	_line("ease", range(COUNT).map(func(i): return ease(absf(units[i]), xs[i] / 10.0)))
	var points := range(COUNT).map(func(i): return Vector2(xs[i], xs[COUNT - 1 - i]))
	_line(
		"Vector2.rotated", _flat(range(COUNT).map(func(i): return points[i].rotated(units[i] * PI)))
	)
	_line("deg_to_rad", xs.map(func(x): return deg_to_rad(x * 7.0)))
	_line("sqrt (control)", positive.map(func(x): return sqrt(x)))
	_line(
		"distance_to (control)",
		range(COUNT).map(func(i): return points[i].distance_to(points[COUNT - 1 - i]))
	)
	_line("normalized (control)", _flat(points.map(func(p): return p.normalized())))
	_line("fposmod (control)", xs.map(func(x): return fposmod(x, 360.0)))
	_line("snappedf (control)", xs.map(func(x): return snappedf(x, 0.0001)))
	_text("hash", range(COUNT).map(func(i): return str(hash([i, "unit", 0, xs[i]]))))
	_text("format", xs.map(func(x): return "%.6f|%s" % [x, str(x)]))
	quit(0)


## COUNT floats spread over [low, high], from a fixed LCG (exact integer arithmetic).
func _inputs(low: float, high: float) -> Array:
	var out := []
	for _i in range(COUNT):
		_state = (_state * 1103515245 + 12345) % 2147483648
		out.append(low + (high - low) * (_state / 2147483648.0))
	return out


func _flat(vectors: Array) -> Array:
	var out := []
	for v in vectors:
		out.append(v.x)
		out.append(v.y)
	return out


func _line(label: String, values: Array) -> void:
	_print(label, PackedFloat64Array(values).to_byte_array())


func _text(label: String, values: Array) -> void:
	_print(label, "\n".join(PackedStringArray(values)).to_utf8_buffer())


func _print(label: String, bytes: PackedByteArray) -> void:
	var digest := HashingContext.new()
	digest.start(HashingContext.HASH_SHA256)
	digest.update(bytes)
	print("  %-24s %s" % [label, digest.finish().hex_encode().substr(0, 16)])
