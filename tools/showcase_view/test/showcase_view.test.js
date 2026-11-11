// The Showcase tab's logic, showcase_view.js (specs/44-project-site-and-showcase.md, step 5,
// Decisions 167 and 169): the timeline newest first, a capture's partner for compare, the
// pairing by scenario and each timing's change.
// Run: node --test "tools/showcase_view/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const view = require('../showcase_view.js');

function timings(tick, p95, phases) {
  return { rounds: 5, tick_median_ms: tick, tick_p95_ms: p95, phases_median_ms: phases };
}

function scenario(name, tick, extra = {}) {
  return {
    name,
    map: 'field',
    seed: 1,
    clip: `https://example.test/${name}.webm`,
    stills: { first: `https://example.test/${name}-first.webp` },
    moments: { first: 0, contact: 120, midpoint: 200, last: 290 },
    ticks: 291,
    seen: extra.seen || { engaging: 124 },
    not_seen: extra.not_seen || ['breaking_off', 'routing'],
    missing: extra.missing || [],
    timings: timings(tick, tick * 2, { movement: tick / 2 }),
  };
}

function capture(pr, state, date, commit, scenarios, closedAt) {
  const out = { pr, title: `PR ${pr}`, state, url: `https://github.com/x/pull/${pr}`, commit,
    date, release: 'showcase-2026-11', run: 'r', scenarios };
  if (closedAt) out.closed_at = closedAt;
  return out;
}

// manifest.py's merge: every record's captures, newest first by date.
const MANIFEST = {
  version: 1,
  captures: [
    capture(903, 'open', '2026-11-08T12:00:00Z', 'c903b', [scenario('field_break_off', 2.0), scenario('field_waves_rout', 4.0)]),
    capture(902, 'merged', '2026-11-08T11:00:00Z', 'c902a', [scenario('field_break_off', 1.0), scenario('field_ford_and_flank', 3.0)], '2026-11-08T11:30:00Z'),
    capture(903, 'open', '2026-11-08T10:00:00Z', 'c903a', [scenario('field_break_off', 2.5)]),
    capture(904, 'closed', '2026-11-08T09:30:00Z', 'c904a', [scenario('field_break_off', 1.5)], '2026-11-08T13:00:00Z'),
    capture(901, 'merged', '2026-11-08T09:00:00Z', 'c901a', [scenario('field_break_off', 0.8)], '2026-11-08T09:10:00Z'),
  ],
};

test('the timeline has one card a PR, its latest capture, newest first', () => {
  const cards = view.timeline(MANIFEST.captures);
  assert.deepEqual(cards.map((c) => [c.pr, c.commit]), [
    [903, 'c903b'], [902, 'c902a'], [904, 'c904a'], [901, 'c901a'],
  ]);
});

test('the timeline sorts by date even when the manifest does not', () => {
  const shuffled = [MANIFEST.captures[4], MANIFEST.captures[0], MANIFEST.captures[1]];
  assert.deepEqual(view.timeline(shuffled).map((c) => c.pr), [903, 902, 901]);
});

test('open captures are marked', () => {
  assert.deepEqual(view.timeline(MANIFEST.captures).map((c) => view.isOpen(c)),
    [true, false, false, false]);
});

test('an open PR is compared with its own previous capture', () => {
  const latest = MANIFEST.captures[0];
  assert.equal(view.partner(latest, MANIFEST.captures).commit, 'c903a');
});

test('an open PR with one capture has nothing to compare with', () => {
  const only = capture(905, 'open', '2026-11-08T14:00:00Z', 'c905a', []);
  assert.equal(view.partner(only, [only, ...MANIFEST.captures]), null);
});

test('a merged PR is compared with the previous merged PR, by when each merged', () => {
  assert.equal(view.partner(MANIFEST.captures[1], MANIFEST.captures).pr, 901);
  assert.equal(view.partner(MANIFEST.captures[4], MANIFEST.captures), null);
});

test('a PR closed unmerged is compared with the PR merged before it closed', () => {
  assert.equal(view.partner(MANIFEST.captures[3], MANIFEST.captures).pr, 902);
});

test('compare pairs scenarios by name, a scenario in one capture alone', () => {
  const pairs = view.pairs(MANIFEST.captures[0], MANIFEST.captures[1]);
  assert.deepEqual(pairs.map((p) => [p.name, Boolean(p.current), Boolean(p.previous)]), [
    ['field_break_off', true, true],
    ['field_waves_rout', true, false],
    ['field_ford_and_flank', false, true],
  ]);
});

test('each timing carries its change in per cent from the capture before', () => {
  const rows = view.timingRows(MANIFEST.captures[0].scenarios[0].timings,
    MANIFEST.captures[1].scenarios[0].timings);
  assert.deepEqual(rows.map((r) => [r.key, r.value, r.before, r.change]), [
    ['tick_median_ms', 2.0, 1.0, 100],
    ['tick_p95_ms', 4.0, 2.0, 100],
    ['movement', 1.0, 0.5, 100],
  ]);
});

test('a timing with nothing before it has no change', () => {
  const rows = view.timingRows(timings(2.0, 3.0, {}), null);
  assert.deepEqual(rows.map((r) => [r.key, r.change]), [['tick_median_ms', null], ['tick_p95_ms', null]]);
  assert.equal(view.change(1.5, 0), null);
  assert.equal(view.change(0.9, 1.2), -25);
});

test('behaviours: seen with their tick, then not seen, the missing ones flagged', () => {
  const made = scenario('field_break_off', 1.0, {
    seen: { engaging: 124 }, not_seen: ['routing', 'breaking_off'], missing: ['breaking_off'],
  });
  assert.deepEqual(view.behaviours(made), [
    { name: 'engaging', tick: 124, missing: false },
    { name: 'breaking_off', tick: null, missing: true },
    { name: 'routing', tick: null, missing: false },
  ]);
});

test('the filters keep the states asked for and one scenario on request', () => {
  const cards = view.timeline(MANIFEST.captures);
  const shown = view.filter(cards, { states: ['merged', 'closed'], scenario: '' });
  assert.deepEqual(shown.map((c) => c.pr), [902, 904, 901]);
  const one = view.filter(cards, { states: ['open', 'merged', 'closed'], scenario: 'field_waves_rout' });
  assert.deepEqual(one.map((c) => c.pr), [903]);
  assert.deepEqual(view.scenarioNames(cards), ['field_break_off', 'field_waves_rout', 'field_ford_and_flank']);
});
