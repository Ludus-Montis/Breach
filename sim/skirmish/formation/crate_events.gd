class_name CrateEvents
extends RefCounted
## The events a crate pass built as data (native/rust/src/events.rs, spec 39 step 8b5-v),
## decoded into the dictionaries FormationEvents made: "type", "tick", "squad", "faction",
## then the fields its kind names, in order; a kind told of a unit ("hit") has "unit" in
## place of "squad", and its unit's faction. The kinds are the crate's, asked for once; a
## field naming an order, a band or how a blow landed is decoded to its name, a faction, a
## staged squad's "then" or a damage type from the lexicon, a flank to a bool, a heading
## from its float's bits.

const SkirmishUnit = preload("res://sim/skirmish/skirmish_unit.gd")
const FormationMorale = preload("res://sim/skirmish/formation/formation_morale.gd")
const BlowLanding = preload("res://sim/skirmish/formation/blow_landing.gd")

## The fields decoded from the lexicon.
const WORDS := ["then", "damage_type"]

static var _kinds := []


## The events the crate of `field` built since last taken, in the order built.
static func taken(field: Object) -> Array:
	var flat: PackedInt64Array = field.events_take()
	if flat.is_empty():
		return []
	if _kinds.is_empty():
		_kinds = field.event_kinds()
	var words: PackedStringArray = field.lexicon_words()
	var out := []
	var at := 0
	while at < flat.size():
		var kind: Array = _kinds[flat[at]]
		var event := {"type": kind[0], "tick": flat[at + 1]}
		event["unit" if kind[2] else "squad"] = flat[at + 2]
		event["faction"] = words[flat[at + 3]]
		at += 4
		for name: String in kind[1]:
			event[name] = words[flat[at]] if name in WORDS else _decoded(name, flat[at])
			at += 1
		out.append(event)
	return out


static func _decoded(name: String, value: int) -> Variant:
	match name:
		"order":
			return SkirmishUnit.Order.keys()[value]
		"band":
			return FormationMorale.Band.keys()[value]
		"blow":
			return BlowLanding.LANDED[value]
		"flank":
			return value != 0
		"heading":
			var bits := PackedByteArray()
			bits.resize(8)
			bits.encode_s64(0, value)
			return bits.decode_double(0)
	return value
