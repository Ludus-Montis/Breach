class_name MapFill
extends RefCounted
## A MapDef from a map's fields as the Rust crate holds them (content/maps/<name>.json, spec 39
## step 2c, Decision 153): every node once under "nodes", and each lane naming its nodes by
## id. A node on several lanes (the home) is one NodeDef, and a node on no lane is off-lane,
## in the order "nodes" lists it. The rest is filled by name (ContentFill).


static func build(fields: Dictionary) -> MapDef:
	var nodes := {}
	for entry: Dictionary in fields.get("nodes", []):
		nodes[entry["id"]] = ContentFill.fill(NodeDef.new(), entry)
	var rest := fields.duplicate()
	rest.erase("nodes")
	rest.erase("lanes")
	var map: MapDef = ContentFill.fill(MapDef.new(), rest)
	var on_lane := {}
	for entry: Dictionary in fields.get("lanes", []):
		var lane := LaneDef.new()
		lane.id = entry["id"]
		lane.player_home_index = entry["player_home_index"]
		for id: String in entry["nodes"]:
			lane.nodes.append(nodes[id])
			on_lane[id] = true
		map.lanes.append(lane)
	for id: String in nodes:
		if not on_lane.has(id):
			map.off_lane_nodes.append(nodes[id])
	return map
