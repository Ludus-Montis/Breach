// The 25k chart's logic, metrics_25k.js (specs/43-metrics-history.md, step 8c, Decision 165):
// the non-negative fit of b·x + c·x² on a commit's grid, its miss at 24,576 a side, and the
// measured lines, broken at a commit on main's line with no 25k set. The fit's cases are the
// fixtures bench_set.py is tested against.
// Run: node --test "tools/metrics_view/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const set = require('../metrics_25k.js');
const view = require('../metrics_view.js');

const FIT = JSON.parse(fs.readFileSync(
  path.join(__dirname, '../../../ci/metrics/fixtures/bench_fit.json'), 'utf8'));

function measure(median) {
  return { unit: 'ms', values: [median], median };
}

function record(position, args, results, extra = {}) {
  const sha = String(position).repeat(40).slice(0, 40);
  return {
    path: `metrics/bench/2026-11-11-${position}-${args.length}.json`,
    record: {
      schema: 'record-v1',
      series: 'bench',
      commit: { sha, position, pr: 800 + position },
      source: { harness: 'breach_tick_phases', args },
      results,
      machine: { cpu: extra.cpu || 'EPYC' },
      provenance: { run_id: extra.run_id || 1, kind: 'fresh', date: '2026-11-11' },
    },
  };
}

function shape(position, args, tick, extra = {}) {
  return record(position, args.split(' ').concat(['series=1']), {
    tick: measure(tick), 'tick-p95': measure(extra.p95 || tick * 1.1), 'tick-p99': measure(extra.p99 || tick * 1.2),
  }, extra);
}

const at = (b, c, x) => b * x + c * x * x;

// A commit's lanes set on b·x + c·x², with its check measured at `check` ms.
function lanes(position, b, c, check) {
  return [
    shape(position, 'side=1024 squads=1', at(b, c, 1024)),
    shape(position, 'side=4096 squads=4', at(b, c, 4096)),
    shape(position, 'side=8192 squads=8', at(b, c, 8192)),
    shape(position, 'side=24576 squads=24', check),
  ];
}

test('grid records exactly on b·x + c·x² give b, c and the predicted 24,576 within rounding', () => {
  const fitted = set.fit(FIT.exact.points);
  assert.ok(Math.abs(fitted.b - FIT.exact.b) < 1e-9);
  assert.ok(Math.abs(fitted.c - FIT.exact.c) < 1e-14);
  assert.ok(Math.abs(set.predict(fitted.b, fitted.c, FIT.exact.at) - FIT.exact.predicted) < 1e-6);
});

test('grid records that would need a negative c fit with c = 0', () => {
  const fitted = set.fit(FIT.negative_c.points);
  assert.equal(fitted.c, 0);
  assert.ok(Math.abs(fitted.b - FIT.negative_c.b) < 1e-12);
});

test('a check point carries its p95 and p99, the projection, the miss and the quadratic share', () => {
  const b = 0.001;
  const c = 1e-8;
  const predicted = at(b, c, 24576);
  const chart = set.chart(lanes(1, b, c, predicted / 1.12));
  const point = chart.families.find((family) => family.name === 'lanes').points[0];
  assert.ok(Math.abs(point.predicted - predicted) < 1e-6);
  assert.ok(Math.abs(point.miss - 0.12) < 1e-9);
  assert.equal(point.ringed, true);
  assert.ok(Math.abs(point.quadShare - c * 24576 * 24576 / predicted) < 1e-9);
  assert.ok(Math.abs(point.p95 - predicted / 1.12 * 1.1) < 1e-9);
  assert.ok(Math.abs(point.p99 - predicted / 1.12 * 1.2) < 1e-9);
  assert.equal(point.cpu, 'EPYC');
});

test('a miss within 10% is not ringed, and a commit without a grid has no projection', () => {
  const chart = set.chart(lanes(1, 0.001, 1e-8, at(0.001, 1e-8, 24576) * 1.05)
    .concat([shape(2, 'side=24576 squads=24', 40)]));
  const points = chart.families.find((family) => family.name === 'lanes').points;
  assert.equal(points[0].ringed, false);
  assert.deepEqual([points[1].predicted, points[1].miss, points[1].ringed], [null, null, false]);
});

