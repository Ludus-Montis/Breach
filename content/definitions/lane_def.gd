class_name LaneDef
extends Resource
## One lane's ordered node sequence - the explicit "connection graph" for a
## track-based map (Decision 4/Decision 19: tracks, not an open grid). See
## specs/09-node-graph-and-lanes.md.
##
## Array order in nodes IS the lane's path: adjacent indices are connected. No
## separate edge-list is modeled - not needed for a track, per the parent spec's own
## rejection of an open-grid design.

@export var id: String = ""
@export var nodes: Array[NodeDef] = []

## Which index in nodes is the player's home - needed since more than one node can
## share NodeType.ORIGIN (e.g. the player's home and the far-end Core both do today).
@export var player_home_index: int = 0
