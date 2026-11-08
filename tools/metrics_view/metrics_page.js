// The metrics page's drawing (specs/43-metrics-history.md, step 7): loads records.json (or
// the metrics folder, picked from disk), then draws metrics_view.js's lines as SVG: the whole
// tick per era, the phases stacked, and a point's comparison with the commit before it.
(function () {
  'use strict';
  var view = window.BreachMetricsView;
  var NS = 'http://www.w3.org/2000/svg';
  var COLOURS = ['--s1', '--s2', '--s3', '--s4', '--s5', '--s6', '--s7', '--s8', '--s9'];
  var HARNESS_COLOUR = { 'tick_phases.gd': '--s2', 'formation_bench.gd': '--s5', 'breach_tick_phases': '--s1' };
  var PAD = { left: 56, right: 16, top: 16, bottom: 40 };
  var HEIGHT = 320;
  var STEP = 14;

  function $(id) { return document.getElementById(id); }

  function colour(name) { return 'var(' + name + ')'; }

  function el(name, attrs, parent) {
    var node = document.createElementNS(NS, name);
    Object.keys(attrs || {}).forEach(function (key) { node.setAttribute(key, attrs[key]); });
    if (parent) parent.appendChild(node);
    return node;
  }

  function svg(container, width) {
    container.textContent = '';
    return el('svg', { width: width, height: HEIGHT, viewBox: '0 0 ' + width + ' ' + HEIGHT,
      role: 'img' }, container);
  }

  function title(node, text) { el('title', {}, node).textContent = text; }

  // One column per commit on main's line that any line has, in first-parent order.
  function columns(lines) {
    var seen = {};
    lines.forEach(function (line) {
      line.points.concat(line.harvested).forEach(function (p) { seen[p.position] = true; });
    });
    var order = Object.keys(seen).map(Number).sort(function (a, b) { return a - b; });
    var index = {};
    order.forEach(function (position, i) { index[position] = i; });
    return { order: order, x: function (position) { return PAD.left + index[position] * STEP + STEP / 2; },
      width: PAD.left + PAD.right + order.length * STEP };
  }

  function short(sha) { return String(sha).slice(0, 7); }

  function drawOverview(lines, bridge) {
    var cols = columns(lines);
    var fresh = {};
    lines.forEach(function (line) { line.points.forEach(function (p) { fresh[p.position] = true; }); });
    fresh = Object.keys(fresh).map(Number);
    var root = svg($('overview'), Math.max(cols.width, 600));
    var ticks = [];
    lines.forEach(function (line) {
      line.points.concat(line.harvested).forEach(function (p) { if (p.tick) ticks.push(p.tick); });
    });
    var lo = Math.log10(Math.min.apply(null, ticks) * 0.8);
    var hi = Math.log10(Math.max.apply(null, ticks) * 1.2);
    var y = function (ms) { return PAD.top + (hi - Math.log10(ms)) / (hi - lo) * (HEIGHT - PAD.top - PAD.bottom); };
    [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000].forEach(function (ms) {
      if (Math.log10(ms) < lo || Math.log10(ms) > hi) return;
      el('line', { class: 'grid', x1: PAD.left, x2: cols.width - PAD.right, y1: y(ms), y2: y(ms) }, root);
      el('text', { x: PAD.left - 6, y: y(ms) + 4, 'text-anchor': 'end' }, root).textContent = ms;
    });
    if (bridge) {
      var bx = cols.x(bridge.position);
      el('line', { class: 'bridge', x1: bx, x2: bx, y1: PAD.top, y2: HEIGHT - PAD.bottom }, root);
      el('text', { x: bx + 4, y: HEIGHT - PAD.bottom + 14 }, root).textContent = 'bridge ' + short(bridge.sha);
    }
    lines.forEach(function (line) {
      var stroke = colour(HARNESS_COLOUR[line.harness] || '--s9');
      var drawn = line.points.filter(function (p) { return p.tick; });
      view.segments(line.points, fresh).forEach(function (run) {
        if (run.length < 2) return;
        el('polyline', { class: 'line', stroke: stroke, points: run.map(function (p) {
          return cols.x(p.position) + ',' + y(p.tick);
        }).join(' ') }, root);
      });
      drawn.concat(line.harvested.filter(function (p) { return p.tick; })).forEach(function (p) {
        var dot = el('circle', { class: 'point', cx: cols.x(p.position), cy: y(p.tick), r: p.harvested ? 3.5 : 3,
          stroke: stroke, fill: p.harvested ? 'var(--panel)' : stroke }, root);
        title(dot, line.harness + ' ' + short(p.sha) + ': ' + p.tick + ' ms' +
          (p.ratio ? ', ×' + p.ratio.toFixed(3) + ' the reference' : '') + (p.harvested ? ' (harvested ' + p.date + ')' : ''));
        dot.addEventListener('click', function () { showDetails(line, p); });
      });
    });
    el('text', { x: PAD.left, y: HEIGHT - 8 }, root).textContent = 'commits on main, oldest first →';
    $('legend').innerHTML = lines.map(function (line) {
      return '<span style="--swatch:' + colour(HARNESS_COLOUR[line.harness] || '--s9') + '">' +
        line.harness + ' (' + line.points.length + ' fresh, ' + line.harvested.length + ' harvested)</span>';
    }).join('');
  }

  function topLevel(points) {
    var totals = {};
    points.forEach(function (p) {
      Object.keys(p.results).forEach(function (path) {
        var parts = path.split('/');
        if (parts.length < 2) return;
        var group = parts.slice(0, 2).join('/');
        totals[group] = (totals[group] || 0) + (view.valueAt(p.results, group) || 0);
      });
    });
    return Object.keys(totals).sort(function (a, b) { return totals[b] - totals[a]; });
  }

  function drawPhases(line) {
    var points = line.points.filter(function (p) { return p.tick; });
    var groups = topLevel(points);
    var shown = groups.slice(0, COLOURS.length - 1);
    var width = Math.max(PAD.left + PAD.right + points.length * STEP * 1.5, 600);
    var root = svg($('phases'), width);
    var top = Math.max.apply(null, points.map(function (p) {
      return Math.max(p.tick, p.referenceTick || 0);
    })) * 1.1;
    var y = function (ms) { return PAD.top + (1 - ms / top) * (HEIGHT - PAD.top - PAD.bottom); };
    for (var g = 0; g <= 4; g++) {
      var ms = top * g / 4;
      el('line', { class: 'grid', x1: PAD.left, x2: width - PAD.right, y1: y(ms), y2: y(ms) }, root);
      el('text', { x: PAD.left - 6, y: y(ms) + 4, 'text-anchor': 'end' }, root).textContent = ms.toFixed(0);
    }
    points.forEach(function (p, i) {
      var x = PAD.left + i * STEP * 1.5 + 2;
      var base = 0;
      shown.concat(['other']).forEach(function (group, k) {
        var value = group === 'other'
          ? Math.max(0, p.tick - shown.reduce(function (sum, s) { return sum + (view.valueAt(p.results, s) || 0); }, 0))
          : view.valueAt(p.results, group) || 0;
        if (!value) return;
        var bar = el('rect', { class: 'bar', x: x, width: STEP * 1.5 - 4, y: y(base + value),
          height: Math.max(0, y(base) - y(base + value)), fill: colour(COLOURS[k]) }, root);
        title(bar, short(p.sha) + ' ' + group.replace('tick/', '') + ': ' + value.toFixed(2) + ' ms');
        bar.addEventListener('click', function () { showDetails(line, p); });
        base += value;
      });
      if (p.referenceTick) {
        var dash = el('line', { class: 'ref', x1: x - 1, x2: x + STEP * 1.5 - 3,
          y1: y(p.referenceTick), y2: y(p.referenceTick) }, root);
        title(dash, 'reference ' + short(p.reference) + ': ' + p.referenceTick.toFixed(2) + ' ms');
      }
    });
    $('phase-legend').innerHTML = shown.concat(['other']).map(function (group, k) {
      return '<span style="--swatch:' + colour(COLOURS[k]) + '">' + group.replace('tick/', '') + '</span>';
    }).join('');
  }

  function cell(value) {
    return value === null ? '<td class="gap">gap</td>' : '<td>' + value.toFixed(2) + '</td>';
  }

  function showDetails(line, p) {
    // A harvested point is compared with this commit's fresh record; a fresh one with the
    // commit before it on its line.
    var index = line.points.indexOf(p);
    var before = p.harvested
      ? line.points.find(function (q) { return q.position === p.position; }) || null
      : index > 0 ? line.points[index - 1] : null;
    $('details').hidden = false;
    $('details-title').textContent = line.harness + ' at ' + short(p.sha) + (p.title ? ': ' + p.title : '') +
      (p.harvested ? ' (harvested)' : '');
    $('details-links').innerHTML = (p.pr ? '<a href="' + p.pr + '">PR</a> · ' : '') +
      '<a href="' + p.recordUrl + '">record</a>' +
      (p.ratio ? ' · ×' + p.ratio.toFixed(3) + ' the reference ' + short(p.reference) : '') +
      (before ? ' · compared with ' + (p.harvested ? 'the fresh record of ' : 'the commit before, ') +
        short(before.sha) + ', at the deepest level both timed' : '');
    var rows = view.compare(before ? before.results : {}, p.results);
    var head = before ? '<th>' + (p.harvested ? 'fresh' : 'before') + ' (ms)</th>' : '';
    $('details-table').innerHTML = '<tr><th>phase</th>' + head + '<th>this (ms)</th>' +
      (before ? '<th>change</th><th></th>' : '') + '</tr>' +
      rows.map(function (row) {
        if (!before) return '<tr><td>' + row.path + '</td>' + cell(row.b) + '</tr>';
        var change = row.a && row.b !== null ? ((row.b - row.a) / row.a * 100).toFixed(1) + '%' : '';
        return '<tr><td>' + row.path + '</td>' + cell(row.a) + cell(row.b) + '<td>' + change + '</td>' +
          '<td class="finer">' + (row.finer.length ? 'finer, one side only: ' + row.finer.join(', ') : '') + '</td></tr>';
      }).join('');
  }

  function draw(bundle) {
    var lines = view.lines(bundle);
    if (!lines.length) { $('status').textContent = 'No bench records found.'; return; }
    var bridge = view.bridge(bundle);
    $('status').textContent = bundle.length + ' records.';
    $('bridge').textContent = bridge
      ? 'At the bridge, ' + short(bridge.sha) + ', tick_phases.gd timed ' + bridge.gdscript.toFixed(1) +
        ' ms and breach_tick_phases ' + bridge.cargo.toFixed(1) + ' ms: the cargo era is ×' +
        bridge.factor.toFixed(2) + ' faster. The two lines are never joined.'
      : '';
    drawOverview(lines, bridge);
    var select = $('harness');
    select.innerHTML = lines.filter(function (l) { return l.points.length; }).map(function (l) {
      return '<option>' + l.harness + '</option>';
    }).join('');
    select.value = lines.some(function (l) { return l.harness === 'breach_tick_phases'; }) ? 'breach_tick_phases' : select.value;
    var redraw = function () {
      drawPhases(lines.find(function (l) { return l.harness === select.value; }));
    };
    select.onchange = redraw;
    redraw();
  }

  function fromFolder(files) {
    var wanted = Array.prototype.filter.call(files, function (file) {
      var path = file.webkitRelativePath || file.name;
      return /\.json$/.test(path) && !/\/(raw|schema)\//.test(path);
    });
    return Promise.all(wanted.map(function (file) {
      return file.text().then(function (text) {
        var path = (file.webkitRelativePath || file.name).replace(/^.*?metrics\//, 'metrics/');
        return { path: path, record: JSON.parse(text) };
      });
    }));
  }

  $('folder').addEventListener('change', function (event) {
    fromFolder(event.target.files).then(draw);
  });
  fetch('records.json').then(function (response) {
    if (!response.ok) throw new Error(response.status);
    return response.json();
  }).then(draw).catch(function () {
    $('status').textContent = 'No records.json beside the page.';
    $('pick').hidden = false;
  });
})();
