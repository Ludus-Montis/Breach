// The 25k chart's logic (specs/43-metrics-history.md, step 8c, Decision 165): the 25k set's
// fresh records (`series=1`, step 8a) into one line per family along main's first-parent
// line, each check point with its p95 and p99, and the projection of the non-negative fit of
// b·x + c·x² on that commit's grid, its miss against the measured 24,576 and its quadratic
// share. The fit is computed here, never stored, as bench_set.py does for the data PR's
// summary; both are tested against ci/metrics/fixtures/bench_fit.json. Step 8d adds the
// 4,096 chart's "on track" lines, × the bridge, the CPU marks and the harvested 25k runs. No
// DOM here; metrics_25k_page.js draws.
(function (root) {
  'use strict';

  var view = typeof module !== 'undefined' && module.exports ? require('./metrics_view.js') : root.BreachMetricsView;

  var SERIES_ARG = 'series=1';
  // The settled series' mark (spec 47 step 2b): not part of a shape; a new era of the line.
  var SETTLED_ARG = 'series=settled';
  var TOLERANCE = 0.10;
  var CHECK = 24576;
  var FOUR_LANES = 4096;
  var GOALS = { target: 50, interim: 100 };
  var MARKS = ['circle', 'square', 'triangle', 'diamond', 'cross', 'star'];
  var BRIDGE_KEY = { tick: 'tick', p95: 'tick-p95', p99: 'tick-p99', predicted: 'tick' };
  var FAMILIES = [
    { name: 'lanes', label: '24 lanes of 1,024',
      grid: ['side=1024 squads=1', 'side=4096 squads=4', 'side=8192 squads=8'], check: 'side=24576 squads=24' },
    { name: 'one squad', label: 'one squad of 24,576',
      grid: ['side=2048 squads=1', 'side=4096 squads=1', 'side=8192 squads=1', 'side=12288 squads=1'],
      check: 'side=24576 squads=1' },
  ];

  function number(value) {
    return typeof value === 'number' && isFinite(value) ? value : null;
  }

  function median(results, key) {
    return number(((results || {})[key] || {}).median);
  }

  function side(shape) {
    var found = /(?:^| )side=(\d+)/.exec(shape);
    return found ? Number(found[1]) : null;
  }

  function predict(b, c, x) {
    return b * x + c * x * x;
  }

  // {b, c} of b·x + c·x², each at least 0, least squares over [[x, t]]: the unconstrained
  // solution when both are non-negative, else the better of each term alone (or none).
  function fit(points) {
    var sum = function (f) { return points.reduce(function (total, p) { return total + f(p[0], p[1]); }, 0); };
    var sxx = sum(function (x) { return x * x; });
    var sx3 = sum(function (x) { return x * x * x; });
    var sx4 = sum(function (x) { return x * x * x * x; });
    var sxt = sum(function (x, t) { return x * t; });
    var sx2t = sum(function (x, t) { return x * x * t; });
    var candidates = [{ b: 0, c: 0 }];
    if (sxx) candidates.push({ b: Math.max(sxt / sxx, 0), c: 0 });
    if (sx4) candidates.push({ b: 0, c: Math.max(sx2t / sx4, 0) });
    var det = sxx * sx4 - sx3 * sx3;
    if (det) {
      var b = (sxt * sx4 - sx2t * sx3) / det;
      var c = (sx2t * sxx - sxt * sx3) / det;
      if (b >= 0 && c >= 0) candidates.push({ b: b, c: c });
    }
    var squares = function (bc) {
      return sum(function (x, t) { var r = predict(bc.b, bc.c, x) - t; return r * r; });
    };
    return candidates.reduce(function (best, next) { return squares(next) < squares(best) ? next : best; });
  }

  function fresh(item) {
    var record = item.record || {};
    return record.series === 'bench' && (record.provenance || {}).kind === 'fresh' &&
      number((record.commit || {}).position) !== null && record.source && record.results;
  }

  // The set's shape a record times ("side=… squads=…"), or null.
  function shapeOf(record) {
    var args = (record.source || {}).args;
    if (!Array.isArray(args) || args.indexOf(SERIES_ARG) < 0) return null;
    return args.filter(function (arg) { return arg !== SERIES_ARG && arg !== SETTLED_ARG; }).join(' ');
  }

  // {position: {shape: record}}, a commit's latest run of each shape winning.
  function byCommit(bundle) {
    var found = {};
    bundle.filter(fresh).forEach(function (item) {
      var shape = shapeOf(item.record);
      if (!shape) return;
      var at = found[item.record.commit.position] = found[item.record.commit.position] || {};
      var run = function (r) { return (r.provenance || {}).run_id || 0; };
      if (!at[shape] || run(item.record) >= run(at[shape])) at[shape] = item.record;
    });
    return found;
  }

  // A family's grid at one commit, [[side, mean tick]], smallest first.
  function gridOf(family, shapes) {
    return family.grid.filter(function (shape) {
      return shapes[shape] && median(shapes[shape].results, 'tick') !== null;
    }).map(function (shape) { return [side(shape), median(shapes[shape].results, 'tick')]; })
      .sort(function (a, b) { return a[0] - b[0]; });
  }

  // A family's point at one commit: the measured check, and the projection from its grid
  // where the grid has two sizes or more.
  function point(family, shapes) {
    var check = shapes[family.check];
    var tick = check ? median(check.results, 'tick') : null;
    if (tick === null) return null;
    var grid = gridOf(family, shapes);
    var fitted = grid.length >= 2 ? fit(grid) : null;
    var predicted = fitted ? predict(fitted.b, fitted.c, CHECK) : null;
    var miss = predicted === null ? null : (predicted - tick) / tick;
    var commit = check.commit;
    return {
      position: commit.position,
      sha: String(commit.sha || ''),
      title: typeof commit.title === 'string' ? commit.title : '',
      cpu: cpuOf(check),
      tick: tick,
      p95: median(check.results, 'tick-p95'),
      p99: median(check.results, 'tick-p99'),
      b: fitted ? fitted.b : null,
      c: fitted ? fitted.c : null,
      predicted: predicted,
      miss: miss,
      ringed: miss !== null && Math.abs(miss) > TOLERANCE,
      quadShare: predicted ? fitted.c * CHECK * CHECK / predicted : null,
      grid: grid,
      bridge: bridgeOf(check),
      runId: (check.provenance || {}).run_id || 0,
      era: (check.source.args || []).indexOf(SETTLED_ARG) >= 0 ? 'settled' : undefined,
    };
  }

  // The bridge's measures timed beside a check in the same job, or null.
  function bridgeOf(check) {
    var results = (check.reference || {}).results;
    if (!results) return null;
    var found = {};
    Object.keys(BRIDGE_KEY).forEach(function (key) { found[key] = median(results, BRIDGE_KEY[key]); });
    return found;
  }

  // A point's `key` (tick, p95, p99 or predicted) in a unit: 'ms', or 'bridge', over the
  // bridge's same measure (an estimate over its mean); null where there is none.
  function valueIn(p, key, unit) {
    var value = p[key];
    if (value === null || value === undefined) return null;
    if (unit !== 'bridge') return value;
    var bridge = (p.bridge || {})[key];
    return bridge ? value / bridge : null;
  }

  function cpuOf(record) {
    return typeof (record.machine || {}).cpu === 'string' ? record.machine.cpu : view.UNKNOWN;
  }

  // The 4,096 chart's "on track" lines (step 8d): per commit with a lanes grid, the 4-lane
  // tick at which the lanes fit predicts each of Decision 159's goals at 24 lanes, beside the
  // set's measured 4-lane tick. A commit with no 25k set has none, so the lines break there.
  function onTrack(bundle) {
    var lanes = FAMILIES[0];
    var commits = byCommit(bundle);
    return Object.keys(commits).map(Number).sort(function (a, b) { return a - b; }).map(function (position) {
      var shapes = commits[position];
      var grid = gridOf(lanes, shapes);
      if (grid.length < 2) return null;
      var fitted = fit(grid);
      var scale = predict(fitted.b, fitted.c, FOUR_LANES) / predict(fitted.b, fitted.c, CHECK);
      if (!isFinite(scale)) return null;
      var four = shapes[lanes.grid[1]];
      var any = shapes[Object.keys(shapes)[0]];
      return { position: position, sha: String(any.commit.sha || ''), cpu: cpuOf(any),
        target: GOALS.target * scale, interim: GOALS.interim * scale,
        measured: four ? median(four.results, 'tick') : null };
    }).filter(function (p) { return p; });
  }

  // The harvested 25k runs (step 8d): read from old logs with other args, placed at their
  // commit's or base's place on main's line, hollow and never fitted. All are 24 lanes.
  function harvested(bundle) {
    var placeOf = view.placer(bundle);
    return bundle.filter(function (item) {
      var record = item.record || {};
      return record.series === 'bench' && (record.provenance || {}).kind === 'harvested' &&
        Array.isArray((record.source || {}).args) && view.harvested25k(record.source.args);
    }).map(function (item) {
      var record = item.record;
      return { position: placeOf(record.commit), sha: String((record.commit || {}).sha || ''),
        tick: median(record.results, 'tick'), cpu: cpuOf(record), family: FAMILIES[0].name,
        args: record.source.args.join(' '), date: (record.provenance || {}).date || '', harvested: true };
    }).filter(function (p) { return p.position !== null && p.tick !== null; })
      .sort(function (a, b) { return a.position - b.position; });
  }

  // The chart: `positions`, every commit on main's line with a fresh bench record from the
  // first 25k set on (so a later commit with no 25k set breaks the lines), and each family's
  // points, oldest first. With `options.harvested`, the harvested 25k runs too, and the
  // columns from the first of them.
  function chart(bundle, options) {
    var seen = {};
    bundle.filter(fresh).forEach(function (item) { seen[item.record.commit.position] = true; });
    var commits = byCommit(bundle);
    var order = Object.keys(commits).map(Number).sort(function (a, b) { return a - b; });
    var old = (options || {}).harvested ? harvested(bundle) : [];
    var first = Math.min.apply(null, order.slice(0, 1).concat(old.map(function (p) { return p.position; })));
    return {
      harvested: old,
      positions: Object.keys(seen).map(Number).filter(function (position) { return position >= first; })
        .sort(function (a, b) { return a - b; }),
      families: FAMILIES.map(function (family) {
        return { name: family.name, label: family.label, check: family.check,
          points: order.map(function (position) { return point(family, commits[position]); })
            .filter(function (p) { return p; }) };
      }),
    };
  }

  // The chart without the points of the CPUs in `hidden` ({cpu: true}). Their records still
  // count in every fit and miss (Decision 172).
  function withoutCpus(drawn, hidden) {
    var kept = function (p) { return !hidden[p.cpu]; };
    return { positions: drawn.positions, harvested: drawn.harvested.filter(kept),
      families: drawn.families.map(function (family) {
        return Object.assign({}, family, { points: family.points.filter(kept) });
      }) };
  }

  // Every CPU the chart's points name, sorted.
  function cpus(drawn) {
    var seen = {};
    drawn.families.forEach(function (family) { family.points.forEach(function (p) { seen[p.cpu] = true; }); });
    drawn.harvested.forEach(function (p) { seen[p.cpu] = true; });
    return Object.keys(seen).sort();
  }

  // A CPU's mark among `all` CPUs: by its place in their sorted order, so every caller agrees.
  function markOf(all, cpu) {
    var sorted = all.slice().sort();
    var at = sorted.indexOf(cpu);
    return MARKS[(at < 0 ? sorted.length : at) % MARKS.length];
  }

  var api = { fit: fit, predict: predict, chart: chart, shapeOf: shapeOf, FAMILIES: FAMILIES,
    TOLERANCE: TOLERANCE, CHECK: CHECK, GOALS: GOALS, onTrack: onTrack, valueIn: valueIn,
    withoutCpus: withoutCpus, cpus: cpus, markOf: markOf, MARKS: MARKS };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.BreachMetrics25k = api;
})(this);
