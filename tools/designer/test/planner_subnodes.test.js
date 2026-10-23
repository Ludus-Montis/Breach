// A node's subnodes in the planner (planner_subnodes.js, spec 26 round 1, Decision 72): default
// areas clipped to the node and round other zones, capture cells always in their zone,
// refusals, removal, and undo across plan and subnodes.
// Run: node --test "tools/designer/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const s = require('../planner_subnodes.js');
const t = require('../planner_tools.js');

// One tile, cells 0..63 each way.
const ctx = { inFootprint: (x, y) => x >= 0 && y >= 0 && x < 64 && y < 64 };
const subs = [];
const wellId = s.place(subs, 'WELL', 20, 20, ctx);
const well = subs[0];
const wellSizes = [well.capture.length, well.zone.length];
const cornerId = s.place(subs, 'OBJECTIVE', 1, 1, ctx);
const cornerSizes = [subs[1].capture.length, subs[1].zone.length];
const insideOther = s.place(subs, 'KEEP', 22, 22, ctx);
const offNode = s.place(subs, 'KEEP', 70, 5, ctx);
const nearId = s.place(subs, 'ORE_VEIN', 30, 20, ctx);
const vein = subs[2];
const veinOverlaps = vein.zone.some((c) => s.has(well.zone, c[0], c[1]));
const secondWell = s.place(subs, 'WELL', 50, 50, ctx);

test('placing gives default square areas', () => {
  assert.equal(wellId, 'well_1');
  assert.deepEqual(wellSizes, [9, 121]);
});

test('default areas are clipped to the node', () => {
  assert.equal(cornerId, 'objective_1');
  assert.deepEqual(cornerSizes, [16, 100]); // the 4x4 fits; the 16x16 is cut to 10x10
});

test('placing is refused inside another zone or off the node', () => {
  assert.equal(insideOther, null);
  assert.equal(offNode, null);
});

test('default zones stop at another zone, and ids count per type', () => {
  assert.equal(nearId, 'ore_vein_1');
  assert.equal(veinOverlaps, false);
  assert.equal(secondWell, 'well_2');
});

test('painting keeps capture cells in their zone and off other zones and the node', () => {
  assert.equal(s.paint(subs, 'well_1', 'capture', 40, 40, true, ctx), true);
  assert.equal(s.has(well.zone, 40, 40), true); // a capture cell joins the zone
  s.paint(subs, 'well_1', 'zone', 20, 21, false, ctx);
  assert.deepEqual([s.has(well.zone, 20, 21), s.has(well.capture, 20, 21)], [false, false]);
  s.paint(subs, 'well_1', 'zone', 20, 20, false, ctx);
  assert.equal(s.has(well.zone, 20, 20), true); // the marker stays
  assert.equal(s.paint(subs, 'well_1', 'zone', 30, 20, true, ctx), false);
  assert.equal(s.paint(subs, 'well_1', 'zone', -1, 5, true, ctx), false);
  assert.equal(s.owner(subs, 30, 20).id, 'ore_vein_1');
});

test('an empty capture area is a problem, and a subnode can be removed', () => {
  const empty = { id: 'objective_9', type: 'OBJECTIVE', at: [60, 60], capture: [], zone: [[60, 60]] };
  assert.deepEqual(s.problems([empty]), ["subnode 'objective_9': its capture area is empty"]);
  assert.ok(s.remove(subs, 'ore_vein_1'));
  assert.deepEqual(subs.map((x) => x.id), ['well_1', 'objective_1', 'well_2']);
});

test('undo covers the plan and the subnodes only', () => {
  const node = { plan: t.emptyPlan(), subnodes: [], other: 'kept' };
  const history = new t.History(['plan', 'subnodes']);
  history.record(node);
  s.place(node.subnodes, 'WELL', 10, 10, ctx);
  node.plan.solid_cells['10,10,0'] = 'ROCK';
  node.other = 'changed';
  assert.ok(history.undo(node));
  assert.deepEqual([node.subnodes.length, Object.keys(node.plan.solid_cells).length, node.other], [0, 0, 'changed']);
  assert.ok(history.redo(node));
  assert.deepEqual([node.subnodes.length, node.plan.solid_cells['10,10,0']], [1, 'ROCK']);
});
