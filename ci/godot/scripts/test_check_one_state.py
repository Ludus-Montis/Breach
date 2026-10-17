"""Tests for check_one_state.py: a write to a unit's state outside UnitState is caught, and
so is a change to a kept list that `living()` returned; squads' states, reads, copies and
UnitState's own calls are not."""
import unittest

from check_one_state import violations_in


class CheckOneStateTest(unittest.TestCase):
    def test_a_write_to_a_units_state_is_caught(self):
        for code in [
            "\tunit.state = SkirmishUnit.State.DEAD",
            "\t\tbody.state = SkirmishUnit.State.CARRIED",
            "\tcaught.state = SkirmishUnit.State.TAKEN  # surrendered",
        ]:
            self.assertEqual(len(violations_in(code)), 1, code)

    def test_unit_state_squads_and_reads_pass(self):
        for code in [
            "\tUnitState.put(squad, unit, SkirmishUnit.State.DEAD)",
            "\tsquad.state = SkirmishSquad.State.ROUTING",
            "\tgroup.state = squad.state",
            "\tif unit.state == SkirmishUnit.State.DOWNED:",
            "\tvar downed: bool = unit.state in [SkirmishUnit.State.DOWNED]",
        ]:
            self.assertEqual(violations_in(code), [], code)

    def test_a_change_to_a_kept_list_is_caught(self):
        for code in [
            "\tsquad.living().sort_custom(func(a, b): return a.id < b.id)",
            "\tmemo.living(squad).append(unit)",
            "func close(squad):\n\tvar order := squad.living()\n\torder.sort_custom(by_place)",
            "func f(squad):\n\tvar ours: Array = squad.living()  # its own\n\tours.erase(unit)",
            "func f(squad):\n\tvar down := UnitState.downed(squad)\n\tdown.reverse()",
        ]:
            self.assertEqual(len(violations_in(code)), 1, code)

    def test_a_copy_or_another_functions_list_passes(self):
        for code in [
            "func f(squad):\n\tvar order := squad.living().duplicate()\n\torder.sort()",
            "func f(squad):\n\tvar order := squad.living()\nfunc g():\n\tvar order := []\n\torder.sort()",
            "func f(squad):\n\tvar near := squad.living().filter(close)\n\tnear.append(unit)",
            "\tfor unit in squad.living():",
        ]:
            self.assertEqual(violations_in(code), [], code)

    def test_start_is_for_tests_and_tools_only(self):
        code = "\tUnitState.start(body, SkirmishUnit.State.DOWNED)"
        self.assertEqual(violations_in(code), [])
        self.assertEqual(len(violations_in(code, in_sim=True)), 1)

    def test_a_comment_is_not_a_write(self):
        self.assertEqual(violations_in("\t# unit.state = SkirmishUnit.State.DEAD"), [])


if __name__ == "__main__":
    unittest.main()
