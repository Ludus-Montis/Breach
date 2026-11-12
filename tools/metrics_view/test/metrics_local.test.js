// The local section's logic, metrics_local.js (specs/48-local-bench-series.md, step 2,
// Decision 178): the owner's PC's records into curves, grouped by squad count or by squad
// size, a phase picker's choices, the runs over time, and CI's charts left as they were.
// Fixtures: the 20 records of the owner's 2026-11-11 grid, under metrics/local/.
// Run: node --test "tools/metrics_view/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const local = require('../metrics_local.js');
const view = require('../metrics_view.js');
const set = require('../metrics_25k.js');

const LOCAL = path.join(__dirname, '../../../metrics/local');
const GRID = fs.readdirSync(LOCAL).filter((name) => /^2026-11-11-\d+\.json$/.test(name)).map((name) => ({
  path: `metrics/local/${name}`,
  record: JSON.parse(fs.readFileSync(path.join(LOCAL, name), 'utf8')),
}));

// Least-squares slope of log y on log x.
function slope(points) {
  const xs = points.map((p) => Math.log(p.x));
  const ys = points.map((p) => Math.log(p.y));
  const mx = xs.reduce((a, b) => a + b, 0) / xs.length;
  const my = ys.reduce((a, b) => a + b, 0) / ys.length;
  let num = 0;
  let den = 0;
  xs.forEach((x, i) => { num += (x - mx) * (ys[i] - my); den += (x - mx) * (x - mx); });
  return num / den;
}

function ciRecord(position, args, tick) {
  const measure = (median) => ({ unit: 'ms', values: [median], median });
  return {
    path: `metrics/bench/2026-11-11-${position}-${args.length}.json`,
    record: {
      schema: 'record-v1',
      series: 'bench',
      commit: { sha: String(position).repeat(40).slice(0, 40), position, pr: 800 + position },
      source: { harness: 'breach_tick_phases', args },
      results: { tick: measure(tick), 'tick-p95': measure(tick * 1.1), 'tick-p99': measure(tick * 1.2) },
      reference: { sha: '8ca4227', results: { tick: measure(tick * 2) } },
      machine: { cpu: 'EPYC' },
      provenance: { run_id: 1, kind: 'fresh', date: '2026-11-11' },
    },
  };
}

test('the grid of 2026-11-11 is one run on a5f574d, on the owner\'s PC', () => {
  const runs = local.runs(GRID);
  assert.equal(runs.length, 1);
  assert.equal(runs[0].date, '2026-11-11');
  assert.equal(runs[0].sha.slice(0, 7), 'a5f574d');
  assert.equal(runs[0].records.length, 20);
  assert.match(runs[0].machine.cpu, /i7-12700K/);
});

test('drawn by squad count: 5 curves of 4 points, the 1-squad tick at 24,576 is 429.7 ms', () => {
  const curves = local.curves(local.runs(GRID)[0], { grouping: 'squads', phase: 'tick' });
  assert.deepEqual(curves.map((c) => c.key), [1, 2, 4, 8, 24]);
  curves.forEach((c) => assert.equal(c.points.length, 4));
  const one = curves[0].points.find((p) => p.x === 24576);
  assert.equal(one.y, 429.7);
  assert.ok(one.lo <= one.y && one.y <= one.hi, 'the bar spans its rounds');
});

test('units walk to places, per squad, by squad size: 4 totals on one line of slope about 2', () => {
  const curves = local.curves(local.runs(GRID)[0],
    { grouping: 'size', phase: 'tick/units-walk-to-places', perSquad: true });
  assert.deepEqual(curves.map((c) => c.key), [3072, 6144, 12288, 24576]);
  curves.forEach((c) => {
    assert.equal(c.points.length, 5);
    const s = slope(c.points);
    assert.ok(s > 1.7 && s < 2.2, `slope of ${c.key}: ${s}`);
  });
  const all = slope([].concat(...curves.map((c) => c.points)));
  assert.ok(all > 1.7 && all < 2.2, `all together: ${all}`);
});

test('the phase picker offers the tick first, and the default thin curves are the three costliest at the largest cell', () => {
  const run = local.runs(GRID)[0];
  const phases = local.phases(run);
  assert.equal(phases[0], 'tick');
  assert.ok(phases.indexOf('tick/units-walk-to-places') > 0);
  assert.deepEqual(local.costliest(run, 3).slice().sort(),
    ['tick/scrum/engage-prepare-stance', 'tick/scrum/seek', 'tick/units-walk-to-places']);
});

