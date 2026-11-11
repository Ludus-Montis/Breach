// The Showcase tab's logic (specs/44-project-site-and-showcase.md, step 5, Decisions 167 and
// 169), from manifest.json's captures (ci/showcase/README.md): the timeline, one card a PR
// (its latest capture) newest first; the capture each is compared with; the pairing by
// scenario; each timing's change; the behaviours; the filters. No DOM here;
// showcase_page.js draws.
(function (root) {
  'use strict';

  var TICK_KEYS = ['tick_median_ms', 'tick_p95_ms'];

  function newestFirst(captures) {
    return captures.slice().sort(function (a, b) {
      return a.date < b.date ? 1 : a.date > b.date ? -1 : 0;
    });
  }

  function timeline(captures) {
    var seen = {};
    return newestFirst(captures).filter(function (capture) {
      if (seen[capture.pr]) return false;
      seen[capture.pr] = true;
      return true;
    });
  }

  function isOpen(capture) {
    return capture.state === 'open';
  }

  // Open: the PR's own capture before this one. Merged or closed: the capture of the PR
  // merged last before this one closed (main as it stood).
  function partner(capture, captures) {
    var sorted = newestFirst(captures);
    if (isOpen(capture)) {
      return sorted.filter(function (other) {
        return other.pr === capture.pr && other.date < capture.date;
      })[0] || null;
    }
    var best = null;
    sorted.forEach(function (other) {
      if (other.state !== 'merged' || other.pr === capture.pr) return;
      if (!other.closed_at || !(other.closed_at < capture.closed_at)) return;
      if (!best || other.closed_at > best.closed_at) best = other;
    });
    return best;
  }

  function byName(scenarios) {
    var out = {};
    (scenarios || []).forEach(function (made) { out[made.name] = made; });
    return out;
  }

  // In the current capture's order, then the previous capture's own.
  function pairs(current, previous) {
    var before = byName(previous && previous.scenarios);
    var now = byName(current.scenarios);
    var out = current.scenarios.map(function (made) {
      return { name: made.name, current: made, previous: before[made.name] || null };
    });
    ((previous && previous.scenarios) || []).forEach(function (made) {
      if (!now[made.name]) out.push({ name: made.name, current: null, previous: made });
    });
    return out;
  }

  function number(value) {
    return typeof value === 'number' && isFinite(value) ? value : null;
  }

  function change(value, before) {
    value = number(value);
    before = number(before);
    if (value === null || before === null || before === 0) return null;
    return Math.round(((value - before) / before) * 1000) / 10;
  }

  function row(key, value, before) {
    return { key: key, value: number(value), before: number(before), change: change(value, before) };
  }

  // The tick's median and 95th percentile, then each phase's median.
  function timingRows(timings, before) {
    timings = timings || {};
    before = before || {};
    var rows = TICK_KEYS.map(function (key) { return row(key, timings[key], before[key]); });
    var phases = timings.phases_median_ms || {};
    var phasesBefore = before.phases_median_ms || {};
    Object.keys(phases).forEach(function (key) {
      rows.push(row(key, phases[key], phasesBefore[key]));
    });
    return rows;
  }

  // Seen (with the first tick), then not seen; `missing` marks those the scenario is named
  // for and didn't show.
  function behaviours(made) {
    var missing = made.missing || [];
    var seen = made.seen || {};
    var out = Object.keys(seen).map(function (name) {
      return { name: name, tick: seen[name], missing: false };
    });
    var notSeen = (made.not_seen || []).slice().sort(function (a, b) {
      return (missing.indexOf(b) >= 0) - (missing.indexOf(a) >= 0);
    });
    notSeen.forEach(function (name) {
      out.push({ name: name, tick: null, missing: missing.indexOf(name) >= 0 });
    });
    return out;
  }

  function hasScenario(capture, name) {
    return capture.scenarios.some(function (made) { return made.name === name; });
  }

  function filter(cards, options) {
    return cards.filter(function (capture) {
      if (options.states.indexOf(capture.state) < 0) return false;
      return !options.scenario || hasScenario(capture, options.scenario);
    });
  }

  function scenarioNames(cards) {
    var out = [];
    cards.forEach(function (capture) {
      capture.scenarios.forEach(function (made) {
        if (out.indexOf(made.name) < 0) out.push(made.name);
      });
    });
    return out;
  }

  var api = {
    timeline: timeline,
    isOpen: isOpen,
    partner: partner,
    pairs: pairs,
    change: change,
    timingRows: timingRows,
    behaviours: behaviours,
    filter: filter,
    scenarioNames: scenarioNames,
  };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  root.ShowcaseView = api;
})(typeof window !== 'undefined' ? window : globalThis);
