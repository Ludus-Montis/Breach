class_name GarrisonUnitDef
extends Resource
## One unit's assignment at a node, per specs/13-faction-def-and-garrison-unit.md.
## Deliberately minimal: affiliation/behavior fields only, not combat stats -
## NodeDef.garrison/garrison_hp/garrison_dmg (pre-existing, Decision 11, already read
## live by sim/lane_simulation.gd) stay pooled on the node for now. A full "structure
## is just an immobile unit" unification would mean adopting the combat addendum's
## whole Combatant/Encounter model (which also subsumes UnitDef/ResponseUnitDef) -
## large, separate, future work, not this stub.
##
## can_sortie/patrol_route/delivery_target_id are each unit's BASELINE STANDING
## ORDER - what it starts the map with, not a fixed property. A future Command &
## Control system (messenger-delivered orders, interceptable, tiered by delivery
## assurance) is expected to let these change at runtime; this schema only authors
## the starting state.

## Cross-referenced against the map's faction roster in the crate's map rules.
@export var faction_id: String = ""

## Matches the combat addendum's already-drafted sortie concept (temporary
## intercept, returns after). Baseline order - see class doc.
@export var can_sortie: bool = false

## Ordered node ids this unit patrols between, starting out. Empty = static.
## Cross-referenced against the map's real node ids in the crate's map rules. Baseline
## order - see class doc.
@export var patrol_route: Array[String] = []

## Which node this unit's future worker-delivery route targets, per the combat
## addendum's Worker-as-Combatant section. Cross-referenced against the map's real
## node ids in the crate's map rules. Baseline order - see class doc.
@export var delivery_target_id: String = ""
