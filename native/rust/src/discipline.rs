//! A formation's discipline (FormationDiscipline, Decisions 81, 92, 109 and 112; spec 39
//! step 8b5-v), read from its living units' sheets (`Columns::sheet`, step 8b5-i): their
//! mean discipline, rounded, bolstered by its best living leader's leadership. Short of
//! meeting threats it is disordered, by how far short. A unit's own discipline is bolstered
//! by the same leader, and sets how far it chases on its own (its leash). Each answer reads
//! one formation alone (Decision 97).

use crate::bands;
use crate::columns::Columns;
use crate::roster::Listed;

/// The tuning the rules read (BattleTuning's `discipline_*`).
#[derive(Clone, Debug, Default)]
pub struct DisciplineTuning {
    pub per_leadership: i64,
    pub meets_threats: i64,
    pub expected_max: f64,
    pub leash_shares: Vec<f64>,
    pub leash_steps: Vec<f64>,
}

/// The best living leader's leadership of the listed squad.
pub fn leadership(columns: &Columns, listed: &Listed) -> i64 {
    let leaderships: Vec<i64> =
        listed.living.iter().map(|m| columns.sheet[m.slot].leadership).collect();
    bands::leadership(&leaderships)
}

/// FormationDiscipline.of: 0 with none living.
pub fn of(columns: &Columns, listed: &Listed, t: &DisciplineTuning) -> i64 {
    if listed.living.is_empty() {
        return 0;
    }
    let total: i64 = listed.living.iter().map(|m| columns.sheet[m.slot].discipline).sum();
    let mean = (total as f64 / listed.living.len() as f64).round() as i64;
    mean + t.per_leadership * leadership(columns, listed)
}

/// FormationWithdraw.disorder: how far from orderly it is, 0 (meets threats) to 1 (none).
pub fn disorder(columns: &Columns, listed: &Listed, t: &DisciplineTuning) -> f64 {
    let held = of(columns, listed, t);
    if held >= t.meets_threats {
        return 0.0;
    }
    1.0 - held as f64 / t.meets_threats as f64
}

/// FormationDiscipline.unit_discipline: the unit in store slot `slot`'s, bolstered by its
/// squad's best leader, whose leadership is `leadership`.
pub fn unit(columns: &Columns, slot: usize, leadership: i64, t: &DisciplineTuning) -> i64 {
    columns.sheet[slot].discipline + t.per_leadership * leadership
}

/// FormationDiscipline._leash: the step of `discipline`, moved by `shift`.
pub fn leash(discipline: f64, shift: i64, t: &DisciplineTuning) -> f64 {
    let mut step = t.leash_shares.len() as i64 + 1;
    for (index, &share) in t.leash_shares.iter().enumerate() {
        if discipline / t.expected_max >= share {
            step = index as i64 + 1;
            break;
        }
    }
    t.leash_steps[(step + shift).clamp(0, t.leash_steps.len() as i64 - 1) as usize]
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ground::Walker;
    use crate::roster::Member;

    /// Members in these store slots, their ids their slots.
    pub fn members(slots: &[usize]) -> Vec<Member> {
        slots.iter().map(|&slot| member(slot)).collect()
    }

    fn member(slot: usize) -> Member {
        let walker = Walker { height: 1.0, swimmer: 0, climber: 0, swims: false, climbs: false };
        Member {
            id: slot as i64,
            slot,
            foot: (1, 1),
            preferred: 0,
            attack_range: 0,
            kind: 0,
            turn_rate: 90.0,
            backward_pace: 0.5,
            detection: 8.0,
            max_stamina: 100.0,
            walker,
        }
    }

    fn tuning() -> DisciplineTuning {
        DisciplineTuning {
            per_leadership: 5,
            meets_threats: 50,
            expected_max: 100.0,
            leash_shares: vec![0.8, 0.5],
            leash_steps: vec![2.0, 4.0, 8.0, 16.0],
        }
    }

    /// A squad of units with these disciplines and leaderships.
    fn squad(units: &[(i64, i64)]) -> (Columns, Listed) {
        let mut columns = Columns::default();
        let mut listed = Listed::default();
        for &(discipline, leadership) in units {
            let slot = columns.position.len();
            columns.grow();
            columns.sheet[slot].discipline = discipline;
            columns.sheet[slot].leadership = leadership;
            listed.living.push(member(slot));
        }
        (columns, listed)
    }

    #[test]
    fn a_formation_is_its_mean_discipline_rounded_and_bolstered_by_its_best_leader() {
        let (columns, listed) = squad(&[(30, 0), (31, 2), (30, 1)]);
        assert_eq!(of(&columns, &listed, &tuning()), 30 + 10);
        assert_eq!(of(&columns, &Listed::default(), &tuning()), 0);
    }

    #[test]
    fn short_of_meeting_threats_it_is_disordered_by_how_far_short() {
        let (columns, listed) = squad(&[(20, 1), (20, 0)]);
        assert_eq!(disorder(&columns, &listed, &tuning()), 0.5);
        let (columns, listed) = squad(&[(45, 1)]);
        assert_eq!(disorder(&columns, &listed, &tuning()), 0.0);
        assert_eq!(disorder(&columns, &Listed::default(), &tuning()), 1.0);
    }

    #[test]
    fn a_unit_is_bolstered_by_its_leader_and_its_leash_is_its_steps() {
        let (columns, _) = squad(&[(40, 0)]);
        assert_eq!(unit(&columns, 0, 3, &tuning()), 55);
        assert_eq!(leash(80.0, 0, &tuning()), 4.0);
        assert_eq!(leash(55.0, 0, &tuning()), 8.0);
        assert_eq!(leash(10.0, 0, &tuning()), 16.0);
        assert_eq!(leash(10.0, 5, &tuning()), 16.0);
        assert_eq!(leash(80.0, -3, &tuning()), 2.0);
    }
}
