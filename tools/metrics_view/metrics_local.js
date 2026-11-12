// The local section's logic (specs/48-local-bench-series.md, step 2, Decision 178): the
// owner's PC's records (`series` "local", spec 48 step 1) into runs, one per date and commit,
// and a run into curves: the tick or a phase against units a side, one curve per squad count,
// or against squad size, one curve per total. Each point is a cell's median, with its rounds'
// range. Local records are raw ms on one machine; CI's charts never read them (their filters
// keep `series` "bench"). A run's repeat records (spec 48 step 5c) are not cells of its series:
// they are points at their ticks beside it, a guide with Q47's caveat (#909). No DOM here;
// metrics_local_page.js draws.
(function (root) {
  'use strict';

  var TICK = 'tick';
  // Decision 159's goals at 24,576 a side, measured on CI's 4-thread runner.
  var GOALS = [{ ms: 50, label: 'target' }, { ms: 100, label: 'interim' }];
  var GOAL_SIDE = 24576;
  // The repeat mode's records (spec 48 step 5a) and Q47's answer (a), #909.
  var REPEAT_HARNESS = 'tick_phases-repeat';
  var REPEAT_CAVEAT = 'Repeats replay one tick: they under-report phases whose cost builds up ' +
    'across ticks (strays, groups, bodies: pushes, stamina). The whole tick is the judge.';
  var GROUPINGS = {
    squads: { curve: 'squads', x: function (cell) { return cell.side; } },
    size: { curve: 'side', x: function (cell) { return cell.side / cell.squads; } },
  };

  function number(value) {
    return typeof value === 'number' && isFinite(value) ? value : null;
  }

  function local(item) {
    var record = (item || {}).record || {};
    return record.series === 'local' && record.source && record.results && record.commit;
  }

  function argOf(record, name) {
    var found = null;
    ((record.source || {}).args || []).forEach(function (arg) {
      var parts = String(arg).split('=');
      if (parts[0] === name) found = Number(parts[1]);
    });
    return found;
  }

  // A record's cell, {side, squads}, or null when its args don't name both.
  function cellOf(record) {
    var side = argOf(record, 'side');
    var squads = argOf(record, 'squads');
    return side > 0 && squads > 0 ? { side: side, squads: squads } : null;
  }

  function repeated(record) {
    return record.source.harness === REPEAT_HARNESS && argOf(record, 'at') !== null;
  }

  function dateOf(record) {
    var date = (record.provenance || {}).date;
    return typeof date === 'string' ? date : '';
  }

  // Every run, newest first: the records of one date and commit, with the machine they name.
  function runs(bundle) {
    var found = {};
    bundle.filter(local).forEach(function (item) {
      var record = item.record;
      if (!cellOf(record)) return;
      var sha = String(record.commit.sha || '');
      var key = dateOf(record) + ' ' + sha;
      var run = found[key] = found[key] || {
        key: key, date: dateOf(record), sha: sha, position: number(record.commit.position),
        title: typeof record.commit.title === 'string' ? record.commit.title : '',
        machine: record.machine || {}, records: [], repeats: [],
      };
      (repeated(record) ? run.repeats : run.records).push(record);
    });
    return Object.keys(found).map(function (key) { return found[key]; }).sort(function (a, b) {
      if (a.date !== b.date) return a.date < b.date ? 1 : -1;
      return (b.position || 0) - (a.position || 0);
    });
  }

  // The run after `key` in time's order, the one drawn dotted beside it, or null.
  function before(all, key) {
    for (var i = 0; i < all.length; i++) {
      if (all[i].key === key) return all[i + 1] || null;
    }
    return null;
  }

  function measure(record, phase) {
    var result = (record.results || {})[phase] || {};
    var median = number(result.median);
    if (median === null) return null;
    var values = Array.isArray(result.values) ? result.values.filter(function (v) { return number(v) !== null; }) : [];
    if (!values.length) values = [median];
    return { median: median, lo: Math.min.apply(null, values), hi: Math.max.apply(null, values) };
  }

  // The phases a run's records time: the tick first, then the rest in path order.
  function phases(run) {
    var seen = {};
    run.records.concat(run.repeats || []).forEach(function (record) {
      Object.keys(record.results || {}).forEach(function (phase) {
        if (measure(record, phase)) seen[phase] = true;
      });
    });
    return Object.keys(seen).sort(function (a, b) {
      if (a === TICK || b === TICK) return a === TICK ? -1 : 1;
      return a < b ? -1 : a > b ? 1 : 0;
    });
  }

  // The run's largest cell: the most units a side, and of those the costliest tick.
  function largest(run) {
    return run.records.reduce(function (best, record) {
      if (!best) return record;
      var a = cellOf(record).side;
      var b = cellOf(best).side;
      if (a !== b) return a > b ? record : best;
      return ((measure(record, TICK) || {}).median || 0) > ((measure(best, TICK) || {}).median || 0) ? record : best;
    }, null);
  }

  // The `count` costliest phases below the tick at the largest cell, costliest first.
  function costliest(run, count) {
    var record = largest(run);
    if (!record) return [];
    return phases(run).filter(function (phase) { return phase !== TICK; }).map(function (phase) {
      return { phase: phase, median: (measure(record, phase) || {}).median };
    }).filter(function (p) { return p.median !== undefined; })
      .sort(function (a, b) { return b.median - a.median; })
      .slice(0, count).map(function (p) { return p.phase; });
  }

  // A run's curves for `options.phase` (default the tick): `grouping` "squads" (x units a
  // side, a curve per squad count) or "size" (x squad size, a curve per units a side), with
  // `perSquad` dividing each value by the cell's squads. Curves and their points in order;
  // a cell with no number for the phase is left out.
  function curves(run, options) {
    var grouping = GROUPINGS[(options || {}).grouping] || GROUPINGS.squads;
    var phase = (options || {}).phase || TICK;
    var divide = !!(options || {}).perSquad;
    var found = {};
    run.records.forEach(function (record) {
      var cell = cellOf(record);
      var at = measure(record, phase);
      if (!at) return;
      var by = divide ? cell.squads : 1;
      var key = cell[grouping.curve];
      (found[key] = found[key] || []).push({
        x: grouping.x(cell), y: at.median / by, lo: at.lo / by, hi: at.hi / by,
        side: cell.side, squads: cell.squads,
      });
    });
    return Object.keys(found).map(Number).sort(function (a, b) { return a - b; }).map(function (key) {
      return { key: key, points: found[key].sort(function (a, b) { return a.x - b.x; }) };
    });
  }

  // A run's repeats for `options` as curves' points take them: each at its cell's x, in the
  // curve `key` its cell's series is in, with the `tick` it replays and its `n` repeats.
  // Ordered by curve, x, then tick.
  function repeats(run, options) {
    var grouping = GROUPINGS[(options || {}).grouping] || GROUPINGS.squads;
    var phase = (options || {}).phase || TICK;
    var divide = !!(options || {}).perSquad;
    var found = [];
    (run.repeats || []).forEach(function (record) {
      var cell = cellOf(record);
      var at = measure(record, phase);
      if (!at) return;
      var by = divide ? cell.squads : 1;
      found.push({
        key: cell[grouping.curve], x: grouping.x(cell), y: at.median / by, lo: at.lo / by, hi: at.hi / by,
        tick: argOf(record, 'at'), n: argOf(record, 'repeat'), side: cell.side, squads: cell.squads,
      });
    });
    return found.sort(function (a, b) { return a.key - b.key || a.x - b.x || a.tick - b.tick; });
  }

  // A machine's heading: CPU, cores and threads, RAM and OS, from what the record names.
  function machineLine(machine) {
    var parts = [];
    if (typeof machine.cpu === 'string') parts.push(machine.cpu);
    var cores = number(machine.cores);
    var threads = number(machine.threads);
    if (cores !== null || threads !== null) {
      parts.push([cores !== null ? cores + ' cores' : '', threads !== null ? threads + ' threads' : '']
        .filter(Boolean).join(', '));
    }
    var ram = number(machine.ram_mb);
    if (ram !== null) parts.push(Math.round(ram / 1024) + ' GB');
    if (typeof machine.os === 'string') parts.push(machine.os);
    return parts.length ? parts.join(', ') : 'unknown machine';
  }

  var api = { runs: runs, before: before, phases: phases, costliest: costliest, curves: curves,
    repeats: repeats, cellOf: cellOf, machineLine: machineLine, GOALS: GOALS, GOAL_SIDE: GOAL_SIDE,
    TICK: TICK, REPEAT_CAVEAT: REPEAT_CAVEAT };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.BreachMetricsLocal = api;
})(this);
