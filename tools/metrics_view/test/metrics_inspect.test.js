// The repeats' panels, metrics_inspect.js (specs/47-tick-inspection.md, step 5b): each
// inspection tick's repeats (series `inspect`, step 5a) beside the same phase's lap from the
// whole tick, the 25k set's record of the same commit and shape, in % of tick by default, with
// Q47's caveat on the phases the repeats under-report.
// Run: node --test "tools/metrics_view/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const inspect = require('../metrics_inspect.js');

function measure(median) {
  return { unit: 'ms', values: [median], median };
}

function results(tick, phases) {
  const out = { tick: measure(tick) };
  Object.keys(phases).forEach((path) => { out[path] = measure(phases[path]); });
  return out;
}

function sha(position) {
  return String(position).repeat(40).slice(0, 40);
}

function repeat(position, preset, at, tick, phases, extra = {}) {
  return {
    path: `metrics/inspect/2026-11-12-${position}-${at}.json`,
    record: {
      schema: 'record-v1',
      series: 'inspect',
      commit: { sha: sha(position), position, pr: 900 + position },
      source: { harness: 'tick_phases-repeat', args: [`preset=${preset}`, `at=${at}`, 'repeat=50'] },
      results: results(tick, phases),
      reference: { commit: '8ca4227', results: { tick: measure(extra.reference || 100) } },
      machine: { cpu: extra.cpu || 'EPYC' },
      provenance: { run_id: extra.run_id || 1, kind: 'fresh', date: '2026-11-12' },
    },
  };
}

function lap(position, side, squads, tick, phases, extra = {}) {
  return {
    path: `metrics/bench/2026-11-12-${position}-${side}.json`,
    record: {
      schema: 'record-v1',
      series: 'bench',
      commit: { sha: sha(position), position, pr: 900 + position },
      source: { harness: 'breach_tick_phases',
        args: [`side=${side}`, `squads=${squads}`, 'series=1', 'series=settled'] },
      results: results(tick, phases),
      reference: { commit: '8ca4227', results: { tick: measure(extra.reference || 400) } },
      machine: { cpu: extra.cpu || 'EPYC' },
      provenance: { run_id: extra.run_id || 1, kind: 'fresh', date: '2026-11-12' },
    },
  };
}

function panel(chart, preset, phase) {
  return chart.sizes.find((s) => s.preset === preset).panels.find((p) => p.phase === phase);
}

test('a repeat pairs with the same commit\'s 25k set record of its preset\'s shape', () => {
  const bundle = [
    repeat(5, '4096', 91, 40, { 'tick/roster': 4 }),
    repeat(5, '25k', 91, 200, { 'tick/roster': 30 }),
    lap(5, 4096, 4, 32, { 'tick/roster': 4.8 }),
    lap(5, 24576, 24, 160, { 'tick/roster': 16 }),
    lap(5, 4096, 1, 20, { 'tick/roster': 9 }),
  ];
  const chart = inspect.chart(bundle, 'percent');
  assert.deepEqual(panel(chart, '4096', 'tick/roster').laps.map((p) => p.value), [15]);
  assert.deepEqual(panel(chart, '25k', 'tick/roster').laps.map((p) => p.value), [10]);
});

test('% of tick is the default: each side over its own tick, so no runner cancels into the gap', () => {
  const bundle = [repeat(5, '4096', 91, 40, { 'tick/roster': 4 }), lap(5, 4096, 4, 32, { 'tick/roster': 4.8 })];
  const roster = panel(inspect.chart(bundle), '4096', 'tick/roster');
  assert.equal(inspect.DEFAULT_UNIT, 'percent');
  assert.deepEqual(roster.repeats.map((p) => p.value), [10]);
  assert.deepEqual(roster.laps.map((p) => p.value), [15]);
});

test('in × the reference and in ms each side is over its own record\'s reference, or as timed', () => {
  const bundle = [repeat(5, '4096', 91, 40, { 'tick/roster': 4 }, { reference: 80 }),
    lap(5, 4096, 4, 32, { 'tick/roster': 4.8 }, { reference: 160 })];
  const reference = panel(inspect.chart(bundle, 'reference'), '4096', 'tick/roster');
  assert.deepEqual([reference.repeats[0].value, reference.laps[0].value], [0.05, 0.03]);
  const ms = panel(inspect.chart(bundle, 'ms'), '4096', 'tick/roster');
  assert.deepEqual([ms.repeats[0].value, ms.laps[0].value], [4, 4.8]);
});