test('a commit on main\'s line with no 25k set breaks the 25k lines there, from the first set on', () => {
  const plain = (position) => record(position, ['side=4096', 'squads=4'], { tick: measure(40) });
  const chart = set.chart([plain(0)].concat(lanes(1, 0.001, 1e-8, 30), [plain(2)], lanes(3, 0.001, 1e-8, 31)));
  assert.deepEqual(chart.positions, [1, 2, 3]);
  const points = chart.families.find((family) => family.name === 'lanes').points;
  const runs = view.segments(points, chart.positions, 'p95');
  assert.deepEqual(runs.map((run) => run.map((p) => p.position)), [[1], [3]]);
  const squad = chart.families.find((family) => family.name === 'one squad');
  assert.deepEqual(squad.points, []);
});

test('a record marked series=settled has the shape of one without, and the line breaks between them', () => {
  const settled = (position) => lanes(position, 0.001, 1e-8, 30).map((item) => {
    item.record.source.args.push('series=settled');
    return item;
  });
  const chart = set.chart(lanes(1, 0.001, 1e-8, 30).concat(lanes(2, 0.001, 1e-8, 31), settled(3), settled(4)));
  const points = chart.families.find((family) => family.name === 'lanes').points;
  assert.deepEqual(points.map((p) => p.position), [1, 2, 3, 4]);
  assert.ok(points.every((p) => p.predicted !== null), 'a settled commit keeps its grid');
  const runs = view.segments(points, chart.positions, 'tick');
  assert.deepEqual(runs.map((run) => run.map((p) => p.position)), [[1, 2], [3, 4]]);
});

test('a commit\'s latest run of a shape is the one drawn', () => {
  const chart = set.chart([shape(1, 'side=24576 squads=24', 50, { run_id: 1 }),
    shape(1, 'side=24576 squads=24', 40, { run_id: 2 })]);
  const points = chart.families.find((family) => family.name === 'lanes').points;
  assert.deepEqual(points.map((p) => p.tick), [40]);
});

// Step 8d: the 4,096 chart's "on track" lines, × the bridge, the CPU marks, the detail
// panel's grid and the harvested 25k runs.

function grid(position, b, c) {
  return lanes(position, b, c, at(b, c, 24576)).slice(0, 3);
}

test('given lanes fit b and c, the "on track" 4-lane tick predicts 50 ms at 24 lanes', () => {
  const b = 0.001;
  const c = 1e-8;
  const track = set.onTrack(grid(1, b, c));
  assert.deepEqual(track.map((p) => p.position), [1]);
  const target = track[0].target;
  assert.ok(Math.abs(target * set.predict(b, c, 24576) / set.predict(b, c, 4096) - 50) < 1e-9);
  assert.ok(Math.abs(track[0].interim - 2 * target) < 1e-9);
  assert.ok(Math.abs(track[0].measured - at(b, c, 4096)) < 1e-9);
});

test('a commit with no 25k set breaks the 4,096 chart\'s "on track" lines there', () => {
  const plain = (position) => record(position, ['side=4096', 'squads=4'], { tick: measure(40) });
  const bundle = [plain(1), plain(2), plain(3)].concat(grid(1, 0.001, 1e-8), grid(3, 0.001, 1e-8));
  const fresh = view.positions(view.lines(bundle));
  assert.deepEqual(fresh, [1, 2, 3]);
  const runs = view.segments(set.onTrack(bundle), fresh, 'target');
  assert.deepEqual(runs.map((run) => run.map((p) => p.position)), [[1], [3]]);
});

