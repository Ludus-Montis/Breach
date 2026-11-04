class_name SquadRoutes
extends RefCounted
## The squads' routes as the crate holds them (spec 39 step 8b5-xxxvii): SkirmishSquad.route
## reads and writes them here. The crate names each route by a code (`Squads::route_code`;
## the same code, the same route), so a pass there that hands a squad another's route (a
## pursuer its quarry's, a split's group its squad's) or a route of its own (a group re-routed
## round its command's way, a pursuer back at its post) needs no copy made by hand: a squad
## reads the route kept for its code, and one the crate made is made here once, from the
## crate's points and corridor, and kept for its code.

const FormationRoute = preload("res://sim/skirmish/formation/formation_route.gd")

var _field: Object
var _routes := {}  # code -> the route


func _init(field: Object) -> void:
	_field = field


## The route the squad in `slot` follows (null: a straight lane along x).
func of(slot: int) -> FormationRoute:
	var code: int = _field.squad_route_code(slot)
	if code < 0:
		return null
	var route: FormationRoute = _routes.get(code)
	if route == null:
		var half_width: float = _field.squad_route_half_width(slot)
		route = FormationRoute.new(_field.squad_route(slot), half_width)
		_field.name_route(code, route.get_instance_id())
		_routes[code] = route
	return route


## Sets the route the squad in `slot` follows (null: a straight lane along x).
func put(slot: int, route: FormationRoute) -> void:
	var points := PackedVector2Array() if route == null else route.points()
	var half_width := 0.0 if route == null else route.corridor_half_width
	var object := 0 if route == null else route.get_instance_id()
	_field.squad_put_route(slot, route != null, points, half_width, object)
	if route != null:
		_routes[_field.squad_route_code(slot)] = route


## Sets the route of the squad in slot `to` to the one `from`'s squad in `slot` followed:
## a squad carried to this store (StoredSquad.store).
func carry(from: SquadRoutes, slot: int, to: int) -> void:
	put(to, from.of(slot))