test('each inspection tick of a commit is its own repeat point, named by its tick', () => {
  const bundle = [repeat(5, '4096', 91, 40, { 'tick/roster': 4 }), repeat(5, '4096', 170, 20, { 'tick/roster': 1 }),
    repeat(6, '4096', 91, 40, { 'tick/roster': 2 })];
  const roster = panel(inspect.chart(bundle), '4096', 'tick/roster');
  assert.deepEqual(roster.repeats.map((p) => [p.position, p.at, p.value]), [[5, 91, 10], [5, 170, 5], [6, 91, 5]]);
  assert.deepEqual(roster.laps, []);
});

test('a panel is a top-level phase, a finer one summed into it, and a gap is no point', () => {
  const bundle = [repeat(5, '4096', 91, 40, { 'tick/bodies/pushes': 1, 'tick/bodies/pair-search': 3 }),
    repeat(6, '4096', 91, 40, { 'tick/roster': 2 })];
  const chart = inspect.chart(bundle);
  assert.deepEqual(panel(chart, '4096', 'tick/bodies').repeats.map((p) => [p.position, p.value]), [[5, 10]]);
  assert.deepEqual(panel(chart, '4096', 'tick/roster').repeats.map((p) => p.position), [6]);
});

test('Q47\'s caveat marks the panels holding a phase the repeats under-report', () => {
  const bundle = [repeat(5, '4096', 91, 40, { 'tick/strays': 1, 'tick/groups-form-up-split': 1,
    'tick/bodies/pushes': 1, 'tick/stamina': 1, 'tick/roster': 1 })];
  const chart = inspect.chart(bundle);
  const caveat = chart.sizes[0].panels.filter((p) => p.caveat).map((p) => p.phase).sort();
  assert.deepEqual(caveat, ['tick/bodies', 'tick/groups-form-up-split', 'tick/stamina', 'tick/strays']);
});

test('a commit\'s gap is its repeats\' median over its lap, at its latest commit with both', () => {
  const bundle = [
    repeat(5, '4096', 91, 40, { 'tick/roster': 8 }), lap(5, 4096, 4, 40, { 'tick/roster': 4 }),
    repeat(6, '4096', 91, 40, { 'tick/roster': 2 }), repeat(6, '4096', 104, 40, { 'tick/roster': 4 }),
    repeat(6, '4096', 170, 40, { 'tick/roster': 6 }), lap(6, 4096, 4, 40, { 'tick/roster': 8 }),
  ];
  const roster = panel(inspect.chart(bundle), '4096', 'tick/roster');
  assert.deepEqual(roster.gap, { position: 6, sha: sha(6), ratio: 0.5 });
});

test('a rerun of a commit\'s tick keeps its latest run; hidden CPUs and other series stay out', () => {
  const bundle = [
    repeat(5, '4096', 91, 40, { 'tick/roster': 4 }, { run_id: 1 }),
    repeat(5, '4096', 91, 40, { 'tick/roster': 8 }, { run_id: 2 }),
    repeat(6, '4096', 91, 40, { 'tick/roster': 4 }, { cpu: 'Xeon' }),
    lap(5, 4096, 4, 32, { 'tick/roster': 4.8 }, { cpu: 'Xeon' }),
  ];
  const roster = panel(inspect.chart(bundle, 'percent', { Xeon: true }), '4096', 'tick/roster');
  assert.deepEqual(roster.repeats.map((p) => [p.position, p.value, p.cpu]), [[5, 20, 'EPYC']]);
  assert.deepEqual(roster.laps, []);
  assert.deepEqual(inspect.chart([lap(5, 4096, 4, 32, { 'tick/roster': 4 })]).sizes, []);
});

test('the laps start where the repeats do: a set from before them has nothing to pair with', () => {
  const bundle = [lap(4, 4096, 4, 40, { 'tick/roster': 4 }), lap(5, 4096, 4, 40, { 'tick/roster': 4 }),
    repeat(5, '4096', 91, 40, { 'tick/roster': 4 })];
  assert.deepEqual(panel(inspect.chart(bundle), '4096', 'tick/roster').laps.map((p) => p.position), [5]);
});
