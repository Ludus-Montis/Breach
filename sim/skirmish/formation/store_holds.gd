class_name StoreHolds
extends RefCounted
## Takes and gives back a pass's holds on stores' columns (ColumnsHeld): while held, a
## store's `columns` serves the named columns from local copies, and giving the hold back
## commits what changed and restores the store's own columns.

const ColumnsHeld = preload("res://sim/skirmish/formation/columns_held.gd")
## What a native entry asserts (none_held).
const SPANS := "no hold spans a native call (Decision 146)"


## Holds `names` on every store `squads` use; returns those stores, for `release`.
static func hold(squads: Array, names: Array) -> Array:
	var stores := []
	for squad in squads:
		if not stores.has(squad.store):
			stores.append(squad.store)
	for store in stores:
		hold_store(store, names)
	return stores


## Holds `names` of one store's columns until it is released.
static func hold_store(store: UnitStore, names: Array) -> void:
	assert(not store.columns is ColumnsHeld, "a store is held by one pass at a time")
	store.columns = ColumnsHeld.new(store.columns, names)


## Commits the held columns of each of `stores` and gives each its own columns back. A
## store not held is left as it is.
static func release(stores: Array) -> void:
	for store: UnitStore in stores:
		if store.columns is ColumnsHeld:
			var held: ColumnsHeld = store.columns
			held.commit_held()
			store.columns = held.inner


## Whether `store` is held by no pass: every GDScript entry into a native pass asserts it,
## since the native pass reads the field's own columns, not the hold's (Decision 146).
static func unheld(store: UnitStore) -> bool:
	return store == null or not store.columns is ColumnsHeld


## Whether no store `squads` use is held: each GDScript entry into a native pass asserts it.
static func none_held(squads: Array) -> bool:
	for squad in squads:
		if not unheld(squad.store):
			return false
	return true
