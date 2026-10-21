// The outcome trace page's drawing and controls (specs/37-outcome-trace.md, step 4): two
// canvases on one scrubber, each unit a dot with its facing, coloured by its squad's
// faction, the units the diff names ringed, the selected unit's fields on both sides. The
// logic it draws from is trace_view.js's, tested in test_trace_view_js.py.
(function () {
  'use strict';

  var View = window.BreachTraceView;
  var SIDE_COLOURS = ['--side-a', '--side-b', '--side-c', '--side-d'];
  var PAD = 24;
  var STEP_MS = 100;

  var state = { texts: { base: null, head: null, diff: null }, view: null, index: 0,
    selected: null, timer: null, frame: null, factions: [] };
  var el = function (id) { return document.getElementById(id); };
  var css = function (name) {
    return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  };

  function readFile(input, key) {
    input.addEventListener('change', function () {
      var file = input.files[0];
      if (!file) return;
      file.text().then(function (text) {
        state.texts[key] = text;
        rebuild();
      });
    });
  }

  function rebuild() {
    if (state.texts.base === null && state.texts.head === null) return;
    try {
      var diff = state.texts.diff ? JSON.parse(state.texts.diff) : null;
      state.view = View.load(state.texts.base || '', state.texts.head || '', diff);
    } catch (error) {
      el('first').textContent = 'Could not read a file: ' + error.message;
      return;
    }
    state.frame = bounds(state.view);
    state.factions = factions(state.view);
    var first = state.view.diff && state.view.diff.first;
    el('first').textContent = first ? describe(first) : 'Loaded; no diff JSON, or no difference.';
    var at = first ? state.view.ticks.indexOf(first.tick) : 0;
    state.index = Math.max(0, at);
    el('scrubber').max = String(Math.max(0, state.view.ticks.length - 1));
    show();
  }

  function describe(first) {
    var parts = ['first difference: tick ' + first.tick];
    if (first.squad !== null) parts.push('squad ' + first.squad);
    if (first.unit !== null) parts.push('unit ' + first.unit);
    if (first.field) parts.push(first.field + ': ' + JSON.stringify(first.base) + ' -> ' +
      JSON.stringify(first.head));
    else if (first.only) parts.push(first.only + ' only');
    else parts.push('events differ');
    return parts.join('  ');
  }

  // One frame for both sides and every tick, so a unit that moves is seen to move.
  function bounds(view) {
    var box = { minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity };
    view.ticks.forEach(function (tick) {
      var both = view.at(tick);
      both.base.units.concat(both.head.units).forEach(function (unit) {
        var p = unit.position;
        if (!Array.isArray(p) || !isFinite(p[0]) || !isFinite(p[1])) return;
        box.minX = Math.min(box.minX, p[0]); box.maxX = Math.max(box.maxX, p[0]);
        box.minY = Math.min(box.minY, p[1]); box.maxY = Math.max(box.maxY, p[1]);
      });
    });
    if (!isFinite(box.minX)) box = { minX: 0, minY: 0, maxX: 1, maxY: 1 };
    return box;
  }

  function factions(view) {
    var seen = [];
    view.ticks.forEach(function (tick) {
      var both = view.at(tick);
      both.base.squads.concat(both.head.squads).forEach(function (squad) {
        if (seen.indexOf(squad.faction) === -1) seen.push(squad.faction);
      });
    });
    return seen.sort();
  }

  function transform(canvas) {
    var f = state.frame;
    var spanX = Math.max(f.maxX - f.minX, 1);
    var spanY = Math.max(f.maxY - f.minY, 1);
    var scale = Math.min((canvas.width - 2 * PAD) / spanX, (canvas.height - 2 * PAD) / spanY);
    var left = (canvas.width - spanX * scale) / 2; // centred
    var top = (canvas.height - spanY * scale) / 2;
    return {
      scale: scale,
      x: function (x) { return left + (x - f.minX) * scale; },
      y: function (y) { return top + (y - f.minY) * scale; },
    };
  }

  function draw(canvas, side, ringed) {
    var ctx = canvas.getContext('2d');
    ctx.fillStyle = css('--panel');
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    var t = transform(canvas);
    var radius = Math.max(2, Math.min(8, t.scale * 0.35));
    side.units.forEach(function (unit) {
      var p = unit.position;
      if (!Array.isArray(p)) return;
      var x = t.x(p[0]);
      var y = t.y(p[1]);
      var colour = css(SIDE_COLOURS[state.factions.indexOf(unit.faction) % SIDE_COLOURS.length]);
      ctx.beginPath();
      ctx.arc(x, y, radius, 0, 2 * Math.PI);
      if (typeof unit.hp === 'number' && unit.hp <= 0) {
        ctx.strokeStyle = colour;
        ctx.lineWidth = 1;
        ctx.stroke();
      } else {
        ctx.fillStyle = colour;
        ctx.fill();
      }
      if (typeof unit.bearing === 'number') {
        var angle = unit.bearing * Math.PI / 180; // 0 points to -y, clockwise (unit_motion.gd)
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.lineTo(x + Math.sin(angle) * radius * 2, y - Math.cos(angle) * radius * 2);
        ctx.strokeStyle = css('--ink');
        ctx.lineWidth = 1;
        ctx.stroke();
      }
      if (ringed.indexOf(unit.id) !== -1 || unit.id === state.selected) {
        ctx.beginPath();
        ctx.arc(x, y, radius + 3, 0, 2 * Math.PI);
        ctx.strokeStyle = unit.id === state.selected ? css('--ink') : css('--ring');
        ctx.lineWidth = 2;
        ctx.stroke();
      }
    });
  }

  function show() {
    var view = state.view;
    if (!view || !view.ticks.length) return;
    var tick = view.ticks[state.index];
    el('scrubber').value = String(state.index);
    el('tick').textContent = 'tick ' + tick;
    var both = view.at(tick);
    var ringed = view.ringed(tick);
    draw(el('baseCanvas'), both.base, ringed);
    draw(el('headCanvas'), both.head, ringed);
    fields(tick);
  }

  function fields(tick) {
    var table = el('fields');
    table.textContent = '';
    if (state.selected === null) return;
    var unit = state.view.unit(tick, state.selected);
    el('selectedTitle').textContent = 'Unit ' + state.selected + ' at tick ' + tick;
    var names = [];
    [unit.base, unit.head].forEach(function (u) {
      Object.keys(u || {}).forEach(function (n) { if (names.indexOf(n) === -1) names.push(n); });
    });
    var head = table.insertRow();
    ['field', 'base', 'head'].forEach(function (text) {
      var th = document.createElement('th');
      th.textContent = text;
      head.appendChild(th);
    });
    names.forEach(function (name) {
      var a = unit.base ? JSON.stringify(unit.base[name]) : '(none)';
      var b = unit.head ? JSON.stringify(unit.head[name]) : '(none)';
      var row = table.insertRow();
      if (a !== b) row.className = 'differs';
      [name, a, b].forEach(function (text) { row.insertCell().textContent = String(text); });
    });
  }

  // The unit nearest a click, on whichever side was clicked.
  function pick(canvas, key, event) {
    if (!state.view) return;
    var rect = canvas.getBoundingClientRect();
    var cx = (event.clientX - rect.left) * canvas.width / rect.width;
    var cy = (event.clientY - rect.top) * canvas.height / rect.height;
    var t = transform(canvas);
    var best = null;
    var bestDistance = 16 * 16;
    state.view.at(state.view.ticks[state.index])[key].units.forEach(function (unit) {
      if (!Array.isArray(unit.position)) return;
      var dx = t.x(unit.position[0]) - cx;
      var dy = t.y(unit.position[1]) - cy;
      if (dx * dx + dy * dy < bestDistance) { bestDistance = dx * dx + dy * dy; best = unit.id; }
    });
    state.selected = best;
    if (best === null) el('selectedTitle').textContent = 'Click a unit to see its fields on both sides';
    show();
  }

  function step(by) {
    if (!state.view) return;
    var last = state.view.ticks.length - 1;
    state.index = Math.min(last, Math.max(0, state.index + by));
    if (state.index === last) pause();
    show();
  }

  function pause() {
    clearInterval(state.timer);
    state.timer = null;
    el('play').textContent = 'Play';
  }

  readFile(el('baseFile'), 'base');
  readFile(el('headFile'), 'head');
  readFile(el('diffFile'), 'diff');
  el('back').addEventListener('click', function () { pause(); step(-1); });
  el('forward').addEventListener('click', function () { pause(); step(1); });
  el('play').addEventListener('click', function () {
    if (state.timer) { pause(); return; }
    if (!state.view) return;
    if (state.index >= state.view.ticks.length - 1) state.index = 0;
    el('play').textContent = 'Pause';
    state.timer = setInterval(function () { step(1); }, STEP_MS);
  });
  el('scrubber').addEventListener('input', function (event) {
    state.index = Number(event.target.value);
    show();
  });
  el('baseCanvas').addEventListener('click', function (e) { pick(el('baseCanvas'), 'base', e); });
  el('headCanvas').addEventListener('click', function (e) { pick(el('headCanvas'), 'head', e); });
})();
