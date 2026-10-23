// The Library page (Decision 128): browse, search and filter the registry's tags, traits
// and statuses (content/registry/*.json) - what each is, its tags, where a trait may sit,
// whether every unit has it implicitly (its "default" level), and the content using it -
// and the units with every trait present on each, the implicit ones labelled (spec 30) -
// from GET /api/registry. Filtering is pure (LibraryFilter), so it can be tested apart from
// the page.
(function (root) {
  'use strict';

  var KINDS = [
    { key: 'traits', label: 'Traits' },
    { key: 'tags', label: 'Tags' },
    { key: 'statuses', label: 'Statuses' },
    { key: 'units', label: 'Units' }
  ];

  // The entries of `kind` that pass `filter`: {search, tags: [ids all required], target,
  // usedOnly}; `usage` maps ids to the files using them.
  function select(registry, kind, filter, usage) {
    var words = (filter.search || '').toLowerCase().split(/\s+/).filter(Boolean);
    return (registry[kind] || []).filter(function (entry) {
      var text = (entry.id + ' ' + (entry.description || '')).toLowerCase();
      if (!words.every(function (w) { return text.indexOf(w) >= 0; })) return false;
      var own = tagsOf(entry, kind);
      if (!(filter.tags || []).every(function (t) { return own.indexOf(t) >= 0; })) return false;
      if (filter.target && (entry.targets || []).indexOf(filter.target) < 0) return false;
      if (filter.usedOnly && !(usage[entry.id] || []).length) return false;
      return true;
    }).sort(function (a, b) { return a.id < b.id ? -1 : 1; });
  }

  // The tags an entry carries: a tag entry is "tagged" by what it tags.
  function tagsOf(entry, kind) {
    return kind === 'tags' ? [entry.tags] : (entry.tags || []);
  }

  // Every tag the entries of `kind` carry, for the filter chips.
  function tagChoices(registry, kind) {
    var seen = {};
    (registry[kind] || []).forEach(function (e) { tagsOf(e, kind).forEach(function (t) { seen[t] = true; }); });
    return Object.keys(seen).sort();
  }

  // "implicit N" for a trait every unit has at level N without listing it (its registry
  // "default"), else "".
  function implicitOf(entry) {
    return entry.default != null ? 'implicit ' + entry.default : '';
  }

  // A unit's traits as a line: "climber 2, swimmer 0 (implicit)".
  function traitLine(unit) {
    return (unit.traits || []).map(function (t) {
      return t.id + ' ' + t.level + (t.implicit ? ' (implicit)' : '');
    }).join(', ');
  }

  var LibraryFilter = { select: select, tagChoices: tagChoices, tagsOf: tagsOf, implicitOf: implicitOf, traitLine: traitLine };
  if (typeof module !== 'undefined' && module.exports) { module.exports = LibraryFilter; return; }
  root.LibraryFilter = LibraryFilter;

  var state = { registry: null, kind: 'traits', tags: [], search: '', target: '', usedOnly: false };

  function el(tag, text, attrs) {
    var node = document.createElement(tag);
    if (text != null) node.textContent = text;
    Object.keys(attrs || {}).forEach(function (k) { node.setAttribute(k, attrs[k]); });
    return node;
  }

  function chip(text, on, onClick) {
    var b = el('button', text, { type: 'button', class: 'chip', 'data-on': on ? 'true' : 'false' });
    b.addEventListener('click', onClick);
    return b;
  }

  function toggleTag(tag) {
    var at = state.tags.indexOf(tag);
    if (at >= 0) state.tags.splice(at, 1); else state.tags.push(tag);
    render();
  }

  // The units, each with every trait present on it; the implicit ones dimmed and labelled.
  function renderUnits(reg) {
    var words = state.search.toLowerCase().split(/\s+/).filter(Boolean);
    var shown = (reg.units || []).filter(function (u) {
      var text = (u.id + ' ' + traitLine(u)).toLowerCase();
      return words.every(function (w) { return text.indexOf(w) >= 0; });
    });
    document.getElementById('count').textContent = shown.length + ' shown';
    var head = document.getElementById('head');
    head.innerHTML = '';
    var tr = el('tr');
    ['Id', 'Traits', 'File'].forEach(function (c) { tr.appendChild(el('th', c)); });
    head.appendChild(tr);
    var rows = document.getElementById('rows');
    rows.innerHTML = '';
    shown.forEach(function (unit) {
      var row = el('tr');
      row.appendChild(el('td', unit.id, { class: 'id' }));
      var cell = el('td');
      unit.traits.forEach(function (t) {
        var item = el('div', t.id + ' ' + t.level);
        if (t.implicit) item.appendChild(el('span', ' implicit', { class: 'implicit' }));
        cell.appendChild(item);
      });
      row.appendChild(cell);
      row.appendChild(el('td', unit.path, { class: 'used' }));
      rows.appendChild(row);
    });
  }

  function render() {
    var reg = state.registry;
    var kinds = document.getElementById('kinds');
    kinds.innerHTML = '';
    KINDS.forEach(function (k) {
      var b = el('button', k.label + ' (' + (reg[k.key] || []).length + ')', { type: 'button', class: 'view-btn', 'data-active': state.kind === k.key ? 'true' : 'false' });
      b.addEventListener('click', function () { state.kind = k.key; state.tags = []; render(); });
      kinds.appendChild(b);
    });
    var chips = document.getElementById('tagFilter');
    chips.innerHTML = '';
    tagChoices(reg, state.kind).forEach(function (t) { chips.appendChild(chip(t, state.tags.indexOf(t) >= 0, function () { toggleTag(t); })); });
    document.getElementById('target').disabled = state.kind !== 'traits';
    if (state.kind === 'units') { renderUnits(reg); return; }
    var shown = select(reg, state.kind, state, reg.usage);
    document.getElementById('count').textContent = shown.length + ' shown';
    var head = document.getElementById('head');
    head.innerHTML = '';
    var cols = ['Id', 'Description', state.kind === 'tags' ? 'Tags' : 'Tags', state.kind === 'tags' ? 'Reaches' : 'May sit on', 'Used in'];
    var tr = el('tr');
    cols.forEach(function (c) { tr.appendChild(el('th', c)); });
    head.appendChild(tr);
    var rows = document.getElementById('rows');
    rows.innerHTML = '';
    shown.forEach(function (entry) {
      var row = el('tr');
      var idCell = el('td', entry.id, { class: 'id' });
      if (implicitOf(entry)) idCell.appendChild(el('div', implicitOf(entry), { class: 'implicit' }));
      row.appendChild(idCell);
      row.appendChild(el('td', entry.description || ''));
      var tagCell = el('td');
      var tagBox = el('div', null, { class: 'chips' });
      tagsOf(entry, state.kind).forEach(function (t) { tagBox.appendChild(chip(t, state.tags.indexOf(t) >= 0, function () { toggleTag(t); })); });
      tagCell.appendChild(tagBox);
      row.appendChild(tagCell);
      var reach = state.kind === 'statuses' ? [] : (entry.targets || []);
      row.appendChild(el('td', reach.join(', ')));
      row.appendChild(el('td', (reg.usage[entry.id] || []).join('\n') || '—', { class: 'used', style: 'white-space: pre-line' }));
      rows.appendChild(row);
    });
  }

  function start() {
    ['search', 'target', 'usedOnly'].forEach(function (id) {
      document.getElementById(id).addEventListener('input', function (e) {
        state[id] = e.target.type === 'checkbox' ? e.target.checked : e.target.value;
        render();
      });
    });
    fetch('/api/registry').then(function (r) { return r.json(); }).then(function (reg) {
      state.registry = reg;
      var kinds = {};
      (reg.tags || []).forEach(function (t) { (t.targets || []).forEach(function (k) { kinds[k] = true; }); });
      var select = document.getElementById('target');
      Object.keys(kinds).sort().forEach(function (k) { select.appendChild(el('option', k, { value: k })); });
      document.getElementById('libraryStatus').textContent = 'content/registry';
      render();
    }).catch(function () {
      document.getElementById('libraryStatus').textContent = 'Start the designer server (native/bin/breach_designer, built by bash native/build.sh) to browse the library';
    });
  }

  document.addEventListener('DOMContentLoaded', start);
})(this);
