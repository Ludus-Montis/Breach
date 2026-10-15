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
        ]:
            code = f"func f():\n\tfor unit in squad.living():\n\t\t{call}\n"
            self.assertEqual(len(violations_in(code)), 1, call)

    def test_reads_outside_loops_and_fetched_columns_are_not(self):
        code = (
            "func f():\n"
            "\tvar at: Vector2 = columns.value(unit.slot, UnitColumns.POSITION)\n"
            "\tvar points: PackedVector2Array = columns.column(UnitColumns.POSITION)\n"
            "\tvar places := UnitPlace.table(squad)\n"
            "\tfor unit in squad.living():\n"
            "\t\tvar here := points[unit.slot]\n"
            "\t\tvar place := UnitPlace.read(places, squad, unit)\n"
            "\tcolumns.commit(UnitColumns.POSITION, points)\n"
        )
        self.assertEqual(violations_in(code), [])

    def test_a_marked_line_or_loop_is_not_caught(self):
        line = (
            "func f():\n\tfor unit in moved:\n"
            "\t\tvar rank := UnitPlace.rank(squad, unit)  # store: scattered (step-ups)\n"
        )
        self.assertEqual(violations_in(line), [])
        loop = (
            "func f():\n"
            "\t# store: scattered (the GDScript reference's walk)\n"
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
