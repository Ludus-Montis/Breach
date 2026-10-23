class_name MapEdgeDef
extends Resource
## An explicit extra connection between two nodes, per
## specs/11-graph-topology-edges.md. Undirected at the topology layer - node_a_id/
## node_b_id carry no implied direction, deliberately not from/to (a future
## route-choice/retreat item decides how directionality, if any, factors in).
##
## A map's full adjacency graph is the union of each LaneDef's own implied path edges
## (consecutive array entries) and MapDef.edges' explicit extra connections like this
## one. Existence-of-referenced-node and duplicate-pair checks need the full
## node/edge set, so those live in the crate's map rules instead of here.

@export var node_a_id: String = ""
@export var node_b_id: String = ""
