"""Tests for check_store_reads.py: a store read unit by unit inside a loop is caught, and so
is a whole-column copy (a view or a column) inside a loop (Decision 150); reads outside
loops, fetched columns, a held view's `of` and marked loops are not."""
import unittest

from check_store_reads import violations_in


class CheckStoreReadsTest(unittest.TestCase):
    def test_a_scattered_read_or_write_inside_a_loop_is_caught(self):
        for call in [
            "var at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)",
            "columns.put(unit.slot, UnitColumns.NEXT, at)",
            "squad.store.columns.value(unit.slot, UnitColumns.BEARING)",
            "var rank := UnitPlace.rank(squad, unit)",
            "UnitPlace.put(squad, unit, place)",
            "var at := SquadEntries.loose_at(squad, unit)",
            "EntryKeys.keep(columns, unit.slot, UnitColumns.CAUGHT, 1)",
            "var at := FormationRout.where(squad, unit.id)",
            "var hp := UnitHealth.of_one(squad, unit)",
            "UnitHealth.hurt_one(squad, unit, 3)",
            "UnitHealth.heal_one(squad, unit, 3)",
            "UnitHealth.put_one(squad, unit, 0)",
        ]:
            code = f"func f():\n\tfor unit in squad.living():\n\t\t{call}\n"
            self.assertEqual(len(violations_in(code)), 1, call)

    def test_reads_outside_loops_and_fetched_columns_are_not(self):
        code = (
            "func f():\n"
            "\tvar at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)\n"
            "\tvar points: PackedVector2Array = columns.column(UnitColumns.POSITION)\n"
            "\tvar view := UnitView.read(squad, [UnitColumns.RANK])\n"
            "\tvar ranks: PackedInt32Array = view.rank\n"
            "\tfor unit in squad.living():\n"
            "\t\tvar here := points[unit.slot]\n"
            "\t\tvar rank := ranks[unit.slot]\n"
            "\t\tUnitHealth.hurt(view, unit, 3)\n"
            "\tcolumns.commit(UnitColumns.POSITION, points)\n"
        )
        self.assertEqual(violations_in(code), [])

    def test_a_marked_line_or_loop_is_not_caught(self):
        line = (
            "func f():\n\tfor unit in moved:\n"
            "\t\tvar rank := UnitPlace.rank(squad, unit)  # store: scattered (few: step-ups)\n"
        )
        self.assertEqual(violations_in(line), [])
        loop = (
            "func f():\n"
            "\t# store: scattered (reference: the GDScript reference's walk)\n"
            "\tfor unit in squad.living():\n"
            "\t\tvar at := SquadEntries.loose_at(squad, unit)\n"
            "\t\tcolumns.put(unit.slot, UnitColumns.NEXT, at)\n"
        )
        self.assertEqual(violations_in(loop), [])

    def test_a_reason_is_required(self):
        code = (
            "func f():\n\tfor unit in moved:\n"
            "\t\tvar rank := UnitPlace.rank(squad, unit)  # store: scattered ()\n"
        )
        self.assertEqual(len(violations_in(code)), 1)

    def test_a_mark_s_reason_is_few_or_reference(self):
        for reason in ["step-ups", "under 0.1 ms at the bench", "fewish: a trade"]:
            code = (
                "func f():\n\tfor unit in moved:\n"
                f"\t\tvar rank := UnitPlace.rank(squad, unit)  # store: scattered ({reason})\n"
            )
            self.assertEqual(len(violations_in(code)), 1, reason)
        loop = (
            "func f():\n"
            "\tfor unit in squad.living():  # store: scattered (bench: 0.1 ms)\n"
            "\t\tvar at := SquadEntries.loose_at(squad, unit)\n"
        )
        self.assertEqual(len(violations_in(loop)), 1)

    def test_a_read_in_a_loop_s_own_header_runs_once(self):
        code = (
            "func f():\n"
            "\tfor friend in friends_of(EntryKeys.held(columns, unit.slot, KEY)):\n"
            "\t\tpass\n"
        )
        self.assertEqual(violations_in(code), [])

    def test_the_loop_ends_with_its_body(self):
        code = (
            "func f():\n"
            "\tfor unit in squad.living():\n"
            "\t\tpass\n"
            "\tvar at := SquadEntries.loose_at(squad, unit)\n"
        )
        self.assertEqual(violations_in(code), [])

    def test_a_copy_inside_a_loop_over_squads_is_caught(self):
        for call in [
            "var view := UnitView.read(squad, [UnitColumns.RANK])",
            "var held := UnitView.hold([squad], HELD)",
            "var at: PackedVector2Array = squad.store.columns.column(UnitColumns.POSITION)",
        ]:
            code = f"func f():\n\tfor squad in squads:\n\t\t{call}\n"
            found = violations_in(code)
            self.assertEqual(len(found), 1, call)
            self.assertEqual(found[0][0], 3, call)  # it names the line

    def test_a_copy_marked_per_squad_is_not_caught(self):
        line = (
            "func f():\n\tfor squad in squads:\n"
            "\t\tvar view := UnitView.read(squad, NAMES)  # store: per squad (bench: 0.1 ms)\n"
        )
        self.assertEqual(violations_in(line), [])
        loop = (
            "func f():\n"
            "\t# store: per squad (two squads at most: a merge's)\n"
            "\tfor squad in merging:\n"
            "\t\tvar at = squad.store.columns.column(UnitColumns.POSITION)\n"
        )
        self.assertEqual(violations_in(loop), [])

    def test_a_per_squad_mark_needs_a_reason_and_covers_only_copies(self):
        empty = (
            "func f():\n\tfor squad in squads:\n"
            "\t\tvar view := UnitView.read(squad, NAMES)  # store: per squad ()\n"
        )
        self.assertEqual(len(violations_in(empty)), 1)
        scattered = (
            "func f():\n\tfor unit in moved:\n"
            "\t\tvar rank := UnitPlace.rank(squad, unit)  # store: per squad (few)\n"
        )
        self.assertEqual(len(violations_in(scattered)), 1)

    def test_a_held_view_s_of_and_a_read_once_in_a_loop_are_not_caught(self):
        code = (
            "func f():\n"
            "\tvar held := UnitView.hold(squads, HELD)\n"
            "\tvar views := {}\n"
            "\tfor squad in squads:\n"
            "\t\tvar view := UnitView.of(squad)\n"
            "\t\tvar places := UnitView.read_once(views, squad, UnitView.PLACES)\n"
            "\tUnitView.release(held)\n"
        )
        self.assertEqual(violations_in(code), [])


if __name__ == "__main__":
    unittest.main()
