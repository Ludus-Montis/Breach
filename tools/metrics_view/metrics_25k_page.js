// The 25k chart's drawing (specs/43-metrics-history.md, step 8c, Decision 165): metrics_25k.js's
// lines as SVG, below the 4,096 chart. 24,576 a side in ms on a log scale: each family's p95 as
// the line and p99 as a thin band above it, its projection from the grid dotted, a merge whose
// miss is beyond 10% ringed, and Decision 159's goal lines. metrics_page.js hands it the bundle.
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

  function $(id) { return document.getElementById(id); }

  function el(name, attrs, parent) {
    var node = document.createElementNS(NS, name);
    Object.keys(attrs || {}).forEach(function (key) { node.setAttribute(key, attrs[key]); });
    if (parent) parent.appendChild(node);
    return node;
  }

  function short(sha) { return String(sha).slice(0, 7); }

  function percent(value) { return (value * 100).toFixed(1) + '%'; }

  // A point's tooltip: its numbers, the runner's CPU, the miss and the quadratic share.
  function describe(family, p) {
    return family.label + ' at ' + short(p.sha) + (p.title ? ': ' + p.title : '') +
      '\nCPU: ' + p.cpu +
      '\np95 ' + (p.p95 === null ? 'gap' : p.p95.toFixed(2) + ' ms') +
      ', p99 ' + (p.p99 === null ? 'gap' : p.p99.toFixed(2) + ' ms') + ', mean ' + p.tick.toFixed(2) + ' ms' +
      (p.predicted === null ? '\nno estimate: fewer than two grid sizes timed'
        : '\nestimate ' + p.predicted.toFixed(2) + ' ms, miss ' + (p.miss >= 0 ? '+' : '') + percent(p.miss) +
          (p.ringed ? ' (beyond 10%)' : '') + '\nquadratic share c·x²/t: ' + percent(p.quadShare));
  }

  function polyline(points, x, y, attrs, root) {
    el('polyline', Object.assign({ points: points.map(function (p) { return x(p) + ',' + y(p); }).join(' ') }, attrs), root);
  }

  function draw(bundle) {
    var chart = set.chart(bundle);
    var target = $('chart-25k');
    target.textContent = '';
    var values = [];
    chart.families.forEach(function (family) {
      family.points.forEach(function (p) {
        [p.p95, p.p99, p.predicted].forEach(function (v) { if (v) values.push(v); });
      });
    });
    if (!values.length) { target.textContent = 'No 25k set recorded yet.'; return; }
    values = values.concat(GOALS.map(function (goal) { return goal.ms; }));
    var index = {};
    chart.positions.forEach(function (position, i) { index[position] = i; });
    var width = Math.max(PAD.left + PAD.right + chart.positions.length * STEP, 600);
    var root = el('svg', { width: width, height: HEIGHT, viewBox: '0 0 ' + width + ' ' + HEIGHT, role: 'img' }, target);
    var lo = Math.log10(Math.min.apply(null, values) * 0.8);
    var hi = Math.log10(Math.max.apply(null, values) * 1.2);
    var y = function (v) { return PAD.top + (hi - Math.log10(v)) / (hi - lo) * (HEIGHT - PAD.top - PAD.bottom); };
    var x = function (p) { return PAD.left + index[p.position] * STEP + STEP / 2; };
    [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000].forEach(function (v) {
      if (Math.log10(v) < lo || Math.log10(v) > hi) return;
      el('line', { class: 'grid', x1: PAD.left, x2: width - PAD.right, y1: y(v), y2: y(v) }, root);
      el('text', { x: PAD.left - 6, y: y(v) + 4, 'text-anchor': 'end' }, root).textContent = v;
    });
    el('text', { x: 4, y: 14 }, root).textContent = 'ms';
    GOALS.forEach(function (goal) {
      el('line', { class: 'goal', x1: PAD.left, x2: width - PAD.right, y1: y(goal.ms), y2: y(goal.ms) }, root);
      el('text', { x: width - PAD.right + 4, y: y(goal.ms) + 4 }, root).textContent = goal.ms + ' ms ' + goal.label;
    });
    chart.families.forEach(function (family) {
      var stroke = COLOUR[family.name];
      view.segments(family.points, chart.positions, 'p95').forEach(function (run) {
        var banded = run.filter(function (p) { return p.p99 !== null; });
        if (banded.length > 1) {
          var edge = banded.map(function (p) { return x(p) + ',' + y(p.p99); })
            .concat(banded.slice().reverse().map(function (p) { return x(p) + ',' + y(p.p95); }));
          el('polygon', { class: 'band', fill: stroke, points: edge.join(' ') }, root);
        }
        if (run.length > 1) polyline(run, x, function (p) { return y(p.p95); }, { class: 'line', stroke: stroke }, root);
      });
      view.segments(family.points, chart.positions, 'predicted').forEach(function (run) {
        if (run.length > 1) {
          polyline(run, x, function (p) { return y(p.predicted); }, { class: 'line estimate', stroke: stroke }, root);
        }
      });
      family.points.forEach(function (p) {
        if (p.predicted !== null) {
          el('circle', { class: 'estimate-point', cx: x(p), cy: y(p.predicted), r: 2, fill: stroke }, root);
        }
        if (p.p95 === null) return;
        if (p.ringed) el('circle', { class: 'ring', cx: x(p), cy: y(p.p95), r: 7, stroke: stroke }, root);
        var dot = el('circle', { class: 'point', cx: x(p), cy: y(p.p95), r: 3, stroke: stroke, fill: stroke }, root);
        el('title', {}, dot).textContent = describe(family, p);
      });
    });
    el('text', { x: PAD.left, y: HEIGHT - 8 }, root).textContent = 'commits on main, oldest first →';
    $('legend-25k').innerHTML = chart.families.map(function (family) {
      return '<span style="--swatch:' + COLOUR[family.name] + '">' + family.label + ' (' +
        family.points.length + ' commits): p95, p99 band; dotted: estimate</span>';
    }).join('');
  }

  window.BreachDraw25k = draw;
})();
