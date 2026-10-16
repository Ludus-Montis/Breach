"""Tests for check_one_health.py: a write to the unit store's HP column outside UnitHealth
is caught; UnitHealth's own calls, reads and other columns are not."""
import unittest

from check_one_health import violations_in


class CheckOneHealthTest(unittest.TestCase):
    def test_a_write_to_the_hp_column_is_caught(self):
        for code in [
            "\tcolumns.put(unit.slot, UnitColumns.HP, 0)",
            "\tsquad.store.columns.put(unit.slot, UnitColumns.HP, hp - blow[2])",
            "\tstore.columns.commit(UnitColumns.HP, hp)",
            "\thealth.hp[unit.slot] = 0",
            "\ttable.hp[unit.slot] -= striker.dmg",
        ]:
            self.assertEqual(len(violations_in(code)), 1, code)

    def test_unit_health_and_reads_pass(self):
        for code in [
            "\tUnitHealth.hurt(self, blow[1], blow[2])",
            "\tUnitHealth.put(squad, unit, maxi(UnitHealth.of(squad, unit), 1))",
            "\tvar hp: PackedInt32Array = columns.column(UnitColumns.HP)",
            "\tvar left: int = columns.value(unit.slot, UnitColumns.HP)",
            "\tif health.hp[unit.slot] == 0:",
            "\tcolumns.put(unit.slot, UnitColumns.ATTACK_COOLDOWN, 1)",
            "\tgrem.hp = STOUT  # a UnitDef's",
        ]:
            self.assertEqual(violations_in(code), [], code)

    def test_a_comment_is_not_a_write(self):
        self.assertEqual(violations_in("\t# columns.put(slot, UnitColumns.HP, 0)"), [])


if __name__ == "__main__":
    unittest.main()
