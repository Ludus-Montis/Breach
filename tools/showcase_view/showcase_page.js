// The Showcase tab's drawing (specs/44-project-site-and-showcase.md, step 5): loads
// manifest.json from its own folder (the site build downloads it) and draws the cards with
// showcase_view.js's logic.
(function () {
  'use strict';
  var view = window.ShowcaseView;
  var MOMENTS = ['first', 'contact', 'midpoint', 'last'];
  var LABELS = { tick_median_ms: 'tick, median', tick_p95_ms: 'tick, 95th pct' };
  var captures = [];
  var cards = [];
  var comparing = {};

  function el(tag, attrs, children) {
    var node = document.createElement(tag);
    Object.keys(attrs || {}).forEach(function (key) {
      if (key === 'text') node.textContent = attrs[key];
      else node.setAttribute(key, attrs[key]);
    });
    (children || []).forEach(function (child) { if (child) node.appendChild(child); });
    return node;
  }

  function ms(value) {
    return value === null ? '–' : value.toFixed(3);
  }

  function changeCell(value) {
    if (value === null) return el('td', { text: '' });
    var sign = value > 0 ? '+' : '';
    return el('td', { text: sign + value + '%', class: value > 0 ? 'up' : value < 0 ? 'down' : '' });
  }

  function timingTable(rows, compared) {
    var head = ['', 'ms'].concat(compared ? ['before', 'change'] : []);
    var table = el('table', {}, [el('tr', {}, head.map(function (h) { return el('th', { text: h }); }))]);
    rows.forEach(function (r) {
      var cells = [el('td', { text: LABELS[r.key] || r.key }), el('td', { text: ms(r.value) })];
      if (compared) cells.push(el('td', { text: ms(r.before) }), changeCell(r.change));
      table.appendChild(el('tr', {}, cells));
    });
    return table;
  }

  function timings(made, before, compared) {
    var rows = view.timingRows(made.timings, before && before.timings);
    var tick = rows.slice(0, 2);
    var phases = rows.slice(2);
    var out = el('div', { class: 'timings' }, [timingTable(tick, compared)]);
    if (phases.length) {
      out.appendChild(el('details', {}, [el('summary', { text: 'Phases, median' }),
        timingTable(phases, compared)]));
    }
    return out;
  }

  function behaviourList(made) {
    return el('ul', { class: 'behaviours' }, view.behaviours(made).map(function (b) {
      var name = b.name.replace(/_/g, ' ');
      if (b.tick !== null) return el('li', { text: '✓ ' + name + ' (tick ' + b.tick + ')' });
      return el('li', { class: b.missing ? 'missing' : 'not', text: '✗ ' + name + ': not seen' });
    }));
  }

  function media(made) {
    var stills = el('div', { class: 'stills' }, MOMENTS.filter(function (m) {
      return made.stills && made.stills[m];
    }).map(function (m) {
      var tick = made.moments && made.moments[m];
      return el('figure', {}, [
        el('img', { src: made.stills[m], alt: made.name + ', ' + m, loading: 'lazy' }),
        el('figcaption', { text: m + (tick !== undefined && tick !== null ? ' (tick ' + tick + ')' : '') }),
      ]);
    }));
    var video = el('video', { src: made.clip, controls: '', preload: 'none', playsinline: '' });
    return { stills: stills, video: video };
  }

  // One capture's side of a scenario: stills, player, timings, behaviours.
  function side(made, before, compared, heading) {
    var parts = media(made);
    var box = el('div', {}, [
      heading ? el('h4', { text: heading }) : null,
      el('div', { class: 'body' }, [
        el('div', {}, [parts.stills, parts.video]),
        el('div', {}, [timings(made, before, compared), behaviourList(made)]),
      ]),
    ]);
    return { node: box, video: parts.video };
  }

  // Paired clips start together.
  function together(a, b) {
    function follow(leader, other) {
      leader.addEventListener('play', function () {
        if (!other.paused) return;
        other.currentTime = leader.currentTime;
        other.play().catch(function () {});
      });
      leader.addEventListener('pause', function () { if (!other.paused) other.pause(); });
    }
    follow(a, b);
    follow(b, a);
  }

  function label(capture) {
    return '#' + capture.pr + ' at ' + capture.commit.slice(0, 7);
  }

  function scenarioRow(pair, current, previous) {
    var compared = Boolean(previous);
    var box = el('div', { class: 'scenario' + (compared ? ' compared' : '') }, [el('h3', { text: pair.name })]);
    if (!compared) {
      box.appendChild(side(pair.current, null, false).node);
      return box;
    }
    var grid = el('div', { class: 'side' });
    if (pair.current && pair.previous) {
      var now = side(pair.current, pair.previous, true, label(current));
      var then = side(pair.previous, null, false, label(previous));
      grid.appendChild(now.node);
      grid.appendChild(then.node);
      together(now.video, then.video);
    } else {
      var only = pair.current || pair.previous;
      var whose = pair.current ? current : previous;
      var other = pair.current ? previous : current;
      grid.appendChild(side(only, null, false, label(whose)).node);
      grid.appendChild(el('p', { class: 'alone', text: 'Not in ' + label(other) + ': shown alone.' }));
    }
    box.appendChild(grid);
    return box;
  }

  function card(capture, scenario) {
    var previous = view.partner(capture, captures);
    var on = Boolean(comparing[capture.pr] && previous);
    var title = el('h2', {}, [el('a', { href: capture.url, text: '#' + capture.pr + ' ' + capture.title })]);
    title.appendChild(el('span', { class: 'badge ' + capture.state,
      text: capture.state === 'closed' ? 'closed unmerged' : capture.state }));
    var meta = el('p', { class: 'meta' }, [
      el('a', { href: capture.url.replace(/\/pull\/\d+$/, '/commit/' + capture.commit),
        text: capture.commit.slice(0, 7) }),
      document.createTextNode(' · ' + capture.date.replace('T', ' ').replace(/:\d\dZ$/, ' UTC') + ' '),
    ]);
    if (previous) {
      var button = el('button', { type: 'button', class: 'compare',
        text: on ? 'Hide compare' : 'Compare with ' + label(previous) });
      button.addEventListener('click', function () { comparing[capture.pr] = !on; draw(); });
      meta.appendChild(button);
    }
    var box = el('article', { class: 'card' }, [title, meta]);
    view.pairs(capture, on ? previous : null).forEach(function (pair) {
      if (scenario && pair.name !== scenario) return;
      box.appendChild(scenarioRow(pair, capture, on ? previous : null));
    });
    return box;
  }

  function options() {
    var states = Array.prototype.map.call(
      document.querySelectorAll('#filters input[name=state]:checked'), function (i) { return i.value; });
    return { states: states, scenario: document.getElementById('scenario').value };
  }

  function draw() {
    var picked = options();
    var main = document.getElementById('cards');
    main.textContent = '';
    var shown = view.filter(cards, picked);
    shown.forEach(function (capture) { main.appendChild(card(capture, picked.scenario)); });
    document.getElementById('status').textContent = cards.length
      ? shown.length + ' of ' + cards.length + ' PRs shown.'
      : 'No captures yet: the first game PR after the capture workflow makes one.';
  }

  function start(manifest) {
    captures = (manifest && manifest.captures) || [];
    cards = view.timeline(captures);
    var select = document.getElementById('scenario');
    view.scenarioNames(cards).forEach(function (name) {
      select.appendChild(el('option', { value: name, text: name }));
    });
    document.getElementById('filters').addEventListener('change', draw);
    draw();
  }

  fetch('manifest.json', { cache: 'no-cache' })
    .then(function (response) {
      if (!response.ok) throw new Error(response.status);
      return response.json();
    })
    .then(start)
    .catch(function (error) {
      document.getElementById('status').textContent = 'Could not load manifest.json (' + error.message + ').';
    });
})();
