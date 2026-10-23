// The planner's example structures (planner_examples.js, spec 24 round 4, spec 25): each fits
// one 64 x 64 tile and stands on the ground it names (its bearing) under the shared load paths
// (load_paths.js, the game's rule). Run: node --test "tools/designer/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
require('../planner_tools.js');
const examples = require('../planner_examples.js');
const paths = require('../load_paths.js');

const library = JSON.parse(
  fs.readFileSync(path.join(__dirname, '../../../content/terrain/terrain_library.json'), 'utf8')
);
const materials = {};
library.materials.forEach((m) => {
  materials[m.id] = { weight: m.weight, strength: m.strength, span: m.span, flows: (m.traits || {}).flows || 0 };
});

const built = {};
examples.forEach((example) => {
  const plan = example.build();
  const xs = [];
  const ys = [];
  Object.keys(plan.solid_cells).forEach((key) => { const at = key.split(',').map(Number); xs.push(at[0]); ys.push(at[1]); });
  plan.faces.forEach((face) => { xs.push(face.x); ys.push(face.y); });
  plan.dug.forEach((dug) => { xs.push(dug[0]); ys.push(dug[1]); });
  built[example.id] = {
    failed: paths.solve(plan, materials, example.bearing).failed,
    fieldsFailed: paths.solve(plan, materials, 4).failed.length,
    minXY: Math.min(...xs, ...ys),
    maxXY: Math.max(...xs, ...ys)
  };
});

test('there are four examples', () => {
  assert.deepEqual(Object.keys(built).sort(), ['castle', 'farmhouse', 'fort', 'tower']);
});

test('every example stands on its ground', () => {
  for (const [id, example] of Object.entries(built)) assert.deepEqual(example.failed, [], id);
});

test('the stone examples would sink on fields', () => {
  for (const id of ['tower', 'castle']) assert.ok(built[id].fieldsFailed > 0, id);
});

test('every example fits one tile', () => {
  for (const [id, example] of Object.entries(built)) {
    assert.ok(example.minXY >= 0, id);
    assert.ok(example.maxXY <= 64, id); // a north or west face may sit on edge 64
  }
});
