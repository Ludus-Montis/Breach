//! What a cut is taken from and stepped on (spec 39 step 8d): a digest battle's `Play`, or a
//! field log's FieldPlay with the record whose commands it is given before each tick.

use crate::battles;
use crate::content::Content;
use crate::digest::runs::{Kind, Run};
use crate::events::Log;
use crate::field::Field;
use crate::field_play::carried::Carried;
use crate::field_play::FieldPlay;
use crate::play::Play;
use crate::record::{self, Record};
use crate::squads::Squads;

/// A run being played on one thread.
pub(super) enum Played {
    Battle(Box<Play>),
    Field(Box<(FieldPlay, Record)>),
}

impl Played {
    /// `run` set up on `squads` and `field`, seeded as it is; None if it can't be.
    pub(super) fn opened(
        run: &Run,
        crate_: (&mut Squads, &mut Field),
        content: &Content,
    ) -> Option<Played> {
        match &run.kind {
            Kind::Battle { name, numbers, seed, .. } => {
                let battle = battles::named(name, numbers)?;
                let play = Play::new(&battle, crate_, content, (*seed, run.rolls, false))?;
                Some(Played::Battle(Box::new(play)))
            }
            Kind::Field(text) => {
                let record = record::parse(text)?;
                let at = (record.seed, run.rolls, false);
                let field = FieldPlay::new(record.captained, crate_, content, at)?;
                Some(Played::Field(Box::new((field, record))))
            }
        }
    }

    /// `run` read back at `tick`, seeded `seed`, with the squads `listed` and, for a field,
    /// its own state `carried` (written by `Carried::write`); None if it doesn't fit.
    pub(super) fn resumed(
        run: &Run,
        (tick, seed): (i64, i64),
        (listed, carried): (&[usize], &[u8]),
        content: &Content,
    ) -> Option<Played> {
        let (mut squads, mut field) = (Squads::default(), Field::default());
        match Played::opened(run, (&mut squads, &mut field), content)? {
            Played::Battle(opened) => {
                let mut play = Play::open(opened.set, (seed, run.rolls, false), content);
                play.listed = listed.to_vec();
                play.tick = tick;
                Some(Played::Battle(Box::new(play)))
            }
            Played::Field(opened) => {
                let record = opened.1;
                let carried = Carried::read(carried)?;
                let field = FieldPlay::carrying(&record, &carried, (tick, run.rolls), content)?;
                (field.play.listed == listed).then_some(Played::Field(Box::new((field, record))))
            }
        }
    }

    /// The battle's `Play`.
    pub(super) fn play(&self) -> &Play {
        match self {
            Played::Battle(play) => play,
            Played::Field(field) => &field.0.play,
        }
    }

    /// What a field keeps beside the crate's state, written; nothing for a battle.
    pub(super) fn carried(&self) -> Vec<u8> {
        match self {
            Played::Battle(_) => Vec::new(),
            Played::Field(field) => field.0.carried().write(),
        }
    }

    /// One tick, its events in `log`: a field's commands for the tick given first, as the
    /// digest's replay gives them. None if a battle stops.
    pub(super) fn step(
        &mut self,
        (squads, field): (&mut Squads, &mut Field),
        content: &Content,
        log: &mut Log,
    ) -> Option<()> {
        match self {
            Played::Battle(play) => play.step(squads, field, content, log).map(|_| ()),
            Played::Field(both) => {
                let (played, record) = &mut **both;
                let tick = played.play.tick;
                for given in record.commands.iter().filter(|c| c.tick == tick) {
                    played.apply(given, (squads, field), content);
                }
                played.step(squads, field, content, log);
                Some(())
            }
        }
    }
}
