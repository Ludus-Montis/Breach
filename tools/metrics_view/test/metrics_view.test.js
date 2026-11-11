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

test('the 25k set\'s records (series=1, step 8a) stay off the 4,096 line until their own chart', () => {
  const set = record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(900) }, { run_id: 9 });
  set.record.source.args = ['side=24576', 'squads=24', 'series=1'];
  const records = [record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) }, { run_id: 5 }), set];
  assert.deepEqual(view.lines(records)[0].points.map((p) => p.tick), [50]);
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

test('a line breaks where its points\' era changes, though the commits are adjacent', () => {
  const points = [10, 20, 30, 40].map((position) => ({ position, tick: 1, era: position < 30 ? undefined : 'settled' }));
  const runs = view.segments(points, [10, 20, 30, 40]);
  assert.deepEqual(runs.map((run) => run.map((p) => p.position)), [[10, 20], [30, 40]]);
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

// Step 9 (Decision 172): the chart by the reference, the phase and CPU toggles.

function withReference(item, ms, cpu) {
  item.record.reference.results = ms === null ? {} : { tick: measure(ms) };
  if (cpu !== undefined) item.record.machine = { cpu };
  return item;
}

test('a value in × the reference is its ms over the reference tick of the same record', () => {
  const item = withReference(record('a'.repeat(40), 10, 'breach_tick_phases',
    { tick: measure(43), 'tick/scrum': measure(10) }), 99.9);
  const p = view.lines([item])[0].points[0];
  assert.equal(view.tickIn(p, 'reference').toFixed(3), '0.430');
  assert.equal(view.tickIn(p, 'ms'), 43);
  const stack = view.stack(p, ['tick/scrum'], {}, 'reference');
  assert.equal(stack[0].group, 'tick/scrum');
  assert.equal(stack[0].value.toFixed(3), '0.100');
});

test('in % of tick, hiding a phase leaves the other shares as they were', () => {
  const p = { tick: 100, referenceTick: null, results: {
    tick: measure(100), 'tick/a': measure(10), 'tick/b': measure(20), 'tick/c': measure(70) } };
  const all = view.stack(p, ['tick/a', 'tick/b', 'tick/c'], {}, 'percent');
  assert.deepEqual(all.map((s) => [s.group, s.value]), [['tick/a', 10], ['tick/b', 20], ['tick/c', 70]]);
  const hidden = view.stack(p, ['tick/a', 'tick/b', 'tick/c'], { 'tick/b': true }, 'percent');
  assert.deepEqual(hidden.map((s) => [s.group, s.value]), [['tick/a', 10], ['tick/c', 70]]);
});

test('"other" is the tick less the named phases, and can be hidden like them', () => {
  const p = { tick: 100, referenceTick: 50, results: { tick: measure(100), 'tick/a': measure(30) } };
  assert.deepEqual(view.stack(p, ['tick/a'], {}, 'ms').map((s) => [s.group, s.value]),
    [['tick/a', 30], ['other', 70]]);
  assert.deepEqual(view.stack(p, ['tick/a'], { other: true }, 'reference').map((s) => [s.group, s.value]),
    [['tick/a', 0.6]]);
});

test('a hidden CPU\'s records leave the lines, and the whole-tick line joins the rest', () => {
  const records = [
    withReference(record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) }), 90, 'EPYC'),
    withReference(record('b'.repeat(40), 20, 'breach_tick_phases', { tick: measure(60) }), 99, 'XEON'),
    withReference(record('c'.repeat(40), 30, 'breach_tick_phases', { tick: measure(48) }), 90, 'EPYC'),
    record('d'.repeat(40), 40, 'breach_tick_phases', { tick: measure(47) }),
  ];
  const all = view.lines(records);
  assert.deepEqual(view.cpus(all), ['EPYC', 'XEON', 'unknown']);
  const kept = view.withoutCpus(all, { XEON: true });
  const points = kept[0].points;
  assert.deepEqual(points.map((p) => p.position), [10, 30, 40]);
  const positions = view.positions(kept);
  assert.deepEqual(view.segments(points, positions).map((run) => run.map((p) => p.position)), [[10, 30, 40]]);
  assert.equal(all[0].points.length, 4);
});

test('a record with no reference is left out in × the reference, counted, and kept in ms', () => {
  const records = [
    withReference(record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) }), 100),
    withReference(record('b'.repeat(40), 20, 'breach_tick_phases', { tick: measure(60) }), null),
  ];
  const all = view.lines(records);
  const ratio = view.inUnit(all, 'reference');
  assert.deepEqual(ratio.lines[0].points.map((p) => p.position), [10]);
  assert.equal(ratio.lines[0].points[0].value, 0.5);
  assert.equal(ratio.withoutReference, 1);
  const ms = view.inUnit(all, 'ms');
  assert.deepEqual(ms.lines[0].points.map((p) => p.value), [50, 60]);
  assert.equal(ms.withoutReference, 0);
});

test('the compare shows the phase chart\'s unit, each side over its own record', () => {
  const a = { tick: 100, referenceTick: 200, results: { tick: measure(100), 'tick/a': measure(40) } };
  const b = { tick: 80, referenceTick: 100, results: { tick: measure(80), 'tick/a': measure(40) } };
  const rows = view.compareIn(a, b, 'reference');
  assert.deepEqual(rows.map((row) => [row.path, row.a, row.b]), [['tick', 0.5, 0.8], ['tick/a', 0.2, 0.4]]);
  const shares = view.compareIn(a, b, 'percent');
  assert.deepEqual(shares.map((row) => [row.path, row.a, row.b]), [['tick', 100, 100], ['tick/a', 40, 50]]);
});

test('a point names its CPU as the record does, "unknown" without one', () => {
  const named = withReference(record('a'.repeat(40), 10, 'breach_tick_phases', { tick: measure(50) }), 90,
    'AMD EPYC 9V74 80-Core Processor');
  const bare = record('b'.repeat(40), 20, 'breach_tick_phases', { tick: measure(50) });
  assert.deepEqual(view.lines([named, bare])[0].points.map((p) => p.cpu),
    ['AMD EPYC 9V74 80-Core Processor', 'unknown']);
});
