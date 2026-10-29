class_name CrateScrum
extends RefCounted
## The crate's contest and reach in the scrum (`ScrumRules`, native/rust/src/contest.rs and
## reach.rs; spec 39 step 4a), which ScrumContest and ScrumReach are views over, and
## FormationMorale over its ceiling, bands and shocks (bands.rs, step 8a2-iv). Stateless, so
## one is shared.

const RULES_CLASS := "ScrumRules"

static var _core: Object


## The shared one.
static func core() -> Object:
	if _core == null:
		NativeLibrary.load_for(RULES_CLASS)
		_core = ClassDB.instantiate(RULES_CLASS)
	return _core
