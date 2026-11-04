// Load paths through a structure plan (Decisions 61, 65, spec 24), for the designer's
// live load overlay. This is the same rule as the crate's `structure` module
// (native/rust/src/structure.rs and structure/supports.rs), line for line; both run the
// shared cases in tests/fixtures/structure/load_cases.json, so they cannot drift apart.
//
// A plan: { solid_cells: {"x,y,level": material}, faces: [{x, y, level, side, material,
// thickness}] (side: north, south, east, west or floor; thickness in eighths),
// dug: [[x, y, level]], fills: {"x,y,level": material} (Decision 67), loads: {"x,y,level":
// load units} }. Materials: { id: {weight, strength, span, flows} }. A dug cell filled
// with a solid is ground again; a liquid fill (flows > 0) holds nothing up.
(function (root) {
  'use strict';

  var SIDES = ['north', 'west', 'floor'];

  // A face's one key: south and east are stored as the neighbour's north and west.
  function faceKey(x, y, level, side) {
    if (side === 'south') { y += 1; side = 'north'; }
    if (side === 'east') { x += 1; side = 'west'; }
    return 'face:' + x + ',' + y + ',' + level + ',' + side;
  }
  function cellKey(x, y, level) { return 'cell:' + x + ',' + y + ',' + level; }
  function parse(text) { return text.split(',').map(Number); }

  function Supports(plan, materials) {
    this.cells = {};
    this.faces = {};
    this.dug = {};
    var self = this;
    Object.keys(plan.solid_cells || {}).forEach(function (at) {
      var p = parse(at);
      self.cells[cellKey(p[0], p[1], p[2])] = { at: p, kind: 'cell', material: plan.solid_cells[at], eighths: 8 };
    });
    (plan.faces || []).forEach(function (f) {
      var key = faceKey(f.x, f.y, f.level, f.side);
      var at = parse(key.split(':')[1]);
      self.faces[key] = { at: [at[0], at[1], at[2]], kind: key.split(',')[3], material: f.material, eighths: f.thickness };
    });
    (plan.dug || []).forEach(function (d) {
      var fill = (plan.fills || {})[d.join(',')], m = fill && materials ? materials[fill] : null;
      if (!m || m.flows > 0) self.dug[d.join(',')] = true;
    });
  }
  Supports.prototype.elements = function () {
    var out = {}, self = this;
    Object.keys(this.cells).forEach(function (k) { out[k] = self.cells[k]; });
    Object.keys(this.faces).forEach(function (k) { out[k] = self.faces[k]; });
    return out;
  };
  Supports.prototype.cell = function (x, y, l) { var k = cellKey(x, y, l); return this.cells[k] ? k : ''; };
  Supports.prototype.face = function (x, y, l, side) { var k = faceKey(x, y, l, side); return this.faces[k] ? k : ''; };
  Supports.prototype.ground = function (at, other) {
    if (at[2] !== 0) return '';
    var columns = [at, other];
    for (var i = 0; i < 2; i++) {
      if (!this.dug[[columns[i][0], columns[i][1], -1].join(',')]) return 'ground:' + columns[i][0] + ',' + columns[i][1];
    }
    return '';
  };
  function first(keys) { for (var i = 0; i < keys.length; i++) if (keys[i]) return [keys[i]]; return []; }
  Supports.prototype.direct = function (e) {
    var x = e.at[0], y = e.at[1], l = e.at[2];
    if (e.kind === 'cell') return first([this.cell(x, y, l - 1), this.face(x, y, l, 'floor'), this.ground(e.at, e.at)]);
    if (e.kind === 'floor') {
      var walls = [this.face(x, y, l - 1, 'north'), this.face(x, y + 1, l - 1, 'north'), this.face(x, y, l - 1, 'west'),
        this.face(x + 1, y, l - 1, 'west')].filter(function (k) { return k; });
      if (this.cell(x, y, l - 1)) return [this.cell(x, y, l - 1)];
      return walls.length ? walls : first([this.ground(e.at, e.at)]);
    }
    var beside = e.kind === 'north' ? [x, y - 1, l] : [x - 1, y, l];
    if (this.face(x, y, l - 1, e.kind)) return [this.face(x, y, l - 1, e.kind)];
    var cells = [this.cell(x, y, l - 1), this.cell(beside[0], beside[1], l - 1)].filter(function (k) { return k; });
    return cells.length ? cells : first([this.ground(e.at, beside)]);
  };
  Supports.prototype.neighbours = function (e) {
    var steps = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    if (e.kind === 'north') steps = steps.slice(0, 2);
    else if (e.kind === 'west') steps = steps.slice(2, 4);
    var out = [], self = this;
    steps.forEach(function (s) {
      var x = e.at[0] + s[0], y = e.at[1] + s[1], l = e.at[2];
      var k = e.kind === 'cell' ? self.cell(x, y, l) : self.face(x, y, l, e.kind);
      if (k) out.push(k);
    });
    return out;
  };
  Supports.prototype.loadTarget = function (at) {
    var found = first([this.face(at[0], at[1], at[2], 'floor'), this.cell(at[0], at[1], at[2] - 1), this.ground(at, at)]);
    return found.length ? found[0] : '';
  };

  function nearestHeld(start, elements, held, supports, reach) {
    var seen = {}; seen[start] = true;
    var frontier = [start];
    for (var step = 0; step < reach; step++) {
      var next = [], found = [];
      frontier.forEach(function (key) {
        supports.neighbours(elements[key]).forEach(function (n) {
          if (seen[n]) return;
          seen[n] = true;
          next.push(n);
          if (held[n].length && !hangs(held, n, elements)) found.push(n);
        });
      });
      if (found.length) return found.sort();
      frontier = next;
    }
    return [];
  }
  function hangs(held, key, elements) {
    return held[key].some(function (t) { return elements[t] && elements[t].at[2] === elements[key].at[2]; });
  }

  // {loads: {key: units}, failed: [keys, in the order found]}.
  // The bearing under a "ground:x,y" column: bearing is a number, a function (x, y), or
  // {default, columns: {"x,y": n}} for a plan crossing tiles of different ground (Decision 68).
  function columnBearing(bearing, groundKey) {
    var p = groundKey.slice(7).split(',').map(Number);
    if (typeof bearing === 'function') return bearing(p[0], p[1]);
    if (bearing && typeof bearing === 'object') { var c = (bearing.columns || {})[p[0] + ',' + p[1]]; return c != null ? c : bearing.default || 0; }
    return bearing;
  }
  function solve(plan, materials, bearing) {
    var supports = new Supports(plan, materials);
    var elements = supports.elements();
    var keys = Object.keys(elements);
    var held = {}, failed = [], bridged = {};
    keys.forEach(function (k) { held[k] = supports.direct(elements[k]); });
    keys.forEach(function (k) {
      if (held[k].length) return;
      var m = materials[elements[k].material];
      var anchors = nearestHeld(k, elements, held, supports, m ? m.span : 0);
      if (!anchors.length) failed.push(k);
      held[k] = anchors;
      bridged[k] = true;
    });
    var loads = {};
    keys.forEach(function (k) { var m = materials[elements[k].material]; loads[k] = (m ? m.weight : 0) * elements[k].eighths; });
    Object.keys(plan.loads || {}).forEach(function (at) {
      var target = supports.loadTarget(parse(at));
      if (target) loads[target] = (loads[target] || 0) + Number(plan.loads[at]);
    });
    var ordered = keys.slice().sort(function (a, b) {
      var la = elements[a].at[2], lb = elements[b].at[2];
      if (la !== lb) return lb - la;
      if (!!bridged[a] !== !!bridged[b]) return bridged[a] ? -1 : 1;
      return a < b ? -1 : a > b ? 1 : 0;
    });
    ordered.forEach(function (k) {
      var m = materials[elements[k].material];
      var capacity = (m ? m.strength : 0) * elements[k].eighths;
      if (loads[k] > capacity && failed.indexOf(k) === -1) failed.push(k);
      var targets = held[k].slice().sort();
      if (!targets.length) return;
      var share = Math.floor(loads[k] / targets.length), rest = loads[k] % targets.length;
      targets.forEach(function (t, i) { loads[t] = (loads[t] || 0) + share + (i === 0 ? rest : 0); });
    });
    keys.forEach(function (k) {
      held[k].forEach(function (t) {
        if (t.indexOf('ground:') === 0 && loads[t] > columnBearing(bearing, t) * 8 && failed.indexOf(k) === -1) failed.push(k);
      });
    });
    return { loads: loads, failed: failed, capacities: capacities(elements, materials) };
  }

  function capacities(elements, materials) {
    var out = {};
    Object.keys(elements).forEach(function (k) { var m = materials[elements[k].material]; out[k] = (m ? m.strength : 0) * elements[k].eighths; });
    return out;
  }

  // Every element that fails as each failure brings down what it held (on a copy).
  function settle(plan, materials, bearing) {
    var working = JSON.parse(JSON.stringify(plan)), fallen = [];
    for (;;) {
      var failed = solve(working, materials, bearing).failed;
      if (!failed.length) return fallen;
      fallen = fallen.concat(failed);
      failed.forEach(function (k) {
        if (k.indexOf('cell:') === 0) delete working.solid_cells[k.slice(5)];
      });
      working.faces = (working.faces || []).filter(function (f) { return failed.indexOf(faceKey(f.x, f.y, f.level, f.side)) === -1; });
    }
  }

  var api = { solve: solve, settle: settle, faceKey: faceKey, cellKey: cellKey, SIDES: SIDES };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  else root.BreachLoadPaths = api;
})(this);
