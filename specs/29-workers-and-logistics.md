# Spec 29: Workers and logistics

Decisions 2 (auto-extraction), 26 (roads built in play), 45 (domain builders and reserve),
54 (digging), 75 (routes), 77 and 79 (items), 80 (carried supplies), 136 and 141 (rubble,
spoil, and the squad order that keeps a digger's way back clear).

## Purpose

Mines need shoring timber, tunnels need lamps, roads need building, and resources need
hauling. The user raised **worker units** that move to and from work sites, and asked
whether they are another pool beside the reserves or something separate, with their own
tech (wagons). Carry capacity (Decision 80) would vary per unit: a brute carries more
than a grem, a hauler trait or a backpack raises it, and units on worker duty could be
outfitted with tools rather than weapons.

## Status

**Design conversation not started.** Raised during spec 27; a starting recommendation is
below, nothing is decided.

## Starting recommendation (to discuss)

- **Work is a duty, not a separate species.** Any unit can be put on work duty, and a
  faction may also have cheap non-combat worker types. Both use one model.
- **Work orders beside lanes.** A work order is a site and a task (dig a route, shore,
  build a road, haul between a node and the stockpile, extract). Crews for it come from
  the same builders and reserve (Decision 45), shared out by the same rule, and they
  shuttle along routes. How many work orders a player may run is its own progression
  lever, like the route count (Decision 75).
- **Carry capacity** grows with a unit's size (placeholder: one load unit per cell of
  footprint), plus **hauler N** from the unit or an item (a backpack).
- **Loadouts by duty.** A template's duty picks the kit: weapons for war, tools for work,
  drawn from the domain's item stock when it has them; otherwise units use what they
  have.
- **Wagons** are tech: a carrier that holds far more, needs pulling (crew or beasts, as
  emplacements need crew) and prefers roads and gentle slopes.
- **Crews are targets.** A bypassed garrison raids workers and supply (the "rear threat"
  open item); escorts come with this round.

## Agenda for the design conversation

1. Duty or species: one unit model with duties, or separate worker units?
2. Pools: crews from the shared reserve, or a separate worker pool?
3. Work orders: which tasks, and how a player sets one up.
4. Carrying: capacity by size, hauler trait, backpacks; what is carried (resources,
   supplies, items).
5. Loadouts by duty and the item stock.
6. Wagons and other logistics tech.
7. Danger: raids on crews, and escorts.
8. Medics: support units that heal formations over time (with spec 28's healing trait).

## Rounds

None yet.
