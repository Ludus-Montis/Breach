"""Tests for check_store_reads.py: a store read unit by unit inside a loop is caught; reads
outside loops, fetched columns and marked loops are not."""
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


if __name__ == "__main__":
    unittest.main()
