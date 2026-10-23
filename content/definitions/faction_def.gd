class_name FactionDef
extends Resource
## An author-definable faction, per specs/13-faction-def-and-garrison-unit.md. Real
## content, not a closed enum - a map author can define any faction (e.g. "The
## Kingdom"), not just a fixed player/enemy pair. "Player" is authoring convention
## (an id like any other, e.g. "player"), not a hardcoded special case, matching how
## sim/'s existing ad-hoc ownership strings already work.

@export var id: String = ""
@export var display_name: String = ""
