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

test('a commit\'s latest run of a shape is the one drawn', () => {
  const chart = set.chart([shape(1, 'side=24576 squads=24', 50, { run_id: 1 }),
    shape(1, 'side=24576 squads=24', 40, { run_id: 2 })]);
  const points = chart.families.find((family) => family.name === 'lanes').points;
  assert.deepEqual(points.map((p) => p.tick), [40]);
});