test('a later run is the default, with the one before it', () => {
  const later = GRID.map((item) => ({
    path: item.path.replace('2026-11-11', '2026-11-12'),
    record: Object.assign({}, item.record, {
      provenance: Object.assign({}, item.record.provenance, { date: '2026-11-12' }),
      commit: Object.assign({}, item.record.commit, { sha: 'b'.repeat(40), position: 300 }),
    }),
  }));
  const runs = local.runs(GRID.concat(later));
  assert.deepEqual(runs.map((r) => r.date), ['2026-11-12', '2026-11-11']);
  assert.equal(local.before(runs, runs[0].key).date, '2026-11-11');
  assert.equal(local.before(runs, runs[1].key), null);
});

test('CI\'s 4,096 and 25k charts draw the same points with the local records as without', () => {
  const ci = [ciRecord(10, ['side=4096', 'squads=4'], 20)]
    .concat(['side=1024 squads=1', 'side=4096 squads=4', 'side=8192 squads=8', 'side=24576 squads=24']
      .map((shape, i) => ciRecord(10, shape.split(' ').concat(['series=1']), 5 + i * 30)));
  const both = ci.concat(GRID);
  assert.deepEqual(view.lines(both), view.lines(ci));
  assert.deepEqual(set.chart(both), set.chart(ci));
  assert.deepEqual(set.onTrack(both), set.onTrack(ci));
});

// Step 5c: a cell's repeat records (harness `tick_phases-repeat`, args `at=`/`repeat=`, spec 48
// step 5a) as points at their ticks beside the cell's series, with Q47's caveat (#909).
function repeatOf(item, tick, median, quartiles) {
  const record = item.record;
  return {
    path: item.path.replace('.json', `-repeat-${tick}.json`),
    record: Object.assign({}, record, {
      source: Object.assign({}, record.source, {
        harness: 'tick_phases-repeat',
        args: record.source.args.filter((a) => !a.startsWith('fight=')).concat([`at=${tick}`, 'repeat=50']),
      }),
      results: { tick: Object.assign({ unit: 'ms', values: [median - 1, median + 1], median },
        quartiles ? { quartiles } : {}) },
    }),
  };
}

test('a cell\'s repeats are points at their ticks beside its series, not cells of it', () => {
  const cell = GRID.find((item) => item.record.source.args.includes('side=24576')
    && item.record.source.args.includes('squads=1'));
  const run = local.runs(GRID.concat([repeatOf(cell, 812, 420.5), repeatOf(cell, 340, 401)]))[0];
  assert.equal(run.records.length, 20, 'the series keeps its 20 cells');
  const curves = local.curves(run, { grouping: 'squads', phase: 'tick' });
  assert.equal(curves[0].points.length, 4);
  const points = local.repeats(run, { grouping: 'squads', phase: 'tick' });
  assert.deepEqual(points.map((p) => [p.key, p.x, p.tick, p.y]), [[1, 24576, 340, 401], [1, 24576, 812, 420.5]]);
  assert.deepEqual([points[1].lo, points[1].hi, points[1].n], [419.5, 421.5, 50]);
  assert.equal(local.repeats(run, { grouping: 'size', phase: 'tick', perSquad: true })[0].x, 24576);
  assert.deepEqual(local.repeats(local.runs(GRID)[0], {}), []);
  assert.match(local.REPEAT_CAVEAT, /under-report/);
  assert.match(local.REPEAT_CAVEAT, /whole tick/);
});

// Step 5e: a repeat's bar spans its rounds' quartiles where the record keeps them (step 5d),
// and its rounds' range where it doesn't. `spread` says which, for the hover text.
test('a repeat with quartiles spans the lowest q1 to the highest q3 of its rounds', () => {
  const cell = GRID.find((item) => item.record.source.args.includes('side=24576')
    && item.record.source.args.includes('squads=1'));
  const run = local.runs(GRID.concat([repeatOf(cell, 812, 420.5, [[417, 423.5], [416.25, 425]])]))[0];
  const [point] = local.repeats(run, { grouping: 'squads', phase: 'tick' });
  assert.deepEqual([point.lo, point.hi, point.spread], [416.25, 425, 'quartiles']);
  const [halved] = local.repeats(run, { grouping: 'squads', phase: 'tick', perSquad: true });
  assert.deepEqual([halved.lo, halved.hi], [416.25, 425]);
});

test('a repeat without quartiles spans its rounds\' range, as before', () => {
  const cell = GRID.find((item) => item.record.source.args.includes('side=24576')
    && item.record.source.args.includes('squads=1'));
  const run = local.runs(GRID.concat([repeatOf(cell, 812, 420.5)]))[0];
  const [point] = local.repeats(run, { grouping: 'squads', phase: 'tick' });
  assert.deepEqual([point.lo, point.hi, point.spread], [419.5, 421.5, 'rounds']);
});