test('× the bridge divides each measure by the bridge\'s same measure, and an estimate by its mean', () => {
  const check = shape(1, 'side=24576 squads=24', 40, { p95: 44, p99: 48 });
  check.record.reference = { commit: '8ca4227', results: {
    tick: measure(80), 'tick-p95': measure(88), 'tick-p99': measure(96) } };
  const chart = set.chart(grid(1, 0.001, 1e-8).concat([check]));
  const p = chart.families[0].points[0];
  assert.deepEqual([set.valueIn(p, 'p95', 'bridge'), set.valueIn(p, 'p99', 'bridge'),
    set.valueIn(p, 'tick', 'bridge')], [0.5, 0.5, 0.5]);
  assert.ok(Math.abs(set.valueIn(p, 'predicted', 'bridge') - p.predicted / 80) < 1e-12);
  assert.equal(set.valueIn(p, 'p95', 'ms'), 44);
  const bare = set.chart([shape(2, 'side=24576 squads=24', 40)]).families[0].points[0];
  assert.equal(set.valueIn(bare, 'p95', 'bridge'), null);
});

test('a hidden CPU leaves the chart, and the fit and miss still use every record of the commit', () => {
  const b = 0.001;
  const c = 1e-8;
  const bundle = lanes(1, b, c, at(b, c, 24576) / 1.12).map((item, i) => {
    if (i === 0) item.record.machine.cpu = 'Xeon';
    return item;
  });
  const all = set.chart(bundle);
  const shown = set.withoutCpus(all, { Xeon: true });
  assert.ok(Math.abs(shown.families[0].points[0].miss - 0.12) < 1e-9);
  assert.deepEqual(set.withoutCpus(all, { EPYC: true }).families[0].points, []);
  assert.deepEqual(set.cpus(all), ['EPYC']);
});

test('each CPU has its own mark, the same in every place it is asked for', () => {
  const cpus = ['AMD EPYC 7763', 'Xeon 8370C', 'unknown'];
  const marks = cpus.map((cpu) => set.markOf(cpus, cpu));
  assert.equal(new Set(marks).size, 3);
  assert.equal(set.markOf(cpus.slice().reverse(), 'Xeon 8370C'), set.markOf(cpus, 'Xeon 8370C'));
});

test('a point carries its grid, so the detail panel can draw the fitted curve over size', () => {
  const b = 0.001;
  const c = 1e-8;
  const p = set.chart(lanes(1, b, c, 30)).families[0].points[0];
  assert.deepEqual(p.grid.map((g) => g[0]), [1024, 4096, 8192]);
  assert.ok(Math.abs(p.grid[2][1] - at(b, c, 8192)) < 1e-9);
});

test('harvested 25k runs leave the 4,096 line and come to the 25k chart only when asked for, unfitted', () => {
  const harvested = (sha, args, tick) => ({
    path: `metrics/bench/2026-11-05-${sha}.json`,
    record: {
      schema: 'record-v1', series: 'bench',
      commit: { sha, base: { na: 'not-applicable' }, position: { na: 'not-recorded' } },
      source: { harness: 'breach_tick_phases', args }, results: { tick: measure(tick) },
      machine: { na: 'not-recorded' }, provenance: { run_id: 5, kind: 'harvested', date: '2026-11-05' },
    },
  });
  const plain = record(1, ['side=4096', 'squads=4'], { tick: measure(40) });
  const sha = plain.record.commit.sha.slice(0, 7);
  const bundle = [plain, harvested(sha, ['preset=25k'], 900),
    harvested(sha, ['side=24576', 'squads=24', 'settle=10', 'fight=5'], 800),
    harvested(sha, ['side=4096', 'squads=4'], 41)].concat(lanes(2, 0.001, 1e-8, 30));
  const line = view.lines(bundle).find((l) => l.harness === 'breach_tick_phases');
  assert.deepEqual(line.harvested.map((p) => p.tick), [41]);
  assert.deepEqual(set.chart(bundle).harvested, []);
  assert.deepEqual(set.chart(bundle).positions, [2]);
  const shown = set.chart(bundle, { harvested: true });
  assert.deepEqual(shown.harvested.map((p) => [p.position, p.tick, p.cpu]).sort(), [[1, 800, 'unknown'], [1, 900, 'unknown']]);
  assert.deepEqual(shown.positions, [1, 2]);
  assert.ok(shown.harvested.every((p) => p.predicted === undefined));
});
