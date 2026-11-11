// The 25k chart's drawing (specs/43-metrics-history.md, steps 8c and 8d, Decision 165):
// metrics_25k.js's lines as SVG, below the 4,096 chart. 24,576 a side on a log scale, in ms or
// × the bridge: each family's p95 as the line and p99 as a thin band above it, its projection
// from the grid dotted, a merge whose miss is beyond 10% ringed, Decision 159's goal lines (ms
// only), each point marked by its CPU, the harvested 25k runs hollow when asked for, and a
// point's fitted curve over size when clicked. metrics_page.js hands it the bundle and the
// page's CPU and harvested toggles.
(function () {
  'use strict';
  var view = window.BreachMetricsView;
  var set = window.BreachMetrics25k;
  var NS = 'http://www.w3.org/2000/svg';
  var COLOUR = { lanes: 'var(--s1)', 'one squad': 'var(--s3)' };
  var GOALS = [{ ms: 50, label: 'target (p95, p99)' }, { ms: 100, label: 'interim (p95, p99)' }];
  var PAD = { left: 56, right: 140, top: 28, bottom: 40 };
  var HEIGHT = 320;
  var STEP = 14;
  var UNIT_NAME = { ms: 'ms', bridge: '× the bridge' };
  var state = { bundle: [], options: {}, unit: 'ms', details: null };

  function $(id) { return document.getElementById(id); }

  function el(name, attrs, parent) {
    var node = document.createElementNS(NS, name);
    Object.keys(attrs || {}).forEach(function (key) { node.setAttribute(key, attrs[key]); });
    if (parent) parent.appendChild(node);
    return node;
  }

  function short(sha) { return String(sha).slice(0, 7); }

  function percent(value) { return (value * 100).toFixed(1) + '%'; }

  function format(value) {
    if (value === null) return 'gap';
    return state.unit === 'bridge' ? '×' + value.toFixed(3) : value.toFixed(2) + ' ms';
  }

  // The SVG path of a CPU's mark (metrics_25k.js's MARKS) centred on (x, y).
  function markPath(mark, x, y, r) {
    var at = function (points) {
      return 'M' + points.map(function (p) { return (x + p[0] * r).toFixed(2) + ',' + (y + p[1] * r).toFixed(2); })
        .join('L') + 'Z';
    };
    if (mark === 'square') return at([[-0.9, -0.9], [0.9, -0.9], [0.9, 0.9], [-0.9, 0.9]]);
    if (mark === 'triangle') return at([[0, -1.2], [1.1, 0.8], [-1.1, 0.8]]);
    if (mark === 'diamond') return at([[0, -1.25], [1.25, 0], [0, 1.25], [-1.25, 0]]);
    if (mark === 'cross') {
      return at([[-0.35, -1.1], [0.35, -1.1], [0.35, -0.35], [1.1, -0.35], [1.1, 0.35], [0.35, 0.35],
        [0.35, 1.1], [-0.35, 1.1], [-0.35, 0.35], [-1.1, 0.35], [-1.1, -0.35], [-0.35, -0.35]]);
    }
    if (mark === 'star') {
      var star = [];
      for (var k = 0; k < 10; k++) {
        var a = Math.PI * k / 5 - Math.PI / 2;
        var d = k % 2 ? 0.5 : 1.25;
        star.push([Math.cos(a) * d, Math.sin(a) * d]);
      }
      return at(star);
    }
    return 'M' + (x - r) + ',' + y + 'a' + r + ',' + r + ' 0 1,0 ' + 2 * r + ',0a' + r + ',' + r + ' 0 1,0 ' + -2 * r + ',0Z';
  }

  function mark(cpu, x, y, attrs, parent) {
    return el('path', Object.assign({ d: markPath(set.markOf(state.options.cpus || [], cpu), x, y, 3.2) }, attrs), parent);
  }

  // A small SVG of a CPU's mark, for the page's CPU toggles.
  function swatch(cpus, cpu) {
    var root = el('svg', { width: 12, height: 12, viewBox: '0 0 12 12', class: 'mark' });
    el('path', { d: markPath(set.markOf(cpus, cpu), 6, 6, 4) }, root);
    return root;
  }

  // A point's tooltip: its numbers, the runner's CPU, the miss and the quadratic share.
  function describe(family, p) {
    return family.label + ' at ' + short(p.sha) + (p.title ? ': ' + p.title : '') +
      '\nCPU: ' + p.cpu +
      '\np95 ' + format(set.valueIn(p, 'p95', state.unit)) + ', p99 ' + format(set.valueIn(p, 'p99', state.unit)) +
      ', mean ' + format(set.valueIn(p, 'tick', state.unit)) +
      (p.predicted === null ? '\nno estimate: fewer than two grid sizes timed'
        : '\nestimate ' + format(set.valueIn(p, 'predicted', state.unit)) + ', miss ' + (p.miss >= 0 ? '+' : '') +
          percent(p.miss) + (p.ringed ? ' (beyond 10%)' : '') + '\nquadratic share c·x²/t: ' + percent(p.quadShare)) +
      '\nclick for its fitted curve over size';
  }

  function polyline(points, x, y, attrs, root) {
    el('polyline', Object.assign({ points: points.map(function (p) { return x(p) + ',' + y(p); }).join(' ') }, attrs), root);
  }

  // The chart's values in its unit: each point's measures, and the harvested runs in ms only.
  function values(chart) {
    var found = [];
    var at = function (p, key) { return set.valueIn(p, key, state.unit); };
    chart.families.forEach(function (family) {
      family.points.forEach(function (p) {
        [at(p, 'p95'), at(p, 'p99'), at(p, 'predicted')].forEach(function (v) { if (v) found.push(v); });
      });
    });
    if (state.unit === 'ms') chart.harvested.forEach(function (p) { found.push(p.tick); });
    return found;
  }

  function leftOut(chart) {
    if (state.unit !== 'bridge') return '';
    var count = chart.harvested.length;
    chart.families.forEach(function (family) {
      count += family.points.filter(function (p) { return set.valueIn(p, 'p95', 'bridge') === null; }).length;
    });
    return count ? count + ' point' + (count === 1 ? '' : 's') + ' with no bridge left out; ms shows ' +
      (count === 1 ? 'it' : 'them') + '. The goal lines are in ms only.' : 'The goal lines are in ms only.';
  }

  function drawChart() {
    var bundle = state.bundle;
    var options = state.options;
    var whole = set.chart(bundle, { harvested: options.harvested });
    var chart = set.withoutCpus(whole, options.hidden || {});
    var unit = state.unit;
    var target = $('chart-25k');
    target.textContent = '';
    $('unit-25k-name').textContent = UNIT_NAME[unit];
    $('note-25k').textContent = leftOut(chart);
    var found = values(chart);
    if (!found.length) {
      target.textContent = whole.families.some(function (f) { return f.points.length; })
        ? 'No 25k points with these CPUs in this unit.' : 'No 25k set recorded yet.';
      $('legend-25k').textContent = '';
      return;
    }
    if (unit === 'ms') found = found.concat(GOALS.map(function (goal) { return goal.ms; }));
    var index = {};
    chart.positions.forEach(function (position, i) { index[position] = i; });
    var width = Math.max(PAD.left + PAD.right + chart.positions.length * STEP, 600);
    var root = el('svg', { width: width, height: HEIGHT, viewBox: '0 0 ' + width + ' ' + HEIGHT, role: 'img' }, target);
    var lo = Math.log10(Math.min.apply(null, found) * 0.8);
    var hi = Math.log10(Math.max.apply(null, found) * 1.2);
    var y = function (v) { return PAD.top + (hi - Math.log10(v)) / (hi - lo) * (HEIGHT - PAD.top - PAD.bottom); };
    var x = function (p) { return PAD.left + index[p.position] * STEP + STEP / 2; };
    [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000].forEach(function (v) {
      if (Math.log10(v) < lo || Math.log10(v) > hi) return;
      el('line', { class: 'grid', x1: PAD.left, x2: width - PAD.right, y1: y(v), y2: y(v) }, root);
      el('text', { x: PAD.left - 6, y: y(v) + 4, 'text-anchor': 'end' }, root).textContent = unit === 'ms' ? v : '×' + v;
    });
    el('text', { x: 4, y: 14 }, root).textContent = UNIT_NAME[unit];
    if (unit === 'ms') {
      GOALS.forEach(function (goal) {
        el('line', { class: 'goal', x1: PAD.left, x2: width - PAD.right, y1: y(goal.ms), y2: y(goal.ms) }, root);
        el('text', { x: width - PAD.right + 4, y: y(goal.ms) + 4 }, root).textContent = goal.ms + ' ms ' + goal.label;
      });
    }
    chart.families.forEach(function (family) { drawFamily(family, chart, x, y, root); });
    chart.harvested.filter(function () { return unit === 'ms'; }).forEach(function (p) {
      var dot = mark(p.cpu, x(p), y(p.tick), { class: 'point', stroke: COLOUR[p.family], fill: 'var(--panel)' }, root);
      el('title', {}, dot).textContent = 'harvested ' + p.date + ' at ' + short(p.sha) + ': ' + p.args +
        '\nCPU: ' + p.cpu + '\nmean ' + p.tick.toFixed(2) + ' ms (other args, no reference; not fitted)';
    });
    el('text', { x: PAD.left, y: HEIGHT - 8 }, root).textContent = 'commits on main, oldest first →';
    $('legend-25k').innerHTML = chart.families.map(function (family) {
      return '<span style="--swatch:' + COLOUR[family.name] + '">' + family.label + ' (' +
        family.points.length + ' commits): p95, p99 band; dotted: estimate</span>';
    }).join('') + (chart.harvested.length ? '<span>' + chart.harvested.length + ' harvested, hollow</span>' : '') +
      '<span>shape: CPU (see the CPU toggles)</span>';
  }

  function drawFamily(family, chart, x, y, root) {
    var stroke = COLOUR[family.name];
    var unit = state.unit;
    var points = family.points.map(function (p) {
      return Object.assign({}, p, { v95: set.valueIn(p, 'p95', unit), v99: set.valueIn(p, 'p99', unit),
        vEstimate: set.valueIn(p, 'predicted', unit) });
    });
    view.segments(points, chart.positions, 'v95').forEach(function (run) {
      var banded = run.filter(function (p) { return p.v99 !== null; });
      if (banded.length > 1) {
        var edge = banded.map(function (p) { return x(p) + ',' + y(p.v99); })
          .concat(banded.slice().reverse().map(function (p) { return x(p) + ',' + y(p.v95); }));
        el('polygon', { class: 'band', fill: stroke, points: edge.join(' ') }, root);
      }
      if (run.length > 1) polyline(run, x, function (p) { return y(p.v95); }, { class: 'line', stroke: stroke }, root);
    });
    view.segments(points, chart.positions, 'vEstimate').forEach(function (run) {
      if (run.length > 1) {
        polyline(run, x, function (p) { return y(p.vEstimate); }, { class: 'line estimate', stroke: stroke }, root);
      }
    });
    points.forEach(function (p) {
      if (p.vEstimate !== null) {
        el('circle', { class: 'estimate-point', cx: x(p), cy: y(p.vEstimate), r: 2, fill: stroke }, root);
      }
      if (p.v95 === null) return;
      if (p.ringed) el('circle', { class: 'ring', cx: x(p), cy: y(p.v95), r: 7, stroke: stroke }, root);
      var dot = mark(p.cpu, x(p), y(p.v95), { class: 'point', stroke: stroke, fill: stroke }, root);
      el('title', {}, dot).textContent = describe(family, p);
      dot.addEventListener('click', function () {
        state.details = { family: family.name, position: p.position };
        drawDetails();
      });
    });
  }

  // A point's detail panel: that commit's fitted curve over size, in ms, with its grid points
  // and the measured 24,576 (its mean, the measure the fit is checked against).
  function drawDetails() {
    var want = state.details;
    var panel = $('details-25k');
    var family = want && set.chart(state.bundle).families.find(function (f) { return f.name === want.family; });
    var p = family && family.points.find(function (q) { return q.position === want.position; });
    if (!p) { panel.hidden = true; return; }
    panel.hidden = false;
    $('details-25k-title').textContent = family.label + ' at ' + short(p.sha) + (p.title ? ': ' + p.title : '') +
      ', on ' + p.cpu;
    var target = $('details-25k-chart');
    target.textContent = '';
    var width = 480;
    var height = 240;
    var pad = { left: 56, right: 16, top: 16, bottom: 36 };
    var top = Math.max.apply(null, [p.tick, p.predicted || 0].concat(p.grid.map(function (g) { return g[1]; }))) * 1.15;
    var x = function (side) { return pad.left + side / set.CHECK * (width - pad.left - pad.right); };
    var y = function (ms) { return pad.top + (1 - ms / top) * (height - pad.top - pad.bottom); };
    var root = el('svg', { width: width, height: height, viewBox: '0 0 ' + width + ' ' + height, role: 'img' }, target);
    for (var g = 0; g <= 4; g++) {
      el('line', { class: 'grid', x1: pad.left, x2: width - pad.right, y1: y(top * g / 4), y2: y(top * g / 4) }, root);
      el('text', { x: pad.left - 6, y: y(top * g / 4) + 4, 'text-anchor': 'end' }, root).textContent = (top * g / 4).toFixed(0);
    }
    [1024, 4096, 8192, 12288, 24576].forEach(function (side) {
      el('text', { x: x(side), y: height - pad.bottom + 14, 'text-anchor': 'middle' }, root).textContent = side;
    });
    el('text', { x: 4, y: 12 }, root).textContent = 'ms';
    el('text', { x: pad.left, y: height - 4 }, root).textContent = 'side (units a side, mirrored) →';
    var stroke = COLOUR[family.name];
    if (p.b !== null) {
      var curve = [];
      for (var k = 0; k <= 48; k++) {
        var side = set.CHECK * k / 48;
        curve.push(x(side) + ',' + y(set.predict(p.b, p.c, side)));
      }
      el('polyline', { class: 'line estimate', stroke: stroke, points: curve.join(' ') }, root);
    }
    p.grid.forEach(function (point) {
      el('title', {}, el('circle', { class: 'point', cx: x(point[0]), cy: y(point[1]), r: 3, stroke: stroke, fill: stroke }, root))
        .textContent = 'grid ' + point[0] + ': ' + point[1].toFixed(2) + ' ms (mean)';
    });
    el('title', {}, el('circle', { class: 'ring', cx: x(set.CHECK), cy: y(p.tick), r: 5, stroke: stroke }, root))
      .textContent = 'measured 24,576: ' + p.tick.toFixed(2) + ' ms (mean)';
    $('details-25k-note').textContent = p.b === null
      ? 'No fit: fewer than two grid sizes timed at this commit.'
      : 'Fit (Decision 165): b = ' + p.b.toExponential(3) + ' ms a unit, c = ' + p.c.toExponential(3) +
        ' ms a unit²; estimate at 24,576 ' + p.predicted.toFixed(2) + ' ms against ' + p.tick.toFixed(2) +
        ' ms measured, a miss of ' + (p.miss >= 0 ? '+' : '') + percent(p.miss) + '. Filled: the grid; ring: the measured 24,576.';
  }

  // Draws the chart, with the page's toggles: `hidden` CPUs, `harvested`, and `cpus`, every
  // CPU on the page, which the marks are shared across.
  function draw(bundle, options) {
    state.bundle = bundle;
    state.options = options || {};
    var unit = $('unit-25k');
    unit.checked = state.unit === 'bridge';
    unit.onchange = function () { state.unit = unit.checked ? 'bridge' : 'ms'; drawChart(); };
    drawChart();
    drawDetails();
  }

  window.BreachDraw25k = draw;
  window.BreachMetrics25kPage = { swatch: swatch, mark: markPath };
})();
