// The repeats' drawing (specs/47-tick-inspection.md, step 5b): metrics_inspect.js's panels as
// small multiples, one per top-level phase and size, along main's first-parent line. Each
// inspection tick's repeats are dots, the whole tick's lap of the same commit (the 25k set's
// mean tick) a line, and the panel's title gives the gap at its latest commit with both. A
// panel holding a phase the repeats under-report (Q47) is marked. metrics_page.js hands it the
// bundle and the page's hidden CPUs.
(function () {
  'use strict';
  var inspect = window.BreachMetricsInspect;
  var NS = 'http://www.w3.org/2000/svg';
  var WIDTH = 260;
  var HEIGHT = 150;
  var PAD = { left: 44, right: 8, top: 22, bottom: 20 };
  var UNIT_NAME = { percent: '% of tick', reference: '× the reference', ms: 'ms' };
  var state = { bundle: [], hidden: {}, unit: inspect.DEFAULT_UNIT };

  function $(id) { return document.getElementById(id); }

  function el(name, attrs, parent) {
    var node = document.createElementNS(NS, name);
    Object.keys(attrs || {}).forEach(function (key) { node.setAttribute(key, attrs[key]); });
    if (parent) parent.appendChild(node);
    return node;
  }

  function short(sha) { return String(sha).slice(0, 7); }

  function format(value) {
    var digits = value === 0 ? '0' : Number(value.toPrecision(2)).toString();
    if (state.unit === 'percent') return digits + '%';
    if (state.unit === 'reference') return '×' + digits;
    return digits + ' ms';
  }

  function name(phase) { return phase.replace(/^tick\//, ''); }

  function drawPanel(panel, positions, parent) {
    var box = document.createElement('figure');
    box.className = 'panel' + (panel.caveat ? ' caveat' : '');
    parent.appendChild(box);
    var caption = document.createElement('figcaption');
    caption.textContent = name(panel.phase) + (panel.caveat ? ' ⚠' : '') +
      (panel.gap ? ', repeats ×' + panel.gap.ratio.toFixed(2) + ' the lap' : '');
    caption.title = panel.caveat
      ? 'Q47: the repeats under-report this phase, whose cost builds up across ticks. The whole tick is the judge.'
      : (panel.gap ? 'At ' + short(panel.gap.sha) + ': the repeats\' median over the whole tick\'s lap.' : '');
    box.appendChild(caption);
    var all = panel.repeats.concat(panel.laps);
    var top = Math.max.apply(null, all.map(function (q) { return q.value; })) * 1.15 || 1;
    var span = Math.max(1, positions.length - 1);
    var x = function (position) {
      return PAD.left + positions.indexOf(position) / span * (WIDTH - PAD.left - PAD.right);
    };
    var y = function (value) { return PAD.top + (1 - value / top) * (HEIGHT - PAD.top - PAD.bottom); };
    var root = el('svg', { width: WIDTH, height: HEIGHT, viewBox: '0 0 ' + WIDTH + ' ' + HEIGHT, role: 'img',
      'aria-label': name(panel.phase) }, box);
    [0, 0.5, 1].forEach(function (f) {
      el('line', { class: 'grid', x1: PAD.left, x2: WIDTH - PAD.right, y1: y(top * f), y2: y(top * f) }, root);
      el('text', { x: PAD.left - 4, y: y(top * f) + 4, 'text-anchor': 'end' }, root).textContent = format(top * f);
    });
    if (panel.laps.length > 1) {
      el('polyline', { class: 'line', stroke: 'var(--s1)',
        points: panel.laps.map(function (q) { return x(q.position) + ',' + y(q.value); }).join(' ') }, root);
    }
    panel.laps.forEach(function (q) {
      el('title', {}, el('rect', { class: 'point lap', x: x(q.position) - 3, y: y(q.value) - 3, width: 6, height: 6,
        fill: 'var(--s1)' }, root)).textContent = 'whole tick at ' + short(q.sha) + (q.title ? ': ' + q.title : '') +
        '\nthe settled series\' mean: ' + format(q.value) + '\nCPU: ' + q.cpu;
    });
    panel.repeats.forEach(function (q) {
      el('title', {}, el('circle', { class: 'point repeat', cx: x(q.position), cy: y(q.value), r: 2.5,
        fill: 'var(--s2)', stroke: 'none' }, root)).textContent = 'repeats of tick ' + q.at + ' at ' + short(q.sha) +
        (q.title ? ': ' + q.title : '') + '\n' + format(q.value) + '\nCPU: ' + q.cpu;
    });
  }

  function drawChart() {
    var target = $('inspect');
    target.textContent = '';
    var chart = inspect.chart(state.bundle, state.unit, state.hidden);
    $('inspect-unit-name').textContent = UNIT_NAME[state.unit];
    if (!chart.sizes.length) {
      $('inspect-note').textContent = 'No repeat records yet: bench-inspect writes them at each merge to main ' +
        'that touches native/ or sim/ (spec 47 step 5a).';
      return;
    }
    chart.sizes.forEach(function (size) {
      var heading = document.createElement('h3');
      heading.textContent = size.label + ', ticks ' + size.ticks.join(', ');
      target.appendChild(heading);
      var seen = {};
      size.panels.forEach(function (panel) {
        panel.repeats.concat(panel.laps).forEach(function (q) { seen[q.position] = true; });
      });
      var positions = Object.keys(seen).map(Number).sort(function (a, b) { return a - b; });
      var grid = document.createElement('div');
      grid.className = 'multiples';
      target.appendChild(grid);
      size.panels.forEach(function (panel) { drawPanel(panel, positions, grid); });
    });
    $('inspect-note').textContent = state.unit === 'reference'
      ? 'In × the reference the two sides have different yardsticks (a repeat\'s is the bridge on bench\'s ' +
        'battle, a lap\'s the bridge on its own shape): read each side\'s trend, not the gap.'
      : state.unit === 'ms' ? 'In ms the gap holds the two jobs\' runners as well as the code.' : '';
  }

  // Draws the panels, with the page's `hidden` CPUs ({cpu: true}).
  function draw(bundle, options) {
    state.bundle = bundle;
    state.hidden = (options || {}).hidden || {};
    var unit = $('inspect-unit');
    unit.value = state.unit;
    unit.onchange = function () { state.unit = unit.value; drawChart(); };
    drawChart();
  }

  window.BreachDrawInspect = draw;
})();
