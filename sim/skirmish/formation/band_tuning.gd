class_name BandTuning
extends RefCounted
## Morale's tuning as the crate reads it (`bands::BandTuning`): what FormationMorale, and the
## passes that shock a squad in the crate (FormationOrders, FormationEdges), hand it.

const BattleTuning = preload("res://content/definitions/battle_tuning.gd")


## [morale_per_leadership, morale_steady_at, morale_shaken_at, morale_loss,
## morale_leader_loss].
static func rules() -> PackedInt64Array:
	var tuning := BattleTuning.current()
	return PackedInt64Array(
		[
			tuning.morale_per_leadership,
			tuning.morale_steady_at,
			tuning.morale_shaken_at,
			tuning.morale_loss,
			tuning.morale_leader_loss,
		]
	)
