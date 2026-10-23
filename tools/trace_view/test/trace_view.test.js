// The trace page's logic, trace_view.js (specs/37-outcome-trace.md, step 4): loading a base
// and a head trace and the diff JSON, indexing them by tick, and which units to ring.
// Run: node --test "tools/trace_view/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const view = require('../trace_view.js');

function line(tick, hp = 18) {
  const units = [
    { id: 57, hp, position: [4.5, 2.0], bearing: 90.0 },
    { id: 58, hp: 20, position: [4.5, 3.0], bearing: 90.0 }
  ];
  return { run: 'mirror_headon s1 rtrue', tick, events: [], squads: [{ id: 3, faction: 'grem', position: [4.5, 2.5], units }] };
}

/** A trace's text: one JSON line a tick. */
function trace(lines) {
  return lines.map((each) => JSON.stringify(each) + '\n').join('');
}

function range(from, to) {
  return Array.from({ length: to - from }, (_, at) => from + at);
}

test('the page indexes both traces by tick', () => {
  const ticks = range(10, 16);
  const loaded = view.load(trace(ticks.map((t) => line(t))), trace(ticks.map((t) => line(t))), null);
  assert.deepEqual(loaded.ticks, ticks);
  for (const tick of ticks) {
    for (const side of ['base', 'head']) {
      const seen = loaded.at(tick)[side];
      assert.deepEqual(seen.squads.map((s) => s.id), [3]);
      assert.deepEqual(seen.units.map((u) => u.id), [57, 58]);
      assert.deepEqual([...new Set(seen.units.map((u) => u.squad))], [3]);
      assert.deepEqual([...new Set(seen.units.map((u) => u.faction))], ['grem']);
    }
  }
});

test('the differing units are ringed', () => {
  const diff = {
    first: { tick: 12, squad: 3, unit: 57, field: 'hp' },
    ticks: [
      { tick: 12, units: [{ id: 57, squad: 3, fields: ['hp'] }], squads: [] },
      { tick: 13, units: [{ id: 58, squad: 3, only: 'base' }], squads: [] }
    ],
    only: []
  };
  const ticks = range(10, 16);
  const head = ticks.map((t) => line(t, t >= 12 ? 15 : 18));
  const loaded = view.load(trace(ticks.map((t) => line(t))), trace(head), diff);
  assert.deepEqual([11, 12, 13].map((t) => loaded.ringed(t)), [[], [57], [58]]);
});

test('a tick only one trace has', () => {
  const loaded = view.load(trace(range(10, 16).map((t) => line(t))), trace(range(10, 15).map((t) => line(t))), null);
  assert.deepEqual(loaded.ticks, range(10, 16));
  assert.deepEqual(loaded.at(15).base.units.map((u) => u.id), [57, 58]);
  assert.deepEqual(loaded.at(15).head, { squads: [], units: [] });
});

test("the selected unit's fields on both sides", () => {
  const ticks = range(10, 16);
  const loaded = view.load(trace(ticks.map((t) => line(t))), trace(ticks.map((t) => line(t, 15))), null);
  assert.equal(loaded.unit(12, 57).base.hp, 18);
  assert.equal(loaded.unit(12, 57).head.hp, 15);
  assert.deepEqual(loaded.unit(12, 99), { base: null, head: null });
});

test('infinity and NaN as the trace writes them', () => {
  const text = JSON.stringify(line(10)).replace('"bearing":90', '"bearing":NaN').replace('"hp":20', '"hp":-Infinity') + '\n';
  const loaded = view.load(text, text, null);
  assert.equal(loaded.unit(10, 57).base.bearing, 'NaN');
  assert.equal(loaded.unit(10, 58).base.hp, '-Infinity');
});
