class_name FactionRelationDef
extends Resource
## Declares how two factions relate, per specs/13-faction-def-and-garrison-unit.md.
## faction_a_id/faction_b_id reference FactionDef.id values (author-definable content,
## not a closed enum - see specs/13's rationale for why round 3's FactionId enum was
## replaced). Stance stays a closed enum: a fixed small set of relationship KINDS is
## genuinely different from open-ended faction IDENTITY. sim/'s existing ad-hoc
## String ownership is not migrated to this - separate, future work.

enum Stance { HOSTILE, NEUTRAL, ALLIED }

@export var faction_a_id: String = ""
@export var faction_b_id: String = ""
@export var stance: Stance = Stance.HOSTILE
