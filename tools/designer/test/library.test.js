// The Library page's filtering (library.js's LibraryFilter, Decision 128): search, tag chips,
// target and used-only. Run: node --test "tools/designer/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const lib = require('../library.js');

const REGISTRY = {
  tags: [
    { id: 'behaviour', tags: 'trait', targets: ['unit', 'leader'] },
    { id: 'grem', tags: 'unit', targets: [] }
  ],
  traits: [
    { id: 'cunning', tags: ['behaviour'], targets: ['unit', 'leader'], description: 'Plays dead' },
    { id: 'siege', tags: ['combat'], targets: ['weapon'], description: 'Against structures' },
    { id: 'captor', tags: ['behaviour'], targets: ['unit', 'leader'], description: 'Captures' },
    { id: 'climber', tags: ['movement'], targets: ['unit'], description: 'Climbs', default: 0 }
  ],
  statuses: []
};
const USAGE = { siege: ['content/weapons/brute_fists.json'] };
const UNITS = [
  { id: 'goat', traits: [{ id: 'climber', level: 2, implicit: false }, { id: 'swimmer', level: 0, implicit: true }] },
  { id: 'stone', traits: [] }
];

function ids(filter) {
  return lib.select(REGISTRY, filter.kind, filter, USAGE).map((entry) => entry.id);
}

test('filters select sorted entries, and the tag choices', () => {
  assert.deepEqual(ids({ kind: 'traits' }), ['captor', 'climber', 'cunning', 'siege']);
  assert.deepEqual(ids({ kind: 'traits', search: 'PLAYS' }), ['cunning']);
  assert.deepEqual(ids({ kind: 'traits', tags: ['behaviour'] }), ['captor', 'cunning']);
  assert.deepEqual(ids({ kind: 'traits', target: 'weapon' }), ['siege']);
  assert.deepEqual(ids({ kind: 'traits', usedOnly: true }), ['siege']);
  assert.deepEqual(ids({ kind: 'tags', tags: ['unit'] }), ['grem']);
  assert.deepEqual(lib.tagChoices(REGISTRY, 'traits'), ['behaviour', 'combat', 'movement']);
  assert.deepEqual(lib.tagChoices(REGISTRY, 'tags'), ['trait', 'unit']);
});

test('an implicit trait is labelled with its default', () => {
  assert.deepEqual(REGISTRY.traits.map(lib.implicitOf), ['', '', '', 'implicit 0']);
});

test("a unit's traits read with the implicit ones labelled", () => {
  assert.deepEqual(UNITS.map(lib.traitLine), ['climber 2, swimmer 0 (implicit)', '']);
});
