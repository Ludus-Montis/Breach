// The site shell's routing, site.js (specs/44-project-site-and-showcase.md, step 4,
// Decision 166): a tab from `#name`, the root and unknown names opening the first tab.
// Run: node --test "tools/site/test/*.test.js"
'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const site = require('../site.js');

const TABS = [
  { name: 'metrics', title: 'Metrics', page: 'metrics/index.html' },
  { name: 'showcase', title: 'Showcase', page: 'showcase/index.html' },
];

test('the root URL opens Metrics', () => {
  assert.equal(site.route('', TABS).name, 'metrics');
  assert.equal(site.route('#', TABS).name, 'metrics');
});

test('an unknown name opens Metrics', () => {
  assert.equal(site.route('#unknown', TABS).name, 'metrics');
});

test('#showcase opens Showcase', () => {
  assert.equal(site.route('#showcase', TABS).name, 'showcase');
});

test('a name is read without its case or surrounding space', () => {
  assert.equal(site.route('#Showcase ', TABS).name, 'showcase');
});

test('the shipped list opens Metrics first, from its own folder', () => {
  assert.equal(site.TABS[0].name, 'metrics');
  assert.equal(site.TABS[0].page, 'metrics/index.html');
  assert.equal(site.route('', site.TABS).name, 'metrics');
});
