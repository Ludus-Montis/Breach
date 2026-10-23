// The structure planner's editing rules (planner_tools.js, spec 24 round 3, Decision 67): edge
// picking that keeps to one line, flush walls that grow into the cell drawn from, digs that
// must connect, fills, rooms, and undo. Run: node --test "tools/designer/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const t = require('../planner_tools.js');

const ROOM = {
  height: 2,
  wall: { material: 'TIMBER', thickness: 1 },
  floor: { on: true, material: 'TIMBER', thickness: 2 },
  ceiling: { on: true, material: 'TIMBER', thickness: 8 }
};

test('the nearest grid line picks the edge', () => {
  assert.equal(t.nearestEdge(2.5, 3.05).side, 'north');
  const south = t.nearestEdge(2.5, 3.95);
  assert.deepEqual([south.side, south.index], ['south', 4]);
  assert.equal(t.nearestEdge(2.97, 3.5).side, 'east');
});

test('a stroke keeps to its line and side', () => {
  assert.deepEqual(t.edgeOnLine(5.5, 4.6, { axis: 'h', index: 4, before: true }), { x: 5, y: 3, side: 'south' });
});

test('a south wall grows back into the cell it was drawn from', () => {
  const face = t.face(2, 3, 0, 'south');
  assert.deepEqual([face.x, face.y, face.side], [2, 4, 'north']);
  assert.equal(face.into_neighbour, true);
});

test('digs must connect to the surface or a dug cell, and ground undoes a dig', () => {
  const plan = t.emptyPlan();
  assert.equal(t.canDig(plan, 1, 1, -1, 4), true);
  assert.equal(t.canDig(plan, 1, 1, -2, 4), false);
  plan.dug.push([1, 1, -1]);
  assert.equal(t.canDig(plan, 1, 1, -2, 4), true);
  assert.equal(t.canDig(plan, 1, 1, -5, 4), false);
  assert.equal(t.canDig(plan, 1, 1, 0, 4), false);
  t.fill(plan, 1, 1, -1, 'WATER');
  assert.equal(plan.fills['1,1,-1'], 'WATER');
  t.fill(plan, 1, 1, -1, 'GROUND');
  assert.deepEqual({ dug: plan.dug.length, fills: Object.keys(plan.fills).length }, { dug: 0, fills: 0 });
});

test('a room adds flush walls, a floor and a ceiling', () => {
  const room = t.emptyPlan();
  t.addRoom(room, { x0: 0, y0: 0, x1: 3, y1: 2 }, 0, ROOM);
  assert.equal(room.faces.length, 28 + 12 + 12); // 14 walls x 2 levels, floor, roof
  const south = room.faces.find((f) => f.x === 0 && f.y === 3 && f.level === 0 && f.side === 'north');
  assert.equal(south.into_neighbour, true);
  assert.equal(room.faces.filter((f) => f.side === 'floor' && f.level === 2).length, 12);
});

test('a solid cell replaces the walls and floor in it', () => {
  const solid = t.emptyPlan();
  const one = { height: 1, wall: ROOM.wall, floor: { on: true, material: 'TIMBER', thickness: 1 },
    ceiling: { on: true, material: 'TIMBER', thickness: 1 } };
  t.addRoom(solid, { x0: 2, y0: 2, x1: 2, y1: 2 }, 0, one);
  t.setSolid(solid, 2, 2, 0, 'ROCK');
  assert.equal(solid.solid_cells['2,2,0'], 'ROCK');
  assert.deepEqual(solid.faces.map((f) => f.level + ':' + f.side), ['1:floor']); // only the roof stays
});

test('an area has its cells and a ring of walls', () => {
  const ring = t.emptyPlan();
  t.perimeterWalls(ring, { x0: 1, y0: 1, x1: 3, y1: 2 }, 0, 'TIMBER', 1);
  assert.equal(ring.faces.length, 10); // 3 x 2: 2 x 3 + 2 x 2 edges
  assert.equal(t.cellsOf({ x0: 3, y0: 1, x1: 1, y1: 2 }).length, 6);
});

test('a plan from sixteen-cell tiles moves to the middle once', () => {
  const legacy = t.normalise({ solid_cells: { '2,3,0': 'ROCK' },
    faces: [{ x: 1, y: 1, level: 0, side: 'north', material: 'TIMBER', thickness: 1 }],
    dug: [[4, 4, -1]], fills: {}, loads: {} });
  assert.deepEqual(Object.keys(legacy.solid_cells), ['26,27,0']);
  assert.deepEqual([legacy.faces[0].x, legacy.faces[0].y], [25, 25]);
  assert.deepEqual(legacy.dug[0], [28, 28, -1]);
  assert.equal(legacy.tile_cells, 64);
  assert.deepEqual(Object.keys(t.normalise(legacy).solid_cells), ['26,27,0']);
});

test("edges are picked within the footprint's bounds", () => {
  const bounds = { x0: -64, y0: 0, x1: 63, y1: 63 };
  const left = t.nearestEdge(-60.5, 3.02, bounds);
  assert.deepEqual([left.x, left.side], [-61, 'north']);
  assert.equal(t.nearestEdge(-80, 3.5, bounds).x, -64);
});

test('undo and redo', () => {
  const history = new t.History();
  const plan = t.emptyPlan();
  history.record(plan);
  plan.solid_cells['0,0,0'] = 'ROCK';
  assert.ok(history.undo(plan));
  assert.equal(Object.keys(plan.solid_cells).length, 0);
  assert.ok(history.redo(plan));
  assert.equal(plan.solid_cells['0,0,0'], 'ROCK');
});
