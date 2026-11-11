// The metrics page's logic (specs/43-metrics-history.md, step 7): the records the page
// loads (build_index.py's bundle, [{path, record}]) into lines per harness along main's
// first-parent line, the bridge's factor between the eras, and two commits compared at the
// deepest phase level both have. Step 9 (Decision 172): the units (ms, × the reference, % of
// tick), the phase stack with phases hidden, and the CPU filter. No DOM here; metrics_page.js
// draws.
(function (root) {
  'use strict';

  var REPO = 'https://github.com/Ludus-Montis/Breach';
  var GDSCRIPT = 'tick_phases.gd';
  var CARGO = 'breach_tick_phases';
  var UNKNOWN = 'unknown';

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

  // The 25k set's fresh records (`series=1` in their args, spec 43 step 8a) are another
  // chart's (step 8c): on a harness's line they would take the 4,096 point's place. So are
  // the harvested 25k runs (`preset=25k`, or 24,576 a side; step 8d), behind its toggle.
  function inSet(item) {
    var args = item.record.source.args;
    if (!Array.isArray(args)) return false;
    return kind(item) === 'harvested' ? harvested25k(args) : args.indexOf('series=1') >= 0;
  }

  function harvested25k(args) {
    return args.indexOf('preset=25k') >= 0 || args.indexOf('side=24576') >= 0;
  }

  function benchRecords(bundle) {
    return bundle.filter(function (item) {
      return item.record && item.record.series === 'bench' && item.record.source &&
        item.record.results && !inSet(item);
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
      cpu: typeof (record.machine || {}).cpu === 'string' ? record.machine.cpu : UNKNOWN,
    };
  }

  // A harvested record's place on main's line: its head's, or else its base's, position
  // among the fresh records (a 7-character sha or more matching the start of theirs).
  function placer(bundle) {
    var places = benchRecords(bundle).filter(function (item) {
      return kind(item) !== 'harvested' && number((item.record.commit || {}).position) !== null;
    }).map(function (item) { return { sha: String(item.record.commit.sha), position: item.record.commit.position }; });
    function at(sha) {
      if (typeof sha !== 'string' || sha.length < 7) return null;
      var found = places.find(function (place) { return place.sha.indexOf(sha) === 0; });
      return found ? found.position : null;
    }
    return function (commit) {
      var position = at((commit || {}).sha);
      return position === null ? at((commit || {}).base) : position;
    };
  }

  // Fresh records give each harness's line, one point per commit (its latest run); a
  // harvested record sits on its head's or its base's place on that line, apart.
  function lines(bundle) {
    var byHarness = {};
    var placeOf = placer(bundle);
    benchRecords(bundle).forEach(function (item) {
      var position = number((item.record.commit || {}).position);
      if (kind(item) !== 'harvested' && position !== null) {
        var harness = item.record.source.harness;
        var line = byHarness[harness] = byHarness[harness] || { harness: harness, byCommit: {}, harvested: [] };
        var next = point(item, position);
        var kept = line.byCommit[next.sha];
        if (!kept || next.runId > kept.runId) line.byCommit[next.sha] = next;
      }
    });
    benchRecords(bundle).forEach(function (item) {
      if (kind(item) !== 'harvested') return;
      var position = placeOf(item.record.commit);
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
  // record) lies between two points, so a missing commit is never bridged by the line. It
  // breaks too where a point's `era` changes (the settled series, spec 47 step 2b): the two
  // sides measure different things.
  function segments(points, positions, key) {
    var valueOf = function (p) { return key ? p[key] : p.tick; };
    var order = positions.slice().sort(function (a, b) { return a - b; });
    var runs = [];
    var run = [];
    points.forEach(function (p) {
      var last = run[run.length - 1];
      var adjacent = last && order.indexOf(p.position) === order.indexOf(last.position) + 1 &&
        p.era === last.era;
      if (valueOf(p) === null || !adjacent) {
        if (run.length) runs.push(run);
        run = [];
      }
      if (valueOf(p) !== null) run.push(p);
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

  // Every commit on main's line with a fresh point in these lines.
  function positions(lines) {
    var seen = {};
    lines.forEach(function (line) { line.points.forEach(function (p) { seen[p.position] = true; }); });
    return Object.keys(seen).map(Number);
  }

  // The CPUs the lines' records name, "unknown" last.
  function cpus(lines) {
    var seen = {};
    lines.forEach(function (line) {
      line.points.concat(line.harvested).forEach(function (p) { seen[p.cpu] = true; });
    });
    return Object.keys(seen).sort(function (a, b) {
      if ((a === UNKNOWN) !== (b === UNKNOWN)) return a === UNKNOWN ? 1 : -1;
      return a < b ? -1 : a > b ? 1 : 0;
    });
  }

  // The lines without the records of the CPUs in `hidden` ({cpu: true}).
  function withoutCpus(lines, hidden) {
    var kept = function (p) { return !hidden[p.cpu]; };
    return lines.map(function (line) {
      return { harness: line.harness, points: line.points.filter(kept),
        harvested: line.harvested.filter(kept), hiddenHarvested: line.hiddenHarvested || 0 };
    });
  }

  // A ms value of point `p` in a unit: 'ms'; 'reference', over the reference's tick timed in
  // the same record; 'percent', over the record's own tick. Null when it has no such base.
  function msIn(p, ms, unit) {
    if (ms === null || ms === undefined) return null;
    if (unit === 'reference') return p.referenceTick ? round(ms / p.referenceTick) : null;
    if (unit === 'percent') return p.tick ? round(ms / p.tick * 100) : null;
    return ms;
  }

  function tickIn(p, unit) {
    return msIn(p, p.tick, unit);
  }

  // The lines in a unit, each point with its `value`. In × the reference a record with no
  // reference is left out, and counted in `withoutReference`.
  function inUnit(lines, unit) {
    var without = 0;
    var convert = function (list) {
      return list.filter(function (p) {
        var left = unit === 'reference' && !p.referenceTick;
        if (left) without++;
        return !left;
      }).map(function (p) { return Object.assign({}, p, { value: tickIn(p, unit) }); });
    };
    var converted = lines.map(function (line) {
      return { harness: line.harness, points: convert(line.points), harvested: convert(line.harvested),
        hiddenHarvested: line.hiddenHarvested || 0 };
    });
    return { withoutReference: without, lines: converted };
  }

  // A point's bar: each of `groups`, then 'other' (the tick less every group), in the unit,
  // leaving out the hidden ones ({group: true}) and the empty; no share changes with hiding.
  function stack(p, groups, hidden, unit) {
    var named = 0;
    var parts = groups.map(function (group) {
      var ms = valueAt(p.results, group) || 0;
      named += ms;
      return { group: group, ms: ms };
    });
    parts.push({ group: 'other', ms: Math.max(0, (p.tick || 0) - named) });
    return parts.filter(function (part) { return part.ms > 0 && !hidden[part.group]; })
      .map(function (part) { return { group: part.group, value: msIn(p, round(part.ms), unit) }; });
  }

  // compare() of two points, each side's values in the unit over its own record.
  function compareIn(a, b, unit) {
    return compare(a.results, b.results).map(function (row) {
      return { path: row.path, a: msIn(a, row.a, unit), b: msIn(b, row.b, unit), finer: row.finer };
    });
  }

  var api = { positions: positions, cpus: cpus, withoutCpus: withoutCpus, tickIn: tickIn, inUnit: inUnit,
    stack: stack, compareIn: compareIn, UNKNOWN: UNKNOWN, shown: shown, segments: segments, valueAt: valueAt, compare: compare, lines: lines,
    placer: placer, harvested25k: harvested25k, bridge: bridge, REPO: REPO };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.BreachMetricsView = api;
})(this);
