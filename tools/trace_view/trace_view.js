// The outcome trace page's logic (specs/37-outcome-trace.md, step 4): a base and a head
// trace (the digest's JSON lines, tools/formation_digest.gd `trace=`) and trace_diff.py's
// --json, indexed by tick for the page to draw. No DOM here; trace_page.js draws.
//
// A trace line: {run, tick, events, squads: [{id, faction, <fields>, units: [{id,
// <fields>}]}]}. Units are matched by id, never by place (Decision 97).
(function (root) {
  'use strict';

  var EMPTY = { squads: [], units: [] };
  // Infinity and NaN as the trace writes them (Python's json reads them; JSON.parse doesn't).
  var ODD = /([:\[,]\s*)(-?Infinity|NaN)(?=\s*[,\]}])/g;

  // The trace's lines, by tick.
  function parse(text) {
    var out = {};
    text.split('\n').forEach(function (raw) {
      if (!raw.trim()) return;
      var line = JSON.parse(raw.replace(ODD, '$1"$2"'));
      out[line.tick] = line;
    });
    return out;
  }

  // A tick's squads, and its units with their squad and faction beside their fields.
  function side(line) {
    if (!line) return EMPTY;
    var units = [];
    line.squads.forEach(function (squad) {
      squad.units.forEach(function (unit) {
        units.push(Object.assign({ squad: squad.id, faction: squad.faction }, unit));
      });
    });
    return { squads: line.squads, units: units };
  }

  // Each tick's differing units, from the diff's per-tick entries.
  function ringsOf(diff) {
    var out = {};
    ((diff && diff.ticks) || []).forEach(function (entry) {
      out[entry.tick] = entry.units.map(function (unit) { return unit.id; });
    });
    return out;
  }

  function load(baseText, headText, diff) {
    var base = parse(baseText);
    var head = parse(headText);
    var rings = ringsOf(diff);
    var seen = {};
    Object.keys(base).concat(Object.keys(head)).forEach(function (t) { seen[t] = Number(t); });
    var ticks = Object.keys(seen).map(function (t) { return seen[t]; });
    ticks.sort(function (a, b) { return a - b; });
    var at = function (tick) { return { base: side(base[tick]), head: side(head[tick]) }; };
    var find = function (units, id) {
      return units.filter(function (unit) { return unit.id === id; })[0] || null;
    };
    return {
      ticks: ticks,
      diff: diff || null,
      at: at,
      ringed: function (tick) { return rings[tick] || []; },
      unit: function (tick, id) {
        var both = at(tick);
        return { base: find(both.base.units, id), head: find(both.head.units, id) };
      },
    };
  }

  var api = { load: load, parse: parse };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.BreachTraceView = api;
})(this);
