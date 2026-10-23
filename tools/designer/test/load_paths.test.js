// The designer's load_paths.js against the shared load-path cases (spec 24, round 2): the
// same cases (tests/fixtures/structure/load_cases.json) run against Godot's LoadPaths in
// tests/sim/structure/test_load_cases.gd, so the designer's overlay and the game apply one
// rule. Run: node --test "tools/designer/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const paths = require('../load_paths.js');

const fixture = JSON.parse(
  fs.readFileSync(path.join(__dirname, '../../../tests/fixtures/structure/load_cases.json'), 'utf8')
);
const materials = {};
fixture.materials.forEach((material) => { materials[material.id] = material; });

for (const shared of fixture.cases) {
  test(shared.name, () => {
    const solved = paths.solve(shared.plan, materials, shared.bearing);
    assert.deepEqual(solved.failed, shared.failed);
    for (const [key, units] of Object.entries(shared.loads || {})) {
      assert.equal(solved.loads[key], units);
    }
    if (shared.settled) {
      assert.deepEqual(paths.settle(shared.plan, materials, shared.bearing), shared.settled);
    }
  });
}
