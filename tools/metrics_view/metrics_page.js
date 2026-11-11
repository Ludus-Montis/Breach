// The metrics page's drawing (specs/43-metrics-history.md, steps 7 and 9): loads records.json
// (or the metrics folder, picked from disk), then draws metrics_view.js's lines as SVG: the
// whole tick per era, the phases stacked, and a point's comparison with the commit before it,
// each in its chart's unit (× the reference by default), with CPU and phase toggles. In ms the
// whole-tick chart also has the 25k set's "on track" lines (step 8d); metrics_25k_page.js draws
// the 25k chart itself, with the same CPU and harvested toggles.
(function () {
  'use strict';
  var view = window.BreachMetricsView;
  var set = window.BreachMetrics25k;
  var ON_TRACK = [{ key: 'target', label: 'on track: target' }, { key: 'interim', label: 'on track: interim' }];
  var NS = 'http://www.w3.org/2000/svg';
  var COLOURS = ['--s1', '--s2', '--s3', '--s4', '--s5', '--s6', '--s7', '--s8', '--s9'];
  var HARNESS_COLOUR = { 'tick_phases.gd': '--s2', 'formation_bench.gd': '--s5', 'breach_tick_phases': '--s1' };
  var PAD = { left: 56, right: 16, top: 28, bottom: 40 };
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

  // The page's state (Decision 172): each chart's unit, and what the toggles hide.
  var state = { bundle: [], track: [], cpuList: [], all: [], bridge: null, overviewUnit: 'reference', phaseUnit: 'reference', cpus: {},
    phases: {}, harness: '', details: null };
  var UNIT_NAME = { ms: 'ms', reference: '× the reference', percent: '% of tick' };

  function format(value, unit) {
    if (unit === 'ms') return value.toFixed(2) + ' ms';
    if (unit === 'percent') return value.toFixed(1) + '%';
    return '×' + value.toFixed(3);
  }

  // A point's numbers for a tooltip: its CPU, its ms, and its ratio where it has a reference.
  function describe(p, ms) {
    var ratio = p.referenceTick && ms !== null ? ', ×' + (ms / p.referenceTick).toFixed(3) + ' the reference' : '';
    return 'CPU: ' + p.cpu + '\n' + ms.toFixed(2) + ' ms' + ratio;
  }

  function leftOutNote(count) {
    return count ? count + ' record' + (count === 1 ? '' : 's') + ' with no reference left out; "ms" shows ' +
      (count === 1 ? 'it' : 'them') + '.' : '';
  }

  // The lines with the hidden CPUs' records taken out.
  function visible(withHarvested) {
    return view.withoutCpus(view.shown(state.all, withHarvested), state.cpus);
  }

  function drawOverview() {
    var unit = state.overviewUnit;
    var converted = view.inUnit(visible($('show-harvested').checked), unit);
    var lines = converted.lines;
    var bridge = state.bridge;
    $('overview-unit-name').textContent = UNIT_NAME[unit];
    $('overview-note').textContent = leftOutNote(converted.withoutReference);
    var cols = columns(lines);
    var fresh = view.positions(lines);
    var root = svg($('overview'), Math.max(cols.width, 600));
    var values = [];
    lines.forEach(function (line) {
      line.points.concat(line.harvested).forEach(function (p) { if (p.value) values.push(p.value); });
    });
    var track = unit === 'ms' ? state.track.filter(function (p) {
      return !state.cpus[p.cpu] && cols.order.indexOf(p.position) >= 0;
    }) : [];
    track.forEach(function (p) {
      [p.target, p.interim, p.measured].forEach(function (v) { if (v) values.push(v); });
    });
    if (!values.length) { $('overview').textContent = 'No records to draw with these CPUs in this unit.'; return; }
    var lo = Math.log10(Math.min.apply(null, values) * 0.8);
    var hi = Math.log10(Math.max.apply(null, values) * 1.2);
    var y = function (v) { return PAD.top + (hi - Math.log10(v)) / (hi - lo) * (HEIGHT - PAD.top - PAD.bottom); };
    [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000].forEach(function (v) {
      if (Math.log10(v) < lo || Math.log10(v) > hi) return;
      el('line', { class: 'grid', x1: PAD.left, x2: cols.width - PAD.right, y1: y(v), y2: y(v) }, root);
      el('text', { x: PAD.left - 6, y: y(v) + 4, 'text-anchor': 'end' }, root).textContent = unit === 'ms' ? v : '×' + v;
    });
    el('text', { x: 4, y: 14 }, root).textContent = UNIT_NAME[unit];
    if (bridge && cols.order.indexOf(bridge.position) >= 0) {
      var bx = cols.x(bridge.position);
      el('line', { class: 'bridge', x1: bx, x2: bx, y1: PAD.top, y2: HEIGHT - PAD.bottom }, root);
      el('text', { x: bx + 4, y: HEIGHT - PAD.bottom + 14 }, root).textContent = 'bridge ' + short(bridge.sha);
    }
    drawOnTrack(track, fresh, cols, y, root);
    lines.forEach(function (line) {
      var stroke = colour(HARNESS_COLOUR[line.harness] || '--s9');
      view.segments(line.points, fresh, 'value').forEach(function (run) {
        if (run.length < 2) return;
        el('polyline', { class: 'line', stroke: stroke, points: run.map(function (p) {
          return cols.x(p.position) + ',' + y(p.value);
        }).join(' ') }, root);
      });
      line.points.concat(line.harvested).filter(function (p) { return p.value; }).forEach(function (p) {
        var dot = el('circle', { class: 'point', cx: cols.x(p.position), cy: y(p.value), r: p.harvested ? 3.5 : 3,
          stroke: stroke, fill: p.harvested ? 'var(--panel)' : stroke }, root);
        title(dot, line.harness + ' ' + short(p.sha) + (p.harvested ? ' (harvested ' + p.date + ')' : '') +
          '\n' + describe(p, p.tick));
        dot.addEventListener('click', function () { openDetails(line.harness, p); });
      });
    });
    el('text', { x: PAD.left, y: HEIGHT - 8 }, root).textContent = 'commits on main, oldest first →';
    $('legend').innerHTML = lines.map(function (line) {
      var harvested = line.harvested.length || line.hiddenHarvested;
      return '<span style="--swatch:' + colour(HARNESS_COLOUR[line.harness] || '--s9') + '">' +
        line.harness + ' (' + line.points.length + ' fresh' +
        (harvested ? ', ' + harvested + ' harvested' + (line.hiddenHarvested ? ' hidden' : '') : '') + ')</span>';
    }).join('') + (track.length ? '<span style="--swatch:var(--s4)">the 25k set\'s 4 lanes (' + track.length +
      ' commits), dotted: on track for Decision 159 (target, interim)</span>' : '');
    $('overview-note').textContent += unit === 'ms' || !state.track.length ? '' :
      (converted.withoutReference ? ' ' : '') + 'The "on track" lines show in ms.';
  }

  // The 25k set's "on track" lines (step 8d): per commit, the 4-lane tick at which its lanes
  // fit predicts 50 ms (and 100 ms) at 24 lanes, dotted, against the set's measured 4-lane
  // tick. They break at a commit with no 25k set.
  function drawOnTrack(track, fresh, cols, y, root) {
    var stroke = colour('--s4');
    ON_TRACK.forEach(function (line) {
      view.segments(track, fresh, line.key).forEach(function (run) {
        if (run.length < 2) return;
        el('polyline', { class: 'line on-track ' + line.key, stroke: stroke, points: run.map(function (p) {
          return cols.x(p.position) + ',' + y(p[line.key]);
        }).join(' ') }, root);
      });
    });
    view.segments(track, fresh, 'measured').forEach(function (run) {
      if (run.length < 2) return;
      el('polyline', { class: 'line', stroke: stroke, points: run.map(function (p) {
        return cols.x(p.position) + ',' + y(p.measured);
      }).join(' ') }, root);
    });
    track.forEach(function (p) {
      var cx = cols.x(p.position);
      if (p.measured) {
        title(el('circle', { class: 'point', cx: cx, cy: y(p.measured), r: 2.5, stroke: stroke, fill: stroke }, root),
          'the 25k set\'s 4 lanes at ' + short(p.sha) + ' (a series from contact)\nCPU: ' + p.cpu + '\n' +
          p.measured.toFixed(2) + ' ms; on track: ' + p.target.toFixed(2) + ' ms (target), ' +
          p.interim.toFixed(2) + ' ms (interim)');
      }
      ON_TRACK.forEach(function (line) {
        title(el('circle', { class: 'estimate-point', cx: cx, cy: y(p[line.key]), r: 1.5, fill: stroke }, root),
          line.label + ' at ' + short(p.sha) + ': ' + p[line.key].toFixed(2) + ' ms at 4 lanes predicts ' +
          set.GOALS[line.key] + ' ms at 24');
      });
    });
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

  // The phase chart's points: the selected harness's, without hidden CPUs, and in
  // × the reference only those with a reference.
  function phasePoints() {
    var line = visible(false).find(function (l) { return l.harness === state.harness; });
    var points = line ? line.points.filter(function (p) { return p.tick; }) : [];
    var kept = state.phaseUnit === 'reference' ? points.filter(function (p) { return p.referenceTick; }) : points;
    return { points: kept, leftOut: points.length - kept.length };
  }

  // The reference mark of a point in the phase chart's unit: its reference tick in ms, 1 in
  // × the reference, none in % of tick.
  function referenceMark(p, unit) {
    if (unit === 'ms') return p.referenceTick || null;
    return unit === 'reference' ? 1 : null;
  }

  function drawPhases() {
    var unit = state.phaseUnit;
    var picked = phasePoints();
    var points = picked.points;
    $('phase-note').textContent = leftOutNote(picked.leftOut);
    $('phase-unit-name').textContent = UNIT_NAME[unit];
    // The phases and their colours come from all the harness's records, so a toggle never recolours.
    var whole = state.all.find(function (l) { return l.harness === state.harness; });
    var groups = topLevel(whole ? whole.points : []).slice(0, COLOURS.length - 1);
    var colourOf = function (group) {
      var k = groups.indexOf(group);
      return colour(COLOURS[k < 0 ? groups.length : k]);
    };
    var width = Math.max(PAD.left + PAD.right + points.length * STEP * 1.5, 600);
    var root = svg($('phases'), width);
    var stacks = points.map(function (p) { return view.stack(p, groups, state.phases, unit); });
    var top = unit === 'percent' ? 100 : Math.max.apply(null, [0].concat(points.map(function (p, i) {
      var sum = stacks[i].reduce(function (total, part) { return total + part.value; }, 0);
      return Math.max(sum, referenceMark(p, unit) || 0);
    }))) * 1.1 || 1;
    var y = function (v) { return PAD.top + (1 - v / top) * (HEIGHT - PAD.top - PAD.bottom); };
    for (var g = 0; g <= 4; g++) {
      var v = top * g / 4;
      el('line', { class: 'grid', x1: PAD.left, x2: width - PAD.right, y1: y(v), y2: y(v) }, root);
      el('text', { x: PAD.left - 6, y: y(v) + 4, 'text-anchor': 'end' }, root).textContent =
        unit === 'reference' ? '×' + v.toFixed(2) : unit === 'percent' ? v.toFixed(0) + '%' : v.toFixed(0);
    }
    el('text', { x: 4, y: 14 }, root).textContent = UNIT_NAME[unit];
    if (unit === 'reference') {
      var one = el('line', { class: 'ref', x1: PAD.left, x2: width - PAD.right, y1: y(1), y2: y(1) }, root);
      title(one, 'the reference\'s whole tick, timed in the same job');
    }
    points.forEach(function (p, i) {
      var x = PAD.left + i * STEP * 1.5 + 2;
      var base = 0;
      stacks[i].forEach(function (part) {
        var bar = el('rect', { class: 'bar', x: x, width: STEP * 1.5 - 4, y: y(base + part.value),
          height: Math.max(0, y(base) - y(base + part.value)), fill: colourOf(part.group) }, root);
        var ms = part.group === 'other' ? null : view.valueAt(p.results, part.group);
        title(bar, short(p.sha) + ' ' + part.group.replace('tick/', '') + ': ' + format(part.value, unit) +
          '\n' + describe(p, ms === null ? (unit === 'ms' ? part.value : p.tick) : ms) +
          (ms === null && unit !== 'ms' ? ' (the whole tick)' : ''));
        bar.addEventListener('click', function () { openDetails(state.harness, p); });
        base += part.value;
      });
      if (unit === 'ms' && p.referenceTick) {
        var dash = el('line', { class: 'ref', x1: x - 1, x2: x + STEP * 1.5 - 3,
          y1: y(p.referenceTick), y2: y(p.referenceTick) }, root);
        title(dash, 'reference ' + short(p.reference) + ': ' + p.referenceTick.toFixed(2) + ' ms');
      }
    });
    drawPhaseLegend(groups.concat(['other']), colourOf);
  }

  // Each entry shows or hides its phase in every unit.
  function drawPhaseLegend(groups, colourOf) {
    var legend = $('phase-legend');
    legend.textContent = '';
    groups.forEach(function (group) {
      var button = document.createElement('button');
      button.type = 'button';
      button.className = state.phases[group] ? 'off' : '';
      button.setAttribute('aria-pressed', state.phases[group] ? 'false' : 'true');
      button.style.setProperty('--swatch', colourOf(group));
      button.textContent = group.replace('tick/', '');
      button.title = (state.phases[group] ? 'Show ' : 'Hide ') + group.replace('tick/', '');
      button.addEventListener('click', function () {
        if (state.phases[group]) delete state.phases[group]; else state.phases[group] = true;
        drawPhases();
      });
      legend.appendChild(button);
    });
  }

  // Every CPU on the page: the lines' and the 25k set's, harvested runs included.
  function cpuList(bundle) {
    var seen = {};
    view.cpus(state.all).concat(set.cpus(set.chart(bundle, { harvested: true }))).forEach(function (cpu) {
      seen[cpu] = true;
    });
    return Object.keys(seen).sort();
  }

  // One toggle per CPU, with its mark on the 25k chart (Decision 172).
  function drawCpus() {
    var row = $('cpus');
    row.textContent = 'CPUs: ';
    state.cpuList.forEach(function (cpu) {
      var label = document.createElement('label');
      var box = document.createElement('input');
      box.type = 'checkbox';
      box.checked = !state.cpus[cpu];
      box.addEventListener('change', function () {
        if (box.checked) delete state.cpus[cpu]; else state.cpus[cpu] = true;
        redrawAll();
      });
      label.appendChild(box);
      label.appendChild(document.createTextNode(' ' + cpu));
      if (window.BreachMetrics25kPage) label.appendChild(window.BreachMetrics25kPage.swatch(state.cpuList, cpu));
      row.appendChild(label);
    });
  }

  function cell(value, unit) {
    if (value === null) return '<td class="gap">gap</td>';
    return '<td>' + (unit === 'ms' ? value.toFixed(2) : unit === 'percent' ? value.toFixed(1) : value.toFixed(3)) + '</td>';
  }

  function openDetails(harness, p) {
    state.details = { harness: harness, sha: p.sha, harvested: p.harvested, position: p.position };
    showDetails();
  }

  // The compare, among the points the CPU toggles leave, in the phase chart's unit; a point
  // with no reference falls back to ms. A harvested point is compared with this commit's
  // fresh record; a fresh one with the commit before it on its line.
  function showDetails() {
    var want = state.details;
    if (!want) return;
    var line = visible(true).find(function (l) { return l.harness === want.harness; });
    var p = line && (want.harvested ? line.harvested : line.points).find(function (q) {
      return q.sha === want.sha && q.position === want.position;
    });
    if (!p) { $('details').hidden = true; return; }
    var unit = state.phaseUnit === 'reference' && !p.referenceTick ? 'ms' : state.phaseUnit;
    var choices = line.points.filter(function (q) { return unit !== 'reference' || q.referenceTick; });
    var index = choices.indexOf(p);
    var before = p.harvested
      ? choices.find(function (q) { return q.position === p.position; }) || null
      : index > 0 ? choices[index - 1] : null;
    $('details').hidden = false;
    $('details-title').textContent = line.harness + ' at ' + short(p.sha) + (p.title ? ': ' + p.title : '') +
      (p.harvested ? ' (harvested)' : '') + ', on ' + p.cpu +
      (before ? ', against ' + short(before.sha) + ' on ' + before.cpu : '');
    $('details-links').innerHTML = (p.pr ? '<a href="' + p.pr + '">PR</a> · ' : '') +
      '<a href="' + p.recordUrl + '">record</a>' +
      (p.ratio ? ' · ×' + p.ratio.toFixed(3) + ' the reference ' + short(p.reference) : '') +
      (before ? ' · compared with ' + (p.harvested ? 'the fresh record of ' : 'the commit before, ') +
        short(before.sha) + ', at the deepest level both timed' : '') +
      ' · in ' + UNIT_NAME[unit] + (unit !== state.phaseUnit ? ' (this record has no reference)' : '');
    var rows = view.compareIn(before || { results: {} }, p, unit);
    var head = before ? '<th>' + (p.harvested ? 'fresh' : 'before') + ' (' + UNIT_NAME[unit] + ')</th>' : '';
    $('details-table').innerHTML = '<tr><th>phase</th>' + head + '<th>this (' + UNIT_NAME[unit] + ')</th>' +
      (before ? '<th>change</th><th></th>' : '') + '</tr>' +
      rows.map(function (row) {
        if (!before) return '<tr><td>' + row.path + '</td>' + cell(row.b, unit) + '</tr>';
        var change = row.a && row.b !== null ? ((row.b - row.a) / row.a * 100).toFixed(1) + '%' : '';
        return '<tr><td>' + row.path + '</td>' + cell(row.a, unit) + cell(row.b, unit) + '<td>' + change + '</td>' +
          '<td class="finer">' + (row.finer.length ? 'finer, one side only: ' + row.finer.join(', ') : '') + '</td></tr>';
      }).join('');
  }

  function draw25k() {
    if (!window.BreachDraw25k) return;
    window.BreachDraw25k(state.bundle, { hidden: state.cpus, harvested: $('show-harvested').checked,
      cpus: state.cpuList });
  }

  function drawInspect() {
    if (window.BreachDrawInspect) window.BreachDrawInspect(state.bundle, { hidden: state.cpus });
  }

  function redrawAll() {
    draw25k();
    drawInspect();
    drawOverview();
    drawPhases();
    showDetails();
  }

  function draw(bundle) {
    var lines = view.lines(bundle);
    state.bundle = bundle;
    state.all = lines;
    state.track = set.onTrack(bundle);
    state.cpuList = cpuList(bundle);
    if (!lines.length) { draw25k(); drawInspect(); $('status').textContent = 'No bench records found.'; return; }
    var bridge = view.bridge(bundle);
    state.bridge = bridge;
    $('status').textContent = bundle.length + ' records.';
    $('bridge').textContent = bridge
      ? 'At the bridge, ' + short(bridge.sha) + ', tick_phases.gd timed ' + bridge.gdscript.toFixed(1) +
        ' ms and breach_tick_phases ' + bridge.cargo.toFixed(1) + ' ms: the cargo era is ×' +
        bridge.factor.toFixed(2) + ' faster. The two lines are never joined.'
      : '';
    drawCpus();
    $('show-harvested').onchange = function () { drawOverview(); draw25k(); };
    var overviewUnit = $('overview-unit');
    overviewUnit.value = state.overviewUnit;
    overviewUnit.onchange = function () { state.overviewUnit = overviewUnit.value; drawOverview(); };
    var phaseUnit = $('phase-unit');
    phaseUnit.value = state.phaseUnit;
    phaseUnit.onchange = function () { state.phaseUnit = phaseUnit.value; drawPhases(); showDetails(); };
    var select = $('harness');
    select.innerHTML = lines.filter(function (l) { return l.points.length; }).map(function (l) {
      return '<option>' + l.harness + '</option>';
    }).join('');
    select.value = lines.some(function (l) { return l.harness === 'breach_tick_phases'; }) ? 'breach_tick_phases' : select.value;
    state.harness = select.value;
    select.onchange = function () { state.harness = select.value; drawPhases(); };
    redrawAll();
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
