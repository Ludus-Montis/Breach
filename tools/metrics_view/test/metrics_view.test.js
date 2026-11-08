// The metrics page's logic, metrics_view.js (specs/43-metrics-history.md, step 7): records
// into lines per harness, never joined across eras; the bridge's factor; two commits
// compared at the deepest phase level both have.
// Run: node --test "tools/metrics_view/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const view = require('../metrics_view.js');

const BRIDGE = '8ca422726e68909554eddad7eeee2199e0fe656d';

function measure(median) {
  return { unit: 'ms', values: [median], median };
}

function record(sha, position, harness, results, extra = {}) {
  return {
    path: `metrics/bench/2026-11-08-1-${sha.slice(0, 3)}${harness.length}.json`,
    record: {
      schema: 'record-v1',
      series: 'bench',
      commit: { sha, position, pr: 700 + position },
      source: { harness },
      results,
      reference: { commit: 'r', results: {}, ratio: { tick: 1.0 } },
      provenance: { run_id: extra.run_id || 1, kind: extra.kind || 'fresh', date: '2026-11-08' },
    },
  };
}

test('the deepest level both commits have is where they are compared', () => {
  const coarse = { tick: measure(40), 'tick/scrum': measure(12) };
  const fine = { tick: measure(38), 'tick/scrum/seek': measure(9), 'tick/scrum/walk': measure(2) };
  const rows = view.compare(coarse, fine);
  assert.deepEqual(rows.map((row) => row.path), ['tick', 'tick/scrum']);
  assert.deepEqual(rows[1], { path: 'tick/scrum', a: 12, b: 11, finer: ['tick/scrum/seek', 'tick/scrum/walk'] });
});

test('a path one side lacks entirely is a gap, not a zero', () => {
  const rows = view.compare({ tick: measure(40), 'tick/rout': measure(1) }, { tick: measure(38) });
  assert.deepEqual(rows[1], { path: 'tick/rout', a: 1, b: null, finer: [] });
});

test('a gap in a record is no value, and a parent sums its measured children', () => {
  const results = { 'tick/scrum/seek': measure(9), 'tick/scrum/walk': measure(2),
    'tick/morale': { unit: 'ms', values: { na: 'not-measured' }, median: { na: 'not-measured' } } };
  assert.equal(view.valueAt(results, 'tick/scrum'), 11);
  assert.equal(view.valueAt(results, 'tick/morale'), null);
});

test('records from both eras make separate lines, broken at the bridge, with its factor', () => {
  const records = [
    record('a'.repeat(40), 10, 'tick_phases.gd', { tick: measure(400) }),
    record(BRIDGE, 20, 'tick_phases.gd', { tick: measure(156) }),
    record(BRIDGE, 20, 'breach_tick_phases', { tick: measure(94) }),
    record('b'.repeat(40), 30, 'breach_tick_phases', { tick: measure(42) }),
  ];
  const lines = view.lines(records);
  assert.deepEqual(lines.map((line) => [line.harness, line.points.map((p) => p.position)]),
    [['tick_phases.gd', [10, 20]], ['breach_tick_phases', [20, 30]]]);
  const bridge = view.bridge(records);
  assert.equal(bridge.sha, BRIDGE);
  assert.equal(bridge.gdscript, 156);
  assert.equal(bridge.cargo, 94);
  assert.equal(bridge.factor.toFixed(2), '1.66');
});

test('harvested records are kept apart from fresh ones', () => {
  const records = [
    record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) }),
    record('c'.repeat(7), null, 'breach_tick_phases', { tick: measure(51) }, { kind: 'harvested' }),
  ];
  records[1].record.commit = { sha: 'ccccccc', base: 'aaaaaaa', position: { na: 'not-recorded' } };
  const lines = view.lines(records);
  assert.equal(lines[0].points.length, 1);
  assert.deepEqual(lines[0].harvested.map((p) => [p.position, p.tick]), [[10, 51]]);
});

test('a point carries the reference\'s whole tick, measured in the same job', () => {
  const item = record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) });
  item.record.reference.results = { tick: measure(94) };
  assert.equal(view.lines([item])[0].points[0].referenceTick, 94);
});

test('a commit benched twice draws its latest run', () => {
  const records = [
    record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) }, { run_id: 5 }),
    record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(48) }, { run_id: 9 }),
  ];
  assert.deepEqual(view.lines(records)[0].points.map((p) => p.tick), [48]);
});

test('each point links to its PR and its record', () => {
  const point = view.lines([record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) })])[0].points[0];
  assert.equal(point.pr, 'https://github.com/Ludus-Montis/Breach/pull/710');
  assert.equal(point.referenceTick, null);
  assert.match(point.recordUrl, /^https:\/\/github.com\/Ludus-Montis\/Breach\/blob\/main\/metrics\/bench\//);
});

test('a line breaks where a commit it should have has no number, never joining across', () => {
  const points = [10, 20, 30, 50].map((position) => ({ position, tick: position === 30 ? null : 1 }));
  const runs = view.segments(points, [10, 20, 30, 40, 50]);
  assert.deepEqual(runs.map((run) => run.map((p) => p.position)), [[10, 20], [50]]);
});

test('harvested points are hidden unless asked for, and the fresh lines are untouched', () => {
  const records = [
    record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) }),
    record('c'.repeat(7), null, 'breach_tick_phases', { tick: measure(51) }, { kind: 'harvested' }),
  ];
  records[1].record.commit = { sha: 'ccccccc', base: 'aaaaaaa' };
  const all = view.lines(records);
  const hidden = view.shown(all, false);
  assert.equal(hidden[0].harvested.length, 0);
  assert.equal(hidden[0].points.length, 1);
  assert.equal(view.shown(all, true)[0].harvested.length, 1);
  assert.equal(all[0].harvested.length, 1);
});
