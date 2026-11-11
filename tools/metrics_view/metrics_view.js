// The metrics page's logic (specs/43-metrics-history.md, step 7): the records the page
// loads (build_index.py's bundle, [{path, record}]) into lines per harness along main's
// first-parent line, the bridge's factor between the eras, and two commits compared at the
// deepest phase level both have. No DOM here; metrics_page.js draws.
(function (root) {
  'use strict';

  var REPO = 'https://github.com/Ludus-Montis/Breach';
  var GDSCRIPT = 'tick_phases.gd';
  var CARGO = 'breach_tick_phases';

  function number(value) {
    return typeof value === 'number' && isFinite(value) ? value : null;
  }

  function isRoot(path) {
    return path.indexOf('/') < 0;
  }

  function ancestors(path) {
    var parts = path.split('/');
    var out = [];
    for (var i = 1; i < parts.length; i++) out.push(parts.slice(0, i).join('/'));
    return out;
  }

  function round(value) {
    return Math.round(value * 10000) / 10000;
  }

  // A path's ms: its own median, else the sum of its topmost measured descendants'; null
  // when it has none (a gap is no value, never 0).
  function valueAt(results, path) {
    var own = results[path];
    if (own) return number(own.median);
    var total = null;
    Object.keys(results).forEach(function (other) {
      if (other.indexOf(path + '/') !== 0) return;
      var value = number(results[other].median);
      if (value === null) return;
      var covered = ancestors(other).some(function (up) {
        return up.length > path.length && number((results[up] || {}).median) !== null;
      });
      if (!covered) total = (total || 0) + value;
    });
    return total === null ? null : round(total);
  }

  // A path one side has that the other only has more coarsely: the other's nearest
  // explicit ancestor below the root (the whole tick, whose children are always compared).
  function coarserIn(results, path) {
    var found = null;
    ancestors(path).forEach(function (up) {
      if (!isRoot(up) && results[up]) found = up;
    });
    return found;
  }

  // Rows comparing two records' results at the deepest level both have: {path, a, b, finer},
  // `finer` naming the paths below that only one side measured.
  function compare(a, b) {
    var rows = {};
    var finer = {};
    function visit(path, mine, theirs) {
      var coarse = theirs[path] ? null : coarserIn(theirs, path);
      if (coarse) {
        (finer[coarse] = finer[coarse] || {})[path] = true;
        return;
      }
      rows[path] = true;
    }
    Object.keys(a).forEach(function (path) { visit(path, a, b); });
    Object.keys(b).forEach(function (path) { visit(path, b, a); });
    return Object.keys(rows).sort(function (x, y) {
      if (isRoot(x) !== isRoot(y)) return isRoot(x) ? -1 : 1;
      return x < y ? -1 : x > y ? 1 : 0;
    }).map(function (path) {
      return { path: path, a: valueAt(a, path), b: valueAt(b, path),
        finer: Object.keys(finer[path] || {}).sort() };
    });
  }

  function benchRecords(bundle) {
    return bundle.filter(function (item) {
      return item.record && item.record.series === 'bench' && item.record.source &&
        item.record.results;
    });
  }

  function kind(item) {
    return (item.record.provenance || {}).kind;
  }

  function point(item, position) {
    var record = item.record;
    var commit = record.commit || {};
    var ratio = ((record.reference || {}).ratio || {}).tick;
    return {
      position: position,
      sha: String(commit.sha || ''),
      title: typeof commit.title === 'string' ? commit.title : '',
      tick: valueAt(record.results, 'tick'),
      ratio: number(ratio),
      referenceTick: valueAt(((record.reference || {}).results) || {}, 'tick'),
      reference: typeof (record.reference || {}).commit === 'string' ? record.reference.commit : '',
      pr: typeof commit.pr === 'number' ? REPO + '/pull/' + commit.pr : null,
      recordUrl: REPO + '/blob/main/' + item.path,
      results: record.results,
      runId: (record.provenance || {}).run_id || 0,
      harvested: kind(item) === 'harvested',
      date: (record.provenance || {}).date || '',
    };
  }

  // Fresh records give each harness's line, one point per commit (its latest run); a
  // harvested record sits on its head's or its base's place on that line, apart.
  function lines(bundle) {
    var byHarness = {};
    var places = [];
    benchRecords(bundle).forEach(function (item) {
      var position = number((item.record.commit || {}).position);
      if (kind(item) !== 'harvested' && position !== null) {
        places.push({ sha: String(item.record.commit.sha), position: position });
        var harness = item.record.source.harness;
        var line = byHarness[harness] = byHarness[harness] || { harness: harness, byCommit: {}, harvested: [] };
        var next = point(item, position);
        var kept = line.byCommit[next.sha];
        if (!kept || next.runId > kept.runId) line.byCommit[next.sha] = next;
      }
    });
    function placeOf(sha) {
      if (typeof sha !== 'string' || sha.length < 7) return null;
      var found = places.find(function (place) { return place.sha.indexOf(sha) === 0; });
      return found ? found.position : null;
    }
    benchRecords(bundle).forEach(function (item) {
      if (kind(item) !== 'harvested') return;
      var commit = item.record.commit || {};
      var position = placeOf(commit.sha);
      if (position === null) position = placeOf(commit.base);
      if (position === null) return;
      var harness = item.record.source.harness;
      var line = byHarness[harness] = byHarness[harness] || { harness: harness, byCommit: {}, harvested: [] };
      line.harvested.push(point(item, position));
    });
    return Object.keys(byHarness).map(function (harness) {
      var line = byHarness[harness];
      var points = Object.keys(line.byCommit).map(function (sha) { return line.byCommit[sha]; });
      points.sort(function (x, y) { return x.position - y.position; });
      line.harvested.sort(function (x, y) { return x.position - y.position; });
      return { harness: harness, points: points, harvested: line.harvested };
    }).sort(function (x, y) {
      var first = function (line) {
        var all = line.points.concat(line.harvested);
        return all.length ? Math.min.apply(null, all.map(function (p) { return p.position; })) : Infinity;
      };
      return first(x) - first(y) || (x.harness < y.harness ? -1 : 1);
    });
  }

  // The commit timed with both tick_phases harnesses: their ticks and the factor between.
  function bridge(bundle) {
    var all = lines(bundle);
    var gd = all.find(function (line) { return line.harness === GDSCRIPT; });
    var cargo = all.find(function (line) { return line.harness === CARGO; });
    if (!gd || !cargo) return null;
    var found = null;
    gd.points.forEach(function (left) {
      var right = cargo.points.find(function (p) { return p.sha === left.sha; });
      if (right && left.tick && right.tick && (!found || left.position > found.position)) {
        found = { sha: left.sha, position: left.position, gdscript: left.tick,
          cargo: right.tick, factor: left.tick / right.tick };
      }
    });
    return found;
  }

  // A line's runs of points to join: it breaks at a point with no tick (the harness ran and
  // gave nothing) and where a commit on main's line (`positions`, every commit with a fresh
  // record) lies between two points, so a missing commit is never bridged by the line.
  function segments(points, positions) {
    var order = positions.slice().sort(function (a, b) { return a - b; });
    var runs = [];
    var run = [];
    points.forEach(function (p) {
      var last = run[run.length - 1];
      var adjacent = last && order.indexOf(p.position) === order.indexOf(last.position) + 1;
      if (p.tick === null || !adjacent) {
        if (run.length) runs.push(run);
        run = [];
      }
      if (p.tick !== null) run.push(p);
    });
    if (run.length) runs.push(run);
    return runs;
  }

  // The lines as drawn: harvested points only when asked for (they come from old logs, with
  // other args and no shared reference, so they're hidden by default). Leaves `all` as it is.
  function shown(all, withHarvested) {
    return all.map(function (line) {
      return { harness: line.harness, points: line.points,
        harvested: withHarvested ? line.harvested : [], hiddenHarvested: withHarvested ? 0 : line.harvested.length };
    });
  }

  var api = { shown: shown, segments: segments, valueAt: valueAt, compare: compare, lines: lines, bridge: bridge, REPO: REPO };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.BreachMetricsView = api;
})(this);
