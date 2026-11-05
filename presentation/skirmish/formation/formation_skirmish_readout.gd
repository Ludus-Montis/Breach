class_name FormationSkirmishReadout
extends RefCounted
## The formation feel test's HUD text (specs/22-formation-feel-test.md): pure formatting
## over what the crate's BattleState gives (spec 39 step 9d-ii), kept out of
## FormationSkirmishScene so that stays glue.


static func status(seconds: float, speed: float, paused: bool) -> String:
	return "%.1f s · ×%d%s" % [seconds, int(speed), " · PAUSED" if paused else ""]


## pool: BattleState.pool().
static func pool(slots: Dictionary) -> String:
	var assigned: Dictionary = slots.get("assigned", {})
	return (
		"Slot pool %d: c %d · k %d · free %d · reserve %d/%d"
		% [
			slots.get("total", 0),
			assigned.get("c", 0),
			assigned.get("k", 0),
			slots.get("free", 0),
			slots.get("reserve", 0),
			slots.get("reserve_cap", 0)
		]
	)


## One line per builder type: "Grem ×2: 60% → c · 20% → reserve" (Decision 45). builds:
## BattleState.builds(); counts: the builders of each type, in the names' order.
static func builders(builds: Array, counts: Array, names: Array) -> Array:
	var lines := []
	for index in range(names.size()):
		var under_way := builds.filter(func(b): return b[0] == index)
		var parts := under_way.map(
			func(b): return "%d%% → %s" % [roundi(b[2] * 100.0), b[1] if b[1] != "" else "reserve"]
		)
		var doing := " · ".join(parts) if not parts.is_empty() else "idle"
		lines.append("%s ×%d: %s" % [names[index], counts[index], doing])
	return lines


## [summary text, template places] for one lane, from BattleState.wave(). Each place is
## [rank, column, depth, width, filled, preferred position, kind, progress]: the builds
## claimed for this lane are shown filling its front-most unfilled places of their type.
static func lane(wave: Dictionary, allowance: int, builds: Array, lane_key: String) -> Array:
	var places: Array = wave.get("places", []).duplicate(true)
	var claimed := builds.filter(func(b): return b[1] == lane_key)
	var used := 0
	for place in places:
		used += place[2] * place[3]
		var progress := 0.0
		if not place[4]:
			var build_at := claimed.find_custom(func(b): return b[0] == place[6])
			if build_at != -1:
				progress = claimed[build_at][2]
				claimed.remove_at(build_at)
		place.append(progress)
	var built: int = wave.get("built", 0)
	var full := built > 0 and built == places.size()
	var state := "ready - send it" if full else "building"
	var text := "cells %d/%d · built %d/%d · %s" % [used, allowance, built, places.size(), state]
	return [text, places]


## chosen: [lane key, squad id], or empty when nothing is selected; squad: that squad as
## BattleState.squads() gives it, or null if it is no longer listed.
static func selected(chosen: Array, squad) -> String:
	if chosen.is_empty():
		return "Click a squad (or Tab) to select it"
	if squad == null or squad["state"] == "DESTROYED":
		return "Selected squad is gone"
	return (
		"Lane %s squad #%d: %d alive · %s · %s"
		% [chosen[0], squad["id"], squad["units"].size(), squad["order"], squad["state"]]
	)
