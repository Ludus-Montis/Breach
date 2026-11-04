class_name CrateMirroring
extends RefCounted
## While GDScript's copies are mirrored after the whole tick's one crate call (spec 39 step
## 8b5-xlii, FormationTick): the crate holds the tick's final values, and each pass's mirror
## runs after every later pass, so a write a mirror makes would land over what a later pass
## did (a body the deaths downed and carrying then bore, put back DOWNED). While `on`, the
## writes that reach the crate from GDScript's copies don't: a unit's state and numbers take
## the crate's value instead (UnitState.put, UnitWrites.number), and its words, objects, its
## squad's columns and joined list are left as the crate has them.

static var on := false


## Runs `call` with the writes quiet; returns what it returns.
static func run(call: Callable) -> Variant:
	on = true
	var out: Variant = call.call()
	on = false
	return out
