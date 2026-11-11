// The repeats' panels (specs/47-tick-inspection.md, step 5b; Decision 177): each inspection
// tick's repeats (`metrics/inspect/`, series `inspect`, step 5a) beside the same phase's lap
// from the whole tick, so the gap between them shows how far to trust the fast number.
//
// The lap is the 25k set's record (`series=1`) of the same commit and the preset's shape:
// `4096` is 4 lanes of 1,024 a side, `25k` is 24, the same battle from contact that `at=`
// replays, timed over the whole settled series (its mean tick, not the inspected one).
//
// % of tick is the default unit (the owner, 2026-11-11): each side over its own tick, so the
// runner cancels within each record. × the reference doesn't compare the two sides' levels:
// a repeat's reference is the bridge timing `bench`'s battle (`side=4096 squads=4 fight=3`),
// a set record's is the bridge timing its own shape. ms compares runners as well as code.
//
// Q47 (#909, option a): the repeats under-report phases whose cost builds up across ticks; a
// panel holding one carries the caveat. No DOM here; metrics_inspect_page.js draws.
(function (root) {
  'use strict';

  var view = typeof module !== 'undefined' && module.exports
    ? require('./metrics_view.js') : root.BreachMetricsView;
  var HARNESS = 'tick_phases-repeat';
  var PRESETS = [
    { preset: '4096', label: '4,096 a side (4 lanes)', side: 4096, squads: 4 },
    { preset: '25k', label: '24,576 a side (24 lanes)', side: 24576, squads: 24 },
  ];
  var CAVEAT = ['tick/strays', 'tick/groups-form-up-split', 'tick/bodies/pushes', 'tick/stamina'];
  var DEFAULT_UNIT = 'percent';

  function number(value) {
    return typeof value === 'number' && isFinite(value) ? value : null;
  }

  function round(value) {
    return Math.round(value * 10000) / 10000;
  }

  // An args list's `key=value`, the last one given.
  function arg(args, key) {
    var found = null;
    (args || []).forEach(function (a) {
      if (typeof a === 'string' && a.indexOf(key + '=') === 0) found = a.slice(key.length + 1);
    });
    return found;
  }

  function fresh(item) {
    var record = item.record || {};
    return (record.provenance || {}).kind === 'fresh' && record.source && record.results &&
      number((record.commit || {}).position) !== null;
  }

  function entry(item) {
    var record = item.record;
    return {
      position: record.commit.position,
      sha: String(record.commit.sha || ''),
      title: typeof record.commit.title === 'string' ? record.commit.title : '',
      results: record.results,
      tick: view.valueAt(record.results, 'tick'),
      referenceTick: view.valueAt(((record.reference || {}).results) || {}, 'tick'),
      runId: (record.provenance || {}).run_id || 0,
      cpu: typeof (record.machine || {}).cpu === 'string' ? record.machine.cpu : view.UNKNOWN,
      path: item.path,
    };
  }

  // Keeps, per key, the latest run.
  function latest(byKey, key, next) {
    var kept = byKey[key];
    if (!kept || next.runId > kept.runId) byKey[key] = next;
  }

  // Each preset's repeats, one per (commit, tick), and laps, one per commit, among the CPUs shown.
  function gather(bundle, hidden) {
    var repeats = {};
    var laps = {};
    bundle.filter(fresh).forEach(function (item) {
      var record = item.record;
      var args = record.source.args;
      var e = entry(item);
      if (hidden[e.cpu]) return;
      if (record.series === 'inspect' && record.source.harness === HARNESS) {
        var preset = arg(args, 'preset');
        e.at = Number(arg(args, 'at'));
        if (preset && isFinite(e.at)) latest(repeats, preset + ' ' + e.sha + ' ' + e.at, Object.assign(e, { preset: preset }));
        return;
      }
      if (record.series !== 'bench' || (args || []).indexOf('series=1') < 0) return;
      PRESETS.forEach(function (p) {
        if (arg(args, 'side') === String(p.side) && arg(args, 'squads') === String(p.squads)) {
          latest(laps, p.preset + ' ' + e.sha, Object.assign(e, { preset: p.preset }));
        }
      });
    });
    var values = function (byKey) { return Object.keys(byKey).map(function (k) { return byKey[k]; }); };
    return { repeats: values(repeats), laps: values(laps) };
  }

  // A phase's value in a record, in a unit: 'percent' over its own tick, 'reference' over its
  // own reference's tick, 'ms' as timed. Null for a gap or a missing base.
  function valueIn(e, phase, unit) {
    var ms = view.valueAt(e.results, phase);
    if (ms === null) return null;
    if (unit === 'percent') return e.tick ? round(ms / e.tick * 100) : null;
    if (unit === 'reference') return e.referenceTick ? round(ms / e.referenceTick) : null;
    return ms;
  }

  // The top-level phases (just under the tick) any record names.
  function phasesOf(entries) {
    var seen = {};
    entries.forEach(function (e) {
      Object.keys(e.results).forEach(function (path) {
        var parts = path.split('/');
        if (parts[0] === 'tick' && parts.length > 1) seen['tick/' + parts[1]] = true;
      });
    });
    return Object.keys(seen);
  }

  function caveated(phase) {
    return CAVEAT.some(function (c) { return c === phase || c.indexOf(phase + '/') === 0; });
  }

  function median(values) {
    var sorted = values.slice().sort(function (a, b) { return a - b; });
    var mid = Math.floor(sorted.length / 2);
    return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
  }

  // The repeats' median over the lap, at the latest commit with both.
  function gapOf(repeats, laps) {
    var found = null;
    laps.forEach(function (l) {
      if (!l.value) return;
      var mine = repeats.filter(function (r) { return r.position === l.position; });
      if (!mine.length || (found && found.position > l.position)) return;
      found = { position: l.position, sha: l.sha,
        ratio: round(median(mine.map(function (r) { return r.value; })) / l.value) };
    });
    return found;
  }

  function point(e, phase, unit) {
    return { position: e.position, sha: e.sha, title: e.title, at: e.at, cpu: e.cpu, path: e.path,
      value: valueIn(e, phase, unit) };
  }

  function byPlace(a, b) {
    return a.position - b.position || (a.at || 0) - (b.at || 0);
  }

  // The panels per preset with repeats: {sizes: [{preset, label, ticks, panels: [{phase,
  // caveat, repeats, laps, gap}]}]}, panels largest first, the laps from the first commit with
  // repeats on (no backfill, step 5a). `hidden` CPUs ({cpu: true}) are out.
  function chart(bundle, unit, hidden) {
    unit = unit || DEFAULT_UNIT;
    var found = gather(bundle, hidden || {});
    var sizes = PRESETS.map(function (p) {
      var repeats = found.repeats.filter(function (e) { return e.preset === p.preset; });
      var laps = found.laps.filter(function (e) { return e.preset === p.preset; });
      if (!repeats.length) return null;
      var first = Math.min.apply(null, repeats.map(function (e) { return e.position; }));
      laps = laps.filter(function (e) { return e.position >= first; });
      var panels = phasesOf(repeats).map(function (phase) {
        var mine = repeats.map(function (e) { return point(e, phase, unit); })
          .filter(function (q) { return q.value !== null; }).sort(byPlace);
        var theirs = laps.map(function (e) { return point(e, phase, unit); })
          .filter(function (q) { return q.value !== null; }).sort(byPlace);
        return { phase: phase, caveat: caveated(phase), repeats: mine, laps: theirs, gap: gapOf(mine, theirs) };
      }).filter(function (panel) { return panel.repeats.length; });
      var size = function (panel) {
        return Math.max.apply(null, panel.repeats.concat(panel.laps).map(function (q) { return q.value; }));
      };
      panels.sort(function (a, b) { return size(b) - size(a) || (a.phase < b.phase ? -1 : 1); });
      var ticks = {};
      repeats.forEach(function (e) { ticks[e.at] = true; });
      return { preset: p.preset, label: p.label, panels: panels,
        ticks: Object.keys(ticks).map(Number).sort(function (a, b) { return a - b; }) };
    }).filter(Boolean);
    return { unit: unit, sizes: sizes };
  }

  var api = { chart: chart, valueIn: valueIn, DEFAULT_UNIT: DEFAULT_UNIT, CAVEAT: CAVEAT, PRESETS: PRESETS };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.BreachMetricsInspect = api;
})(this);
