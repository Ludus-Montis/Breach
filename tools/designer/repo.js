// Repo mode for the Lane Tile Designer (Decision 30, specs/17-local-designer-app.md).
//
// Loaded first by index.html. If the designer server (native/bin/breach_designer) is answering, this:
//   1. copies content/designer/*.json into localStorage (so designer.js loads the repo's
//      libraries through its normal migration path), then loads designer.js;
//   2. mirrors every later library write back to content/designer/*.json;
//   3. adds Open / Save (Ctrl+S) / Save as / Import libraries. Saving writes
//      content/maps_src/<name>.designer.json and the server has the Rust crate write
//      content/maps/<name>.json in its schema; its warnings and errors are shown here.
// Without the server (e.g. index.html opened from disk) it just loads designer.js, which
// then behaves like the old standalone prototype (browser storage, Export -> Copy JSON).
(function () {
  'use strict';

  // Must match designer.js's *_KEY constants; checked after load (see checkLibraryKeys).
  var LIBRARY_KEYS = {
    factions: 'breach_faction_library_v1',
    default_relations: 'breach_faction_default_relations_v1',
    terrain: 'breach_terrain_library_v1',
    emplacements: 'breach_static_defense_library_v1',
    room_features: 'breach_feature_library_v1',
    room_prefabs: 'breach_room_prefab_library_v1',
    structure_prefabs: 'breach_structure_prefab_library_v1'
  };
  var CURRENT_FILE_KEY = 'breach_designer_current_file';
  // Must match the server's API_VERSION (native/rust/src/designer/http.rs); an older server is missing endpoints this page uses.
  var REQUIRED_API = 6;
  var DIRTY_KEY = 'breach_designer_dirty';

  var health = null;
  var currentFile = null;
  var dirty = false;
  var lastImport = null;
  var pendingLibraryWrites = {};
  var libraryTimer = null;
  var loadingMap = false;

  function storeGet(k) { try { return localStorage.getItem(k); } catch (e) { return null; } }
  function storeSet(k, v) { try { if (v === null) localStorage.removeItem(k); else localStorage.setItem(k, v); } catch (e) { /* blocked storage */ } }
  function esc(s) { return String(s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); }
  function libraryForKey(k) {
    return Object.keys(LIBRARY_KEYS).find(function (lib) { return LIBRARY_KEYS[lib] === k; }) || null;
  }

  function api(method, path, body) {
    var opts = { method: method, headers: {} };
    if (body !== undefined) { opts.headers['Content-Type'] = 'application/json'; opts.body = JSON.stringify(body); }
    return fetch(path, opts).then(function (r) {
      return r.text().then(function (text) {
        var j;
        try { j = JSON.parse(text); } catch (e) {
          // Not our JSON API: most often a server started before an update (e.g. 501 for a new method).
          throw new Error('the designer server answered HTTP ' + r.status + ' instead of JSON' +
            (r.status === 501 || r.status === 404 ? ' — restart the designer server (native/bin/breach_designer) so it runs the current code' : ''));
        }
        if (!r.ok) throw new Error((j && j.error) || ('HTTP ' + r.status));
        return j;
      });
    });
  }

  function loadDesigner(onLoaded) {
    var s = document.createElement('script');
    s.src = 'designer.js';
    s.onload = function () { if (onLoaded) onLoaded(); };
    document.body.appendChild(s);
  }

  // ---------- boot ----------
  function boot() {
    if (location.protocol === 'file:') { loadDesigner(); return; }
    api('GET', '/api/health').then(function (h) {
      health = h;
      return Promise.all(Object.keys(LIBRARY_KEYS).map(function (lib) {
        return api('GET', '/api/libraries/' + lib).then(function (v) {
          if (v !== null) storeSet(LIBRARY_KEYS[lib], JSON.stringify(v));
        });
      }));
    }).then(function () {
      currentFile = storeGet(CURRENT_FILE_KEY);
      dirty = storeGet(DIRTY_KEY) === 'true';
      window.BreachDesignerHooks = { onStore: onStore, onMapChanged: onMapChanged };
      loadDesigner(function () { checkLibraryKeys(); installUi(); });
    }).catch(function () {
      health = null;
      loadDesigner();
    });
  }

  function checkLibraryKeys() {
    var theirs = (window.BreachDesigner && window.BreachDesigner.libraryKeys) || {};
    Object.keys(LIBRARY_KEYS).forEach(function (lib) {
      if (theirs[lib] !== LIBRARY_KEYS[lib]) console.error('repo.js LIBRARY_KEYS.' + lib + ' does not match designer.js - libraries will not sync');
    });
  }

  // ---------- hooks called by designer.js ----------
  function onStore(k, v) {
    var lib = libraryForKey(k);
    if (!lib) return;
    pendingLibraryWrites[lib] = v;
    clearTimeout(libraryTimer);
    libraryTimer = setTimeout(flushLibraries, 400);
  }
  function flushLibraries() {
    var batch = pendingLibraryWrites;
    pendingLibraryWrites = {};
    Object.keys(batch).forEach(function (lib) {
      var parsed;
      try { parsed = JSON.parse(batch[lib]); } catch (e) { return; }
      api('PUT', '/api/libraries/' + lib, parsed).then(function (res) {
        // Spec 39 step 2b: the terrain library is content/terrain/terrain_library.json, which
        // the server refuses to write if the edit drops a terrain or feature a saved map uses.
        if (res && res.import && !res.import.ok) {
          lastImport = { name: 'terrain library', result: res.import, at: new Date(), library: true };
          showStatus('Terrain library not saved: ' + (res.import.errors[0] || 'refused'), 'bad');
        } else if (res && res.import && res.changed) {
          showStatus('Terrain library saved for every map', 'ok');
        }
      }).catch(function (e) {
        showStatus('Library "' + lib + '" not saved to the repo: ' + e.message, 'bad');
      });
    });
  }
  function onMapChanged() {
    if (loadingMap) return;
    setDirty(true);
  }
  function setDirty(v) {
    dirty = v;
    storeSet(DIRTY_KEY, v ? 'true' : 'false');
    renderRepoBar();
  }
  function setCurrentFile(name) {
    currentFile = name;
    storeSet(CURRENT_FILE_KEY, name);
    renderRepoBar();
  }

  // ---------- UI ----------
  function installUi() {
    var header = document.querySelector('header.top');
    var bar = document.createElement('div');
    bar.id = 'repoBar';
    bar.className = 'repo-bar';
    header.appendChild(bar);

    var toolbar = document.querySelector('#viewLanes .toolbar');
    var exportBtn = document.getElementById('exportBtn');
    [['repoOpen', 'Open…', 'btn ghost', openMapPicker, 'Open a map from content/maps_src (Ctrl+O)'],
      ['repoSaveAs', 'Save as…', 'btn ghost', function () { saveMap(true); }, 'Save under a new file name'],
      ['repoSave', 'Save', 'btn primary', function () { saveMap(false); }, 'Save to content/maps_src and import to content/maps (Ctrl+S)'],
      ['repoView', 'View in Godot', 'btn ghost', viewInGodot, 'Open this map in the Godot map viewer (saves first if there are unsaved changes)']
    ].forEach(function (b) {
      var el = document.createElement('button');
      el.id = b[0]; el.type = 'button'; el.className = b[2]; el.textContent = b[1]; el.title = b[4];
      el.addEventListener('click', b[3]);
      toolbar.insertBefore(el, exportBtn);
    });
    exportBtn.className = 'btn ghost';

    var libBtn = document.createElement('button');
    libBtn.type = 'button'; libBtn.className = 'view-btn'; libBtn.id = 'repoImportLibs';
    libBtn.textContent = 'Import libraries…';
    libBtn.title = 'Paste libraries copied from the old claude.ai designer';
    libBtn.addEventListener('click', openLibraryImport);
    document.querySelector('nav.view-bar').appendChild(libBtn);

    var created = document.getElementById('createNewMap');
    if (created) created.addEventListener('click', function () { setCurrentFile(null); setDirty(true); });

    document.addEventListener('keydown', function (e) {
      if (!(e.ctrlKey || e.metaKey)) return;
      var k = e.key.toLowerCase();
      if (k === 's') { e.preventDefault(); saveMap(e.shiftKey); }
      if (k === 'o') { e.preventDefault(); openMapPicker(); }
    });
    injectStyle();
    renderRepoBar();
  }

  function injectStyle() {
    var css = [
      '.repo-bar { display:flex; gap:10px; align-items:center; flex-wrap:wrap; font-size:12px; color:var(--ink-dim); }',
      '.repo-bar .file { font-family:"IBM Plex Mono",monospace; color:var(--ink); }',
      '.repo-bar .dirty { color:var(--accent); font-weight:600; }',
      '.repo-bar .ok { color:var(--good); } .repo-bar .bad { color:var(--danger); font-weight:600; }',
      '.repo-bar button.linkish { background:none; border:none; color:inherit; text-decoration:underline; cursor:pointer; font:inherit; padding:0; }',
      '.map-pick { display:flex; flex-direction:column; gap:4px; max-height:50vh; overflow:auto; }',
      '.map-pick button { text-align:left; display:flex; justify-content:space-between; gap:12px; }',
      '.import-list { margin:4px 0 10px; padding-left:18px; font-size:12.5px; }',
      '.import-list.bad li { color:var(--danger); }',
      '#repoLibText { width:100%; min-height:220px; font-family:"IBM Plex Mono",monospace; font-size:12px; background:var(--bg); color:var(--ink); border:1px solid var(--line); border-radius:var(--radius); }'
    ].join('\n');
    var st = document.createElement('style');
    st.textContent = css;
    document.head.appendChild(st);
  }

  var statusText = '', statusTone = '';
  function showStatus(text, tone) { statusText = text; statusTone = tone || ''; renderRepoBar(); }

  function renderRepoBar() {
    var bar = document.getElementById('repoBar');
    if (!bar) return;
    var parts = [];
    parts.push('<span title="' + esc(health.repo) + '">Repo</span>');
    parts.push(health.godot_found
      ? '<span class="ok" title="' + esc(health.godot) + '">Godot ✓</span>'
      : '<span class="bad" title="Pass --godot, set GODOT_BIN, or add tools/designer/local_config.json">Godot not configured</span>');
    parts.push(currentFile
      ? '<span>File <span class="file">' + esc(currentFile) + '.designer.json</span></span>'
      : '<span>File <span class="file">(unsaved)</span></span>');
    if ((health.api_version || 1) < REQUIRED_API) {
      parts.push('<span class="bad" title="the designer server was started before the designer was updated">Server is out of date — restart native/bin/breach_designer</span>');
    }
    if (dirty) parts.push('<span class="dirty">● unsaved changes</span>');
    if (statusText) {
      parts.push('<span class="' + statusTone + '">' + esc(statusText) + '</span>');
      if (lastImport) parts.push('<button type="button" class="linkish" id="repoDetails">details</button>');
    }
    bar.innerHTML = parts.join('<span>·</span>');
    var det = document.getElementById('repoDetails');
    if (det) det.addEventListener('click', showImportDetails);
  }

  // ---------- modal helper (reuses the designer's modal styling) ----------
  function modal(title, bodyHtml, buttons) {
    var back = document.createElement('div');
    back.className = 'modal-backdrop';
    back.innerHTML = '<div class="modal"><div class="modal-head"><h2>' + esc(title) + '</h2>' +
      '<button class="close-x" type="button" aria-label="Close">&times;</button></div>' +
      '<div class="modal-body">' + bodyHtml + '</div><div class="modal-foot"></div></div>';
    function close() { back.remove(); }
    back.querySelector('.close-x').addEventListener('click', close);
    back.addEventListener('click', function (e) { if (e.target === back) close(); });
    var foot = back.querySelector('.modal-foot');
    (buttons || [{ label: 'Close', primary: true }]).forEach(function (b) {
      var el = document.createElement('button');
      el.type = 'button'; el.className = 'btn' + (b.primary ? ' primary' : ''); el.textContent = b.label;
      el.addEventListener('click', function () { if (!b.onClick || b.onClick() !== false) close(); });
      foot.appendChild(el);
    });
    document.body.appendChild(back);
    return back;
  }

  // ---------- save ----------
  function slug(s) {
    return (String(s || '').toLowerCase().replace(/[^a-z0-9_-]+/g, '_').replace(/^_+|_+$/g, '').slice(0, 64)) || 'map';
  }
  function validName(n) { return /^[A-Za-z0-9_-]{1,64}$/.test(n); }

  // after(importResult) runs once a save finishes (used by View in Godot).
  function saveMap(askName, after) {
    if (!currentFile || askName) { promptName(function (name) { doSave(name, after); }); return; }
    doSave(currentFile, after);
  }
  function promptName(then) {
    var suggested = currentFile || slug(window.BreachDesigner.mapName());
    var m = modal('Save map as',
      '<div class="field"><label for="repoName">File name</label>' +
      '<input type="text" id="repoName" value="' + esc(suggested) + '" style="width:260px;" /></div>' +
      '<p class="hint">Saved as <code>content/maps_src/&lt;name&gt;.designer.json</code>, written to <code>content/maps/&lt;name&gt;.json</code>. Letters, digits, _ and - only.</p>' +
      '<p class="warn-inline" id="repoNameErr" hidden></p>',
      [{ label: 'Cancel' }, { label: 'Save', primary: true, onClick: submit }]);
    var input = m.querySelector('#repoName');
    input.focus(); input.select();
    input.addEventListener('keydown', function (e) { if (e.key === 'Enter' && submit() !== false) m.remove(); });
    function submit() {
      var name = input.value.trim();
      var err = m.querySelector('#repoNameErr');
      if (!validName(name)) { err.textContent = 'Use letters, digits, _ and - only (max 64).'; err.hidden = false; return false; }
      then(name);
      return true;
    }
  }
  function doSave(name, after) {
    var data = window.BreachDesigner.buildExport();
    showStatus('Saving ' + name + ' and importing…', '');
    var btn = document.getElementById('repoSave');
    if (btn) btn.disabled = true;
    api('PUT', '/api/maps/' + encodeURIComponent(name), data).then(function (res) {
      setCurrentFile(name);
      setDirty(false);
      lastImport = { name: name, result: res.import, at: new Date() };
      var imp = res.import;
      if (imp.ok) {
        showStatus('Saved ' + new Date().toLocaleTimeString() + ' · written to content/maps/' + name + '.json' +
          (imp.warnings.length ? ' (' + imp.warnings.length + ' warning' + (imp.warnings.length === 1 ? '' : 's') + ')' : ''), 'ok');
      } else {
        showStatus('Saved source JSON, but the map ' + (imp.ran ? 'was refused' : 'was not written') + ' — content/maps/' + name + '.json not updated', 'bad');
        showImportDetails();
      }
      if (after) after(imp);
    }).catch(function (e) {
      lastImport = null;
      showStatus('Save failed: ' + e.message, 'bad');
    }).then(function () { if (btn) btn.disabled = false; });
  }
  // ---------- view in Godot (specs/18) ----------
  function viewInGodot() {
    if ((health.api_version || 1) < REQUIRED_API) { showStatus('The designer server is out of date — stop native/bin/breach_designer and start it again, then retry', 'bad'); return; }
    if (!health.godot_found) { showStatus('Godot not configured, so the viewer cannot be opened', 'bad'); return; }
    if (!currentFile || dirty) {
      saveMap(false, function (imp) { if (imp.ok) launchViewer(currentFile); });
      return;
    }
    launchViewer(currentFile);
  }
  function launchViewer(name) {
    api('POST', '/api/maps/' + encodeURIComponent(name) + '/view', {}).then(function () {
      showStatus('Opening ' + name + ' in the Godot map viewer…', 'ok');
    }).catch(function (e) { showStatus('Could not open the viewer: ' + e.message, 'bad'); });
  }

  function showImportDetails() {
    if (!lastImport) return;
    var imp = lastImport.result;
    if (lastImport.library) {
      modal('Terrain library import',
        '<p><code>content/terrain/terrain_library.json</code> was not saved, so maps are unaffected.</p>' +
        '<h3>Errors</h3><ul class="import-list bad">' + imp.errors.map(function (x) { return '<li>' + esc(x) + '</li>'; }).join('') + '</ul>');
      return;
    }
    var html = '<p>Saved <code>content/maps_src/' + esc(lastImport.name) + '.designer.json</code> at ' + esc(lastImport.at.toLocaleTimeString()) + '.</p>';
    html += imp.ok
      ? '<p>Map written: <code>content/maps/' + esc(lastImport.name) + '.json</code></p>'
      : '<p><strong>Map ' + (imp.ran ? 'refused' : 'not written') + '</strong> — <code>content/maps/' + esc(lastImport.name) + '.json</code> was not changed.</p>';
    if (imp.errors.length) html += '<h3>Errors</h3><ul class="import-list bad">' + imp.errors.map(function (x) { return '<li>' + esc(x) + '</li>'; }).join('') + '</ul>';
    if (imp.warnings.length) html += '<h3>Warnings</h3><ul class="import-list">' + imp.warnings.map(function (x) { return '<li>' + esc(x) + '</li>'; }).join('') + '</ul>';
    modal('Save & import', html);
  }

  // ---------- open ----------
  function openMapPicker() {
    api('GET', '/api/maps').then(function (maps) {
      var body = maps.length
        ? '<div class="map-pick">' + maps.map(function (m) {
          return '<button type="button" class="btn ghost" data-map="' + esc(m.name) + '"><span class="file">' + esc(m.name) +
            '</span><span>' + (m.imported ? 'imported' : 'not imported') + ' · ' + esc(new Date(m.modified * 1000).toLocaleString()) + '</span></button>';
        }).join('') + '</div>'
        : '<p>No maps in <code>content/maps_src/</code> yet. Build one and press Save.</p>';
      if (dirty) body = '<p class="warn-inline">You have unsaved changes; opening a map replaces them.</p>' + body;
      var m = modal('Open map', body, [{ label: 'Cancel' }]);
      m.querySelectorAll('[data-map]').forEach(function (b) {
        b.addEventListener('click', function () { m.remove(); openMap(b.getAttribute('data-map')); });
      });
    }).catch(function (e) { showStatus('Could not list maps: ' + e.message, 'bad'); });
  }
  function openMap(name) {
    api('GET', '/api/maps/' + encodeURIComponent(name)).then(function (data) {
      loadingMap = true;
      var res;
      try { res = window.BreachDesigner.loadFromExport(data); } finally { loadingMap = false; }
      setCurrentFile(name);
      setDirty(false);
      lastImport = null;
      showStatus('Opened ' + name + (res.librariesAdded ? ' · added ' + res.librariesAdded + ' library entr' + (res.librariesAdded === 1 ? 'y' : 'ies') + ' it uses' : ''), 'ok');
    }).catch(function (e) { showStatus('Could not open ' + name + ': ' + e.message, 'bad'); });
  }

  // ---------- import libraries (from the retired claude.ai artifact) ----------
  function openLibraryImport() {
    var m = modal('Import libraries',
      '<p>In the old claude.ai designer, press <strong>Copy libraries JSON</strong>, then paste it here. ' +
      'Each library in the paste replaces the repo&rsquo;s copy in <code>content/designer/</code>; the page then reloads.</p>' +
      '<textarea id="repoLibText" placeholder="{ &quot;factions&quot;: [...], &quot;terrain&quot;: {...}, ... }"></textarea>' +
      '<p class="warn-inline" id="repoLibErr" hidden></p>',
      [{ label: 'Cancel' }, { label: 'Import', primary: true, onClick: submit }]);
    function submit() {
      var err = m.querySelector('#repoLibErr'), parsed;
      try { parsed = JSON.parse(m.querySelector('#repoLibText').value); } catch (e) { err.textContent = 'Not valid JSON: ' + e.message; err.hidden = false; return false; }
      var libs = Object.keys(parsed || {}).filter(function (k) { return LIBRARY_KEYS[k] && parsed[k] !== null; });
      if (!libs.length) { err.textContent = 'No known libraries in the paste (expected keys: ' + Object.keys(LIBRARY_KEYS).join(', ') + ').'; err.hidden = false; return false; }
      Promise.all(libs.map(function (lib) {
        storeSet(LIBRARY_KEYS[lib], JSON.stringify(parsed[lib]));
        return api('PUT', '/api/libraries/' + lib, parsed[lib]);
      })).then(function () { location.reload(); }, function (e) { showStatus('Library import failed: ' + e.message, 'bad'); });
      return true;
    }
  }

  boot();
})();
