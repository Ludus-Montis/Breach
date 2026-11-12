// The local section's drawing (specs/48-local-bench-series.md, step 2, Decision 178):
// metrics_local.js's curves as SVG, below CI's charts. Headed by the run's machine, commit and
// date; the tick or a picked phase against units a side (a curve per squad count) or against
// squad size (a curve per total), both log scales, each point a median with a bar for its
// rounds' range. With the tick, the three costliest phases at the largest cell are drawn thin,
// and Decision 159's lines are a guide. A run picker when there are several; the run before
// the picked one is dotted. metrics_page.js hands it the bundle.
(function () {
  'use strict';
  var local = window.BreachMetricsLocal;
  var NS = 'http://www.w3.org/2000/svg';
  var COLOURS = ['var(--s1)', 'var(--s2)', 'var(--s3)', 'var(--s4)', 'var(--s5)', 'var(--s6)', 'var(--s7)', 'var(--s8)'];
  var PAD = { left: 56, right: 150, top: 24, bottom: 44 };
  var WIDTH = 760;
  var HEIGHT = 360;
  var TICKS = [0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000];
  var state = { bundle: [], runs: [], run: null, grouping: 'squads', phase: local.TICK, perSquad: false };

  function $(id) { return document.getElementById(id); }

  function el(name, attrs, parent) {
    var node = document.createElementNS(NS, name);
    Object.keys(attrs || {}).forEach(function (key) { node.setAttribute(key, attrs[key]); });
    if (parent) parent.appendChild(node);
    return node;
  }

  function short(sha) { return String(sha).slice(0, 7); }

  function phaseName(phase) {
    return phase === local.TICK ? 'whole tick' : phase.replace(/^tick\//, '').replace(/\//g, ': ').replace(/-/g, ' ');
  }

  function curveName(key) {
    if (state.grouping === 'size') return key.toLocaleString('en') + ' a side';
    return key + (key === 1 ? ' squad' : ' squads');
  }

  function runName(run) {
    return run.date + ' on ' + short(run.sha);
  }

  function options(phase) {
    return { grouping: state.grouping, phase: phase, perSquad: state.perSquad };
  }

  // The values a chart must show: every curve's bars, and the goal lines where they apply.
  function extent(sets) {
    var xs = [];
    var ys = [];
    sets.forEach(function (curves) {
      curves.forEach(function (c) {
        c.points.forEach(function (p) { xs.push(p.x); ys.push(p.lo, p.hi); });
      });
    });
    return { xs: xs, ys: ys.filter(function (v) { return v > 0; }) };
  }

  function goalsShown() {
    return state.phase === local.TICK && !state.perSquad;
  }

  function polyline(points, x, y, attrs, root) {
    if (points.length < 2) return;
    el('polyline', Object.assign({ points: points.map(function (p) { return x(p.x) + ',' + y(p.y); }).join(' ') }, attrs), root);
  }

  function axes(root, x, y, lo, hi, xs) {
    TICKS.forEach(function (v) {
      if (Math.log10(v) < lo || Math.log10(v) > hi) return;
      el('line', { class: 'grid', x1: PAD.left, x2: WIDTH - PAD.right, y1: y(v), y2: y(v) }, root);
      el('text', { x: PAD.left - 6, y: y(v) + 4, 'text-anchor': 'end' }, root).textContent = v;
    });
    var seen = {};
    xs.forEach(function (v) {
      if (seen[v]) return;
      seen[v] = true;
      el('line', { class: 'grid', x1: x(v), x2: x(v), y1: PAD.top, y2: HEIGHT - PAD.bottom }, root);
      el('text', { x: x(v), y: HEIGHT - PAD.bottom + 14, 'text-anchor': 'middle' }, root).textContent = v.toLocaleString('en');
    });
    el('text', { x: 4, y: 14 }, root).textContent = state.perSquad ? 'ms per squad' : 'ms';
    el('text', { x: PAD.left, y: HEIGHT - 8 }, root).textContent =
      (state.grouping === 'size' ? 'squad size (units)' : 'units a side') + ', log scale →';
  }

  function drawCurve(curve, colour, x, y, root, attrs, label) {
    polyline(curve.points, x, y, Object.assign({ class: 'line', stroke: colour }, attrs.line), root);
    curve.points.forEach(function (p) {
      if (attrs.bars) el('line', { class: 'range', stroke: colour, x1: x(p.x), x2: x(p.x), y1: y(p.lo), y2: y(p.hi) }, root);
      var dot = el('circle', Object.assign({ class: 'point', cx: x(p.x), cy: y(p.y), r: attrs.r, fill: colour, stroke: colour }, attrs.dot), root);
      el('title', {}, dot).textContent = label + ': ' + p.side.toLocaleString('en') + ' a side in ' + p.squads +
        (p.squads === 1 ? ' squad' : ' squads') + ' of ' + (p.side / p.squads).toLocaleString('en') +
        '\nmedian ' + p.y.toFixed(2) + ' ms, rounds ' + p.lo.toFixed(2) + ' to ' + p.hi.toFixed(2);
    });
    var last = curve.points[curve.points.length - 1];
    if (last && attrs.name) {
      el('text', { x: x(last.x) + 6, y: y(last.y) + 4 }, root).textContent = attrs.name;
    }
  }

  // The run's repeats (spec 48 step 5c): hollow points at their cell, set to the left of its
  // series' point, one step per tick it replays, each with its bar: its rounds' quartiles where
  // the record keeps them, else its rounds' medians (step 5e). The hover text says which.
  function drawRepeats(points, main, colour, x, y, root) {
    var placed = {};
    points.forEach(function (p) {
      var i = main.findIndex(function (c) { return c.key === p.key; });
      var tint = colour(i < 0 ? main.length : i);
      var slot = placed[p.key + ' ' + p.x] = (placed[p.key + ' ' + p.x] || 0) + 1;
      var at = x(p.x) - 7 * slot;
      el('line', { class: 'range', stroke: tint, x1: at, x2: at, y1: y(p.lo), y2: y(p.hi) }, root);
      var dot = el('circle', { class: 'point repeat', cx: at, cy: y(p.y), r: 2.5, fill: 'var(--panel)', stroke: tint }, root);
      el('title', {}, dot).textContent = 'tick ' + p.tick + ' repeated ' + p.n + ' times, ' +
        p.side.toLocaleString('en') + ' a side in ' + p.squads + (p.squads === 1 ? ' squad' : ' squads') +
        '\nmedian ' + p.y.toFixed(2) + ' ms, ' + (p.spread === 'quartiles' ? 'quartiles (lowest q1 to highest q3) '
          : 'rounds\' medians ') + p.lo.toFixed(2) + ' to ' + p.hi.toFixed(2) +
        '\n' + local.REPEAT_CAVEAT;
    });
  }

  function drawChart() {
    var target = $('local-chart');
    target.textContent = '';
    var run = state.run;
    var prior = local.before(state.runs, run.key);
    var main = local.curves(run, options(state.phase));
    var older = prior ? local.curves(prior, options(state.phase)) : [];
    var thin = state.phase === local.TICK ? local.costliest(run, 3) : [];
    var thinCurves = thin.map(function (phase) { return { phase: phase, curves: local.curves(run, options(phase)) }; });
    var repeats = local.repeats(run, options(state.phase));
    var found = extent([main, older, [{ points: repeats }]].concat(thinCurves.map(function (t) { return t.curves; })));
    if (!found.ys.length) { target.textContent = 'No numbers for this phase in this run.'; return; }
    if (goalsShown()) found.ys = found.ys.concat(local.GOALS.map(function (g) { return g.ms; }));
    var root = el('svg', { width: WIDTH, height: HEIGHT, viewBox: '0 0 ' + WIDTH + ' ' + HEIGHT, role: 'img' }, target);
    var lo = Math.log10(Math.min.apply(null, found.ys) * 0.8);
    var hi = Math.log10(Math.max.apply(null, found.ys) * 1.25);
    var xlo = Math.log10(Math.min.apply(null, found.xs) / 1.3);
    var xhi = Math.log10(Math.max.apply(null, found.xs) * 1.3);
    var y = function (v) { return PAD.top + (hi - Math.log10(v)) / (hi - lo) * (HEIGHT - PAD.top - PAD.bottom); };
    var x = function (v) { return PAD.left + (Math.log10(v) - xlo) / (xhi - xlo) * (WIDTH - PAD.left - PAD.right); };
    axes(root, x, y, lo, hi, found.xs);
    if (goalsShown()) {
      local.GOALS.forEach(function (goal) {
        el('line', { class: 'goal', x1: PAD.left, x2: WIDTH - PAD.right, y1: y(goal.ms), y2: y(goal.ms) }, root);
        el('text', { x: WIDTH - PAD.right + 4, y: y(goal.ms) + 4 }, root).textContent = goal.ms + ' ms ' + goal.label;
      });
    }
    var colour = function (i) { return COLOURS[i % COLOURS.length]; };
    older.forEach(function (curve) {
      var i = main.findIndex(function (c) { return c.key === curve.key; });
      drawCurve(curve, colour(i < 0 ? main.length : i), x, y, root,
        { line: { class: 'line before' }, dot: { fill: 'var(--panel)' }, r: 2.5 }, runName(prior) + ', ' + curveName(curve.key));
    });
    thinCurves.forEach(function (t) {
      t.curves.forEach(function (curve, i) {
        drawCurve(curve, colour(i), x, y, root, { line: { class: 'line thin' }, r: 1.5 },
          phaseName(t.phase) + ', ' + curveName(curve.key));
      });
    });
    main.forEach(function (curve, i) {
      drawCurve(curve, colour(i), x, y, root, { bars: true, r: 3.5, name: curveName(curve.key) },
        phaseName(state.phase) + ', ' + curveName(curve.key));
    });
    drawRepeats(repeats, main, colour, x, y, root);
    $('local-legend').innerHTML = main.map(function (curve, i) {
      return '<span style="--swatch:' + colour(i) + '">' + curveName(curve.key) + '</span>';
    }).join('') + (thin.length ? '<span>thin: ' + thin.map(phaseName).join(', ') + '</span>' : '') +
      (prior ? '<span>dotted: ' + runName(prior) + '</span>' : '') +
      (repeats.length ? '<span>hollow, left of a point: its repeated ticks, a guide. ' + local.REPEAT_CAVEAT + '</span>' : '');
  }

  function drawHeading() {
    var run = state.run;
    $('local-machine').textContent = local.machineLine(run.machine);
    $('local-run').innerHTML = 'Run of ' + run.date + ' on <code>' + short(run.sha) + '</code>' +
      (run.title ? ': ' + run.title.replace(/</g, '&lt;') : '') + ' (' + run.records.length + ' cells).';
  }

  function fillPickers() {
    var runPick = $('local-run-pick');
    runPick.innerHTML = state.runs.map(function (run) { return '<option value="' + run.key + '">' + runName(run) + '</option>'; }).join('');
    runPick.value = state.run.key;
    runPick.hidden = state.runs.length < 2;
    var phasePick = $('local-phase');
    var all = local.phases(state.run);
    if (all.indexOf(state.phase) < 0) state.phase = local.TICK;
    phasePick.innerHTML = all.map(function (phase) { return '<option value="' + phase + '">' + phaseName(phase) + '</option>'; }).join('');
    phasePick.value = state.phase;
  }

  function redraw() {
    drawHeading();
    fillPickers();
    drawChart();
  }

  // Draws the section from the page's bundle; hidden when it holds no local record.
  function draw(bundle) {
    state.bundle = bundle;
    state.runs = local.runs(bundle);
    $('local').hidden = !state.runs.length;
    if (!state.runs.length) return;
    state.run = state.run && state.runs.find(function (r) { return r.key === state.run.key; }) || state.runs[0];
    $('local-run-pick').onchange = function (event) {
      state.run = state.runs.find(function (r) { return r.key === event.target.value; });
      redraw();
    };
    $('local-phase').onchange = function (event) { state.phase = event.target.value; drawChart(); };
    $('local-grouping').onchange = function (event) { state.grouping = event.target.value; drawChart(); };
    $('local-per-squad').onchange = function (event) { state.perSquad = event.target.checked; drawChart(); };
    redraw();
  }

  window.BreachDrawLocal = draw;
})();
