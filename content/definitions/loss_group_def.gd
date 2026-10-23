class_name LossGroupDef
extends Resource
## One of a faction's loss conditions, per specs/19-map-layout-and-objectives.md: a
## set of critical-asset nodes and a rule. ANY = the faction loses when any one of them
## is captured; ALL = only once every one is. A faction's groups are OR'd - it is
## knocked out when any one group triggers. Authored data; sim/ doesn't evaluate it yet.

enum Rule { ANY, ALL }

@export var faction_id: String = ""
@export var display_name: String = ""
@export var rule: Rule = Rule.ANY
@export var node_ids: Array[String] = []
