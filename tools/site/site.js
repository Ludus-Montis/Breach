// The project site's shell (specs/44-project-site-and-showcase.md, step 4, Decision 166): a
// tab bar from one list of tabs, routed by `#name`. Each tab is a self-contained folder whose
// page loads in its own frame on first view; the root URL and any unknown name open the
// first tab, Showcase (Decision 170; it was Metrics, Decision 166). `#metrics` opens Metrics.
// A new tab is a folder and one line in TABS (and the folder's copy in site-pages.yml).
(function (root) {
  'use strict';

  var TABS = [
    { name: 'showcase', title: 'Showcase', page: 'showcase/index.html' },
    { name: 'metrics', title: 'Metrics', page: 'metrics/index.html' },
  ];

  function route(hash, tabs) {
    var name = String(hash || '').replace(/^#/, '').trim().toLowerCase();
    for (var i = 0; i < tabs.length; i++) {
      if (tabs[i].name === name) return tabs[i];
    }
    return tabs[0];
  }

  var api = { TABS: TABS, route: route };
  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  root.BreachSite = api;

  if (typeof document === 'undefined') return;

  var frames = {};
  var links = {};

  function build() {
    var nav = document.getElementById('tabs');
    TABS.forEach(function (tab) {
      var link = document.createElement('a');
      link.href = '#' + tab.name;
      link.textContent = tab.title;
      link.setAttribute('role', 'tab');
      nav.appendChild(link);
      links[tab.name] = link;
    });
  }

  function frame(tab) {
    if (!frames[tab.name]) {
      var node = document.createElement('iframe');
      node.src = tab.page;
      node.title = tab.title;
      document.getElementById('frames').appendChild(node);
      frames[tab.name] = node;
    }
    return frames[tab.name];
  }

  function show() {
    var tab = route(location.hash, TABS);
    TABS.forEach(function (other) {
      var on = other.name === tab.name;
      links[other.name].setAttribute('aria-selected', on ? 'true' : 'false');
      if (frames[other.name]) frames[other.name].hidden = !on;
    });
    frame(tab).hidden = false;
    document.title = 'Breach: ' + tab.title;
  }

  build();
  window.addEventListener('hashchange', show);
  show();
})(typeof window !== 'undefined' ? window : globalThis);
