// Lane Tile Designer - the map authoring tool (Decision 30, specs/17-local-designer-app.md).
// Served by serve.py for repo mode (Open/Save into content/maps_src + automatic Godot
// import); repo.js loads this file after syncing the repo libraries into localStorage.
(function () {
  'use strict';

  // WAYPOINT is a routing point that bends a link's route - a real NodeType in Godot since specs/16.
  // It replaced structure_slots == 0 (Decision 25).
  var NODE_TYPES = ['ORIGIN', 'RESOURCE', 'FORT', 'NEUTRAL', 'WAYPOINT'];
  var RESOURCE_TYPES = ['FOOD', 'WOOD', 'STONE', 'METAL', 'CRYSTAL'];
  var FACTION_PALETTE = ['#b3761d', '#a3372a', '#4c6b78', '#5a4a80', '#4c7a52', '#8a5a9c'];
  var UNIT_LIBRARY = ['Militia', 'Guard', 'Archer', 'Knight'];
  var STANCES = ['HOSTILE', 'NEUTRAL', 'ALLIED'];

  // Archetype is art only: it picks what to render. The numbers come from the segment
  // profile (node.structure.segments) checked against the tile's capacity.
  var ARCHETYPES = [
    { id: 'NONE', label: 'None' },
    { id: 'BARRICADE', label: 'Barricade' },
    { id: 'PALISADE', label: 'Palisade' },
    { id: 'WATCHTOWER', label: 'Watchtower' },
    { id: 'FORT', label: 'Fort' },
    { id: 'CASTLE', label: 'Castle' }
  ];
  // Room stubs a structure segment can hold (fort-internals idea). Not simulated.
  var ROOM_LIBRARY = [
    { id: 'BARRACKS', label: 'Barracks', abbr: 'BAR', note: 'defender capacity' },
    { id: 'STORES', label: 'Stores', abbr: 'STO', note: 'siege endurance' },
    { id: 'KITCHEN', label: 'Kitchen', abbr: 'KIT', note: 'siege endurance' },
    { id: 'GATEHOUSE', label: 'Gatehouse', abbr: 'GAT', note: 'controlled entry' },
    { id: 'LOOKOUT', label: 'Lookout', abbr: 'LOK', note: 'sight range' },
    { id: 'WALKWAY', label: 'Walkway', abbr: 'WLK', note: 'connects segments' }
  ];
  // Feature library defaults: any feature fits any room; the limit is the segment's
  // feature-point budget. Room prefabs are just named feature sets (Barracks = 5 Bunks).
  var DEFAULT_FEATURES = [
    { id: 'BUNKS', label: 'Bunks', cost: 1, effect: 'rest 4 units' },
    { id: 'HEARTH', label: 'Hearth', cost: 1, effect: 'warmth and morale' },
    { id: 'STORAGE_RACKS', label: 'Storage racks', cost: 1, effect: '+10 siege supply' },
    { id: 'ARMOURY_RACK', label: 'Armoury rack', cost: 2, effect: 'arms 4 units' },
    { id: 'WELL', label: 'Well', cost: 3, effect: 'water through a siege' },
    { id: 'KITCHEN', label: 'Kitchen', cost: 5, effect: 'feeds 20 through a siege' }
  ];
  var DEFAULT_ROOM_PREFABS = [
    { id: 'BARRACKS', label: 'Barracks', features: ['BUNKS', 'BUNKS', 'BUNKS', 'BUNKS', 'BUNKS'] },
    { id: 'KITCHEN', label: 'Kitchen', features: ['KITCHEN'] },
    { id: 'STOREROOM', label: 'Storeroom', features: ['STORAGE_RACKS', 'STORAGE_RACKS', 'STORAGE_RACKS', 'STORAGE_RACKS', 'STORAGE_RACKS'] },
    { id: 'GATEHOUSE', label: 'Gatehouse', features: ['HEARTH', 'ARMOURY_RACK'] }, // its portcullis/murder hole are boundary defenses now
    { id: 'GUARDROOM', label: 'Guardroom', features: ['ARMOURY_RACK', 'BUNKS', 'BUNKS', 'HEARTH'] }
  ];

  // Static defenses: mounted on a wall face or a flat roof. Firing points belong to the
  // defense (Static defenses view) and are manned automatically from the whole garrison.
  var DEFAULT_STATIC_DEFENSES = [
    // Emplacements (Decision 27): immobile crewed weapons placed in a room or on a flat roof.
    // Arrow slits, murder holes and portcullises are boundary types, not emplacements.
    { id: 'BALLISTA', label: 'Ballista', abbr: 'BAL', mount: 'ROOF', manned_by_unit: 'Guard', firing_points: 2, stub_damage: 14, stub_range: 5, notes: '' },
    { id: 'TREBUCHET', label: 'Trebuchet', abbr: 'TRB', mount: 'ROOF', manned_by_unit: 'Guard', firing_points: 3, stub_damage: 30, stub_range: 8, notes: 'needs a big flat roof (stub)' },
    { id: 'BOILING_OIL', label: 'Oil cauldron', abbr: 'OIL', mount: 'ROOM', manned_by_unit: 'Militia', firing_points: 1, stub_damage: 10, stub_range: 0, notes: 'poured through a murder hole or over a wall (stub)' }
  ];

  // Tile-level upgrades: added without needing a larger structure (a barracks at a mine).
  var UPGRADE_LIBRARY = [
    { id: 'GUARD_BARRACKS', label: 'Guard barracks' },
    { id: 'WATCH_POST', label: 'Watch post' },
    { id: 'GRANARY', label: 'Granary' },
    { id: 'PALISADE', label: 'Palisade' },
    { id: 'STOREHOUSE', label: 'Storehouse' }
  ];

  // Capacity per terrain: stability = total segment budget, max_height = tallest column,
  // max_width = widest span. needs_bridge = roads can't enter without a bridge.
  var DEFAULT_TERRAIN_LIBRARY = {
    terrains: [
      { id: 'FIELDS', label: 'Fields', glyph: '"', color: '#8f9a4f', can_be_base: true, needs_bridge: false, move_cost: 1, blocks_unit_classes: '', default_stability: 4, default_max_height: 2, default_max_width: 3, default_max_length: 3, default_max_depth: 1 },
      { id: 'ROCKY', label: 'Rocky', glyph: '◦', color: '#857d6e', can_be_base: true, needs_bridge: false, move_cost: 1.5, blocks_unit_classes: '', default_stability: 6, default_max_height: 3, default_max_width: 2, default_max_length: 2, default_max_depth: 2 },
      { id: 'SNOW', label: 'Snow', glyph: '*', color: '#8fa3b0', can_be_base: true, needs_bridge: false, move_cost: 2, blocks_unit_classes: '', default_stability: 2, default_max_height: 1, default_max_width: 2, default_max_length: 2, default_max_depth: 0 },
      { id: 'DESERT', label: 'Desert', glyph: '∴', color: '#c29b52', can_be_base: true, needs_bridge: false, move_cost: 1.5, blocks_unit_classes: '', default_stability: 3, default_max_height: 1, default_max_width: 3, default_max_length: 3, default_max_depth: 1 },
      { id: 'SWAMP', label: 'Swamp', glyph: '~', color: '#5d6b4a', can_be_base: true, needs_bridge: false, move_cost: 3, blocks_unit_classes: 'siege', default_stability: 1, default_max_height: 1, default_max_width: 1, default_max_length: 1, default_max_depth: 0 },
      { id: 'WATER', label: 'Water', glyph: '≈', color: '#3a6ea5', can_be_base: false, needs_bridge: true, move_cost: 99, blocks_unit_classes: 'infantry, cavalry, siege', default_stability: 0, default_max_height: 0, default_max_width: 0, default_max_length: 0, default_max_depth: 0 },
      { id: 'FOREST', label: 'Forest', glyph: '♣', color: '#2f6b3a', can_be_base: false, needs_bridge: false, move_cost: 2, blocks_unit_classes: 'siege', default_stability: 2, default_max_height: 2, default_max_width: 1, default_max_length: 1, default_max_depth: 1 },
      { id: 'MOUNTAIN', label: 'Mountain', glyph: '▲', color: '#7a6a5a', can_be_base: false, needs_bridge: false, move_cost: 4, blocks_unit_classes: 'cavalry, siege', default_stability: 4, default_max_height: 4, default_max_width: 1, default_max_length: 1, default_max_depth: 3 },
      { id: 'RAVINE', label: 'Ravine', glyph: '∷', color: '#4a3a2a', can_be_base: false, needs_bridge: true, move_cost: 99, blocks_unit_classes: 'all', default_stability: 0, default_max_height: 0, default_max_width: 0, default_max_length: 0, default_max_depth: 0 }
    ],
    features: [
      { id: 'ORE_VEIN', label: 'Ore vein', glyph: '◆', stub_effect: 'metal/stone deposit - a mine could be built here' },
      { id: 'ARABLE', label: 'Arable land', glyph: '≡', stub_effect: 'supports farming' },
      { id: 'SPRING', label: 'Spring', glyph: '○', stub_effect: 'fresh water' },
      { id: 'TIMBER', label: 'Timber stand', glyph: '♠', stub_effect: 'harvestable wood' },
      { id: 'QUARRY', label: 'Quarry face', glyph: '▣', stub_effect: 'exposed stone' }
    ],
    road_move_multiplier: 0.5
  };

  // The ground (Decisions 53, 54, 57): materials, and per terrain the bearing a cell column
  // carries, how far foundations raise it, how deep the strata go (cells), the water table
  // (cells below the surface; null = none) and the strata bands from the surface down (the
  // last continues to the dig depth). Placeholders to tune.
  // Materials (Decisions 54, 57, 63-66): physical properties (weight, span, strength, heat 0-10)
  // and rated traits met in ability-and-demand pairs (dig_difficulty against burrower,
  // climb_difficulty against climber). A material that flows is a liquid: flows N is its rate.
  var DEFAULT_MATERIALS = [
    { id: 'SOIL', label: 'Soil', color: '#6b5440', weight: 1, span: 1, strength: 1, heat: 1, traits: { dig_difficulty: 1 }, heat_transitions: [] },
    { id: 'PEAT', label: 'Peat', color: '#4a3d2c', weight: 1, span: 0, strength: 0, heat: 1, traits: { dig_difficulty: 1, climb_difficulty: 1 }, heat_transitions: [{ above: 3, gains: 'burning' }] },
    { id: 'SAND', label: 'Sand', color: '#cdb27a', weight: 1, span: 0, strength: 0, heat: 1, traits: { dig_difficulty: 1, climb_difficulty: 1, loose: 1 }, heat_transitions: [] },
    { id: 'GRAVEL', label: 'Gravel', color: '#8d877c', weight: 1, span: 0, strength: 1, heat: 1, traits: { dig_difficulty: 1, climb_difficulty: 1, loose: 1 }, heat_transitions: [] },
    { id: 'CLAY', label: 'Clay', color: '#8a6a4a', weight: 1, span: 2, strength: 2, heat: 1, traits: { dig_difficulty: 2 }, heat_transitions: [] },
    { id: 'ROCK', label: 'Rock', color: '#6f6c68', weight: 2, span: 4, strength: 20, heat: 1, traits: { dig_difficulty: 3 }, heat_transitions: [{ above: 7, becomes: 'LAVA' }] },
    { id: 'ORE', label: 'Ore', color: '#7b5e57', weight: 2, span: 4, strength: 20, heat: 1, traits: { dig_difficulty: 4 }, heat_transitions: [] },
    { id: 'TIMBER', label: 'Timber', color: '#8b6a3e', weight: 1, span: 3, strength: 6, heat: 1, traits: { dig_difficulty: 1 }, heat_transitions: [{ above: 3, gains: 'burning' }] },
    { id: 'WATER', label: 'Water', color: '#3a6ea5', weight: 1, span: 0, strength: 0, heat: 1, traits: { flows: 3 }, heat_transitions: [] },
    { id: 'LAVA', label: 'Lava', color: '#d4521c', weight: 1, span: 0, strength: 0, heat: 8, traits: { flows: 1, glows: 1 }, heat_transitions: [{ below: 6, becomes: 'ROCK' }] }
  ];
  function water(min, max, chance, surface) { return { material: 'WATER', min: min, max: max, chance: chance, surface_chance: surface }; }
  function lava(min, max, chance, surface) { return { material: 'LAVA', min: min, max: max, chance: chance, surface_chance: surface }; }
  function ground(bearing, foundationMax, digDepth, liquids, strata) {
    return { bearing: bearing, foundation_max: foundationMax, dig_depth: digDepth, liquids: liquids,
      strata: strata.map(function (b) { return { material: b[0], min: b[1], max: b[2] }; }) };
  }
  var DEFAULT_GROUND = {
    FIELDS: ground(4, 8, 32, [water(12, 24, 1, 0.02)], [['SOIL', 2, 4], ['CLAY', 4, 8], ['ROCK', 8, 16]]),
    ROCKY: ground(8, 12, 32, [water(16, 32, 1, 0), lava(32, 48, 0.2, 0.02)], [['SOIL', 0, 1], ['ROCK', 8, 16]]),
    SNOW: ground(3, 6, 24, [], [['SOIL', 1, 2], ['ROCK', 8, 16]]),
    DESERT: ground(3, 6, 32, [water(20, 32, 0.3, 0)], [['SAND', 4, 10], ['ROCK', 8, 16]]),
    SWAMP: ground(2, 4, 16, [water(0, 2, 1, 0.5)], [['PEAT', 2, 4], ['CLAY', 4, 8]]),
    WATER: ground(0, 0, 0, [], []),
    FOREST: ground(4, 8, 32, [water(10, 20, 1, 0.02)], [['SOIL', 2, 3], ['CLAY', 3, 6], ['ROCK', 8, 16]]),
    MOUNTAIN: ground(8, 12, 48, [lava(24, 48, 0.35, 0.05)], [['SOIL', 0, 1], ['ROCK', 16, 32]]),
    RAVINE: ground(0, 0, 0, [], [])
  };
  // A terrain with no seeded ground (a custom one) starts from this.
  var GENERIC_GROUND = ground(4, 8, 32, [water(12, 24, 1, 0.02)], [['SOIL', 1, 3], ['ROCK', 8, 16]]);
  // Ground height in cells for a tile that sets none (Decision 56): stylised, not 1:1;
  // a mountain reaches the default ceiling and blocks flight (Decision 60).
  var DEFAULT_ELEVATION = { FIELDS: 2, ROCKY: 6, SNOW: 8, DESERT: 2, SWAMP: 0, WATER: 0, FOREST: 3, MOUNTAIN: 64, RAVINE: 0 };
  GENERIC_GROUND.default_elevation = 4;
  Object.keys(DEFAULT_GROUND).forEach(function (id) { DEFAULT_GROUND[id].default_elevation = DEFAULT_ELEVATION[id]; });
  // Seeded unevenness within a tile (Decision 59): [amplitude, scale] in cells.
  var DEFAULT_RELIEF = { FIELDS: [1, 12], ROCKY: [2, 4], SNOW: [1, 10], DESERT: [2, 12], SWAMP: [0, 0], WATER: [0, 0], FOREST: [1, 8], MOUNTAIN: [8, 8], RAVINE: [0, 0] };
  GENERIC_GROUND.relief = { amplitude: 2, scale: 8 };
  Object.keys(DEFAULT_GROUND).forEach(function (id) { DEFAULT_GROUND[id].relief = { amplitude: DEFAULT_RELIEF[id][0], scale: DEFAULT_RELIEF[id][1] }; });
  DEFAULT_TERRAIN_LIBRARY.materials = DEFAULT_MATERIALS;
  DEFAULT_TERRAIN_LIBRARY.features.push({ id: 'LAVA_VENT', label: 'Lava vent', glyph: '♨', stub_effect: 'lava reaches the surface here (Decision 62)' });
  DEFAULT_TERRAIN_LIBRARY.terrains.forEach(function (t) { Object.assign(t, clone(DEFAULT_GROUND[t.id])); });

  function seg(room) { return { room: room || '', features: [], defenses: [] }; }
  function column(h, rooms) { var c = []; for (var i = 0; i < h; i++) c.push(seg(rooms && rooms[i])); return c; }
  function struct(archetype, columns) { return normalizeStructure({ archetype: archetype, columns: columns || [] }); }

  var PRESETS = [
    { id: 'farm', name: 'Farm', meta: 'Resource · Food', node_type: 'RESOURCE', structure: struct('NONE'),
      fields: { yield_food_per_tick: 6, decay_interval_ticks: 5, decay_floor_food: 2, ravage_yield_food: 40, resource_type: 'FOOD', is_inexhaustible: true, total_reserves: 0 } },
    { id: 'fort_light', name: 'Fort — Lightly Defended', meta: 'Fort · 2×1', node_type: 'FORT', structure: struct('FORT', [column(1), column(1)]),
      fields: { garrison_hp: 26, garrison_dmg: 6, dismantle_wood_yield: 8, dismantle_stone_yield: 5, fortify_wood_cost: 10, fortify_stone_cost: 6, capture_reward: '' } },
    { id: 'fort_heavy', name: 'Fort — Heavily Defended', meta: 'Fort · 2×2', node_type: 'FORT', structure: struct('FORT', [column(2, ['BARRACKS', 'ARROW_SLIT']), column(2, ['STORES', 'ARROW_SLIT'])]),
      fields: { garrison_hp: 60, garrison_dmg: 14, dismantle_wood_yield: 16, dismantle_stone_yield: 12, fortify_wood_cost: 20, fortify_stone_cost: 14, capture_reward: '' } },
    { id: 'watchtower', name: 'Watchtower', meta: 'Neutral · 1×2', node_type: 'NEUTRAL', structure: struct('WATCHTOWER', [column(2, ['', 'LOOKOUT'])]), fields: { capture_reward: '' } },
    { id: 'barricade', name: 'Barricade', meta: 'Neutral · 1 segment', node_type: 'NEUTRAL', structure: struct('BARRICADE', [column(1)]), fields: { capture_reward: '' } },
    { id: 'empty_slot', name: 'Empty Slot', meta: 'Neutral · unbuilt', node_type: 'NEUTRAL', structure: struct('NONE'), fields: { capture_reward: '' } },
    { id: 'waypoint', name: 'Waypoint', meta: 'Routing point', node_type: 'WAYPOINT', structure: struct('NONE'), fields: { capture_reward: '' } }
  ];

  var STORE_KEY = 'breach_lane_tile_designer_v13';
  var LEGACY_KEYS = ['breach_lane_tile_designer_v12', 'breach_lane_tile_designer_v11', 'breach_lane_tile_designer_v10', 'breach_lane_tile_designer_v9', 'breach_lane_tile_designer_v8', 'breach_lane_tile_designer_v7', 'breach_lane_tile_designer_v6', 'breach_lane_tile_designer_v5'];
  var LIBRARY_KEY = 'breach_faction_library_v1';
  var DEFAULT_REL_KEY = 'breach_faction_default_relations_v1';
  var TERRAIN_LIBRARY_KEY = 'breach_terrain_library_v1';
  var DEFENSE_LIBRARY_KEY = 'breach_static_defense_library_v1';
  var FEATURE_LIBRARY_KEY = 'breach_feature_library_v1';
  var ROOM_PREFAB_KEY = 'breach_room_prefab_library_v1';
  var STRUCTURE_PREFAB_KEY = 'breach_structure_prefab_library_v1';
  var VIEW_KEY = 'breach_designer_view';
  var CELL = 46, GAP = 3;

  function esc(s) {
    return String(s === undefined || s === null ? '' : s)
      .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
  }
  function clone(o) { return JSON.parse(JSON.stringify(o)); }
  function storeGet(k) { try { return localStorage.getItem(k); } catch (e) { return null; } }
  function storeSet(k, v) {
    try { localStorage.setItem(k, v); } catch (e) { /* private window / blocked storage */ }
    // Repo mode (repo.js): library keys are mirrored into content/designer/*.json.
    if (window.BreachDesignerHooks && window.BreachDesignerHooks.onStore) window.BreachDesignerHooks.onStore(k, v);
  }
  function plural(n, word) { return n + ' ' + word + (n === 1 ? '' : 's'); }

  // ---------- libraries (cross-map, persist independently of any one map) ----------
  var libraryFactions = [];
  var defaultRelations = []; // [{a, b, stance}] - library-level defaults, prototype-only
  var terrainLib = clone(DEFAULT_TERRAIN_LIBRARY);

  function migrateTerrainEntry(t) {
    if (t.default_max_height !== undefined) return;
    // v7 semantics: default_stability meant "max floors", default_footprint meant width.
    var seed = DEFAULT_TERRAIN_LIBRARY.terrains.find(function (x) { return x.id === t.id; });
    if (seed) {
      t.default_stability = seed.default_stability;
      t.default_max_height = seed.default_max_height;
      t.default_max_width = seed.default_max_width;
      t.needs_bridge = seed.needs_bridge;
    } else {
      t.default_max_height = Number(t.default_stability) || 0;
      t.default_max_width = Number(t.default_footprint) || 0;
      t.default_stability = t.default_max_height * t.default_max_width;
      t.needs_bridge = false;
    }
    delete t.default_footprint;
  }

  (function loadLibraries() {
    try {
      var raw = storeGet(LIBRARY_KEY);
      if (raw) libraryFactions = JSON.parse(raw) || [];
    } catch (e) { libraryFactions = []; }
    if (!libraryFactions.length) {
      // Only Player is a permanent default - any other faction must be added explicitly.
      libraryFactions = [{ id: 'player', display_name: 'Player' }];
      saveLibrary();
    }
    try {
      var rawR = storeGet(DEFAULT_REL_KEY);
      if (rawR) defaultRelations = JSON.parse(rawR) || [];
    } catch (e) { defaultRelations = []; }
    try {
      var rawT = storeGet(TERRAIN_LIBRARY_KEY);
      if (rawT) {
        var parsed = JSON.parse(rawT);
        if (parsed && parsed.terrains && parsed.terrains.length) terrainLib = parsed;
      }
    } catch (e) { /* keep defaults */ }
    if (!terrainLib.features) terrainLib.features = [];
    if (terrainLib.road_move_multiplier === undefined) terrainLib.road_move_multiplier = 0.5;
    terrainLib.terrains.forEach(migrateTerrainEntry);
    // v14: the ground model (Decisions 53, 54). Seeded terrains get their ground; custom
    // ones (e.g. Hilly) the generic ground.
    if (!terrainLib.materials) terrainLib.materials = clone(DEFAULT_MATERIALS);
    // v16-17: traits and heat (Decisions 63, 64). Flags and difficulty fields become traits,
    // and liquids become materials that flow; new seeds are added.
    (terrainLib.liquids || []).forEach(function (l) {
      if (materialDef(l.id)) return;
      var flowing = Object.assign({ weight: 1, span: 0 }, l);
      flowing.traits = Object.assign({ flows: (l.temperature || 0) > 500 ? 1 : 3 }, l.traits || {});
      terrainLib.materials.push(flowing);
    });
    delete terrainLib.liquids;
    terrainLib.materials.forEach(function (m) {
      m.traits = m.traits || {};
      if (m.loose) m.traits.loose = 1;
      ['dig_difficulty', 'climb_difficulty'].forEach(function (f) { if (m[f]) m.traits[f] = m[f]; delete m[f]; });
      delete m.loose;
      // v20: heat levels 0-10 replace degrees (Decision 66).
      if (m.heat === undefined) m.heat = heatOf(m.temperature === undefined ? 15 : m.temperature);
      delete m.temperature;
      (m.heat_transitions || []).forEach(function (h) {
        if (h.above > 10) h.above = heatOf(h.above);
        if (h.below > 10) h.below = heatOf(h.below);
      });
      // v19: strength (Decision 65).
      if (m.strength === undefined) { var seedS = DEFAULT_MATERIALS.find(function (x) { return x.id === m.id; }); m.strength = seedS ? seedS.strength : 1; }
      if (!m.heat_transitions) {
        var seedM = DEFAULT_MATERIALS.find(function (x) { return x.id === m.id; });
        m.heat_transitions = seedM ? clone(seedM.heat_transitions) : [];
      }
    });
    ['TIMBER', 'WATER', 'LAVA'].forEach(function (id) {
      if (!materialDef(id)) terrainLib.materials.push(clone(DEFAULT_MATERIALS.find(function (x) { return x.id === id; })));
    });
    if (!featureDef('LAVA_VENT')) terrainLib.features.push(clone(DEFAULT_TERRAIN_LIBRARY.features.find(function (x) { return x.id === 'LAVA_VENT'; })));
    terrainLib.terrains.forEach(function (t) {
      if (t.bearing === undefined) Object.assign(t, clone(DEFAULT_GROUND[t.id] || GENERIC_GROUND));
      // v15: elevation (Decision 56).
      if (t.default_elevation === undefined) t.default_elevation = (DEFAULT_GROUND[t.id] || GENERIC_GROUND).default_elevation;
      // v16: liquid bodies replace the water table (Decision 62).
      if (t.liquids === undefined) {
        t.liquids = t.water_table ? [water(t.water_table.min, t.water_table.max, 1, 0)] : [];
        delete t.water_table;
      }
      t.liquids.forEach(function (b) { if (b.liquid) { b.material = b.liquid; delete b.liquid; } });
      // v18: relief (Decision 59).
      if (!t.relief) t.relief = clone((DEFAULT_GROUND[t.id] || GENERIC_GROUND).relief);
    });
    terrainLib.terrains.forEach(function (t) {
      // v11: basements. Seeded terrains get their default depth; custom ones (e.g. Hilly) get 1.
      if (t.default_max_depth === undefined) {
        var seedD = DEFAULT_TERRAIN_LIBRARY.terrains.find(function (x) { return x.id === t.id; });
        t.default_max_depth = seedD ? seedD.default_max_depth : 1;
      }
      // v12: footprint length (north-south). Seeds and custom terrains (e.g. Hilly) follow width.
      if (t.default_max_length === undefined) t.default_max_length = Number(t.default_max_width) || 0;
    });
    saveTerrainLib();
  })();
  function saveLibrary() { storeSet(LIBRARY_KEY, JSON.stringify(libraryFactions)); }
  function saveDefaultRelations() { storeSet(DEFAULT_REL_KEY, JSON.stringify(defaultRelations)); }
  function saveTerrainLib() { storeSet(TERRAIN_LIBRARY_KEY, JSON.stringify(terrainLib)); }

  var staticDefenseLib = clone(DEFAULT_STATIC_DEFENSES);
  try {
    var rawD = storeGet(DEFENSE_LIBRARY_KEY);
    if (rawD) { var parsedD = JSON.parse(rawD); if (parsedD && parsedD.length) staticDefenseLib = parsedD; }
  } catch (e) { /* keep defaults */ }
  staticDefenseLib.forEach(function (d) {
    // v11: "crew" became "firing_points" (manned automatically from the garrison).
    if (d.firing_points === undefined) d.firing_points = d.crew !== undefined ? d.crew : 1;
    delete d.crew;
  });
  // v13 (Decision 27): the library holds emplacements only. Arrow slits, hoarding, murder
  // holes and portcullises are boundary types now; other wall/floor/door entries move into rooms.
  staticDefenseLib = staticDefenseLib.filter(function (d) { return ['ARROW_SLIT', 'HOARDING', 'MURDER_HOLE', 'PORTCULLIS'].indexOf(d.id) === -1; });
  staticDefenseLib.forEach(function (d) {
    if (d.id === 'BOILING_OIL' && d.label === 'Boiling oil') d.label = 'Oil cauldron';
    if (d.mount !== 'ROOF') d.mount = 'ROOM';
  });
  function saveDefenseLib() { storeSet(DEFENSE_LIBRARY_KEY, JSON.stringify(staticDefenseLib)); }
  saveDefenseLib();

  function loadJson(k, fallback) {
    try { var raw = storeGet(k); if (raw) { var v = JSON.parse(raw); if (v) return v; } } catch (e) { /* keep fallback */ }
    return fallback;
  }
  var featureLib = loadJson(FEATURE_LIBRARY_KEY, { features: clone(DEFAULT_FEATURES), feature_points_per_segment: 5 });
  if (!featureLib.features) featureLib = { features: clone(DEFAULT_FEATURES), feature_points_per_segment: 5 };
  var roomPrefabLib = loadJson(ROOM_PREFAB_KEY, clone(DEFAULT_ROOM_PREFABS));
  var structurePrefabLib = loadJson(STRUCTURE_PREFAB_KEY, []);
  var BOUNDARY_IDS = ['MURDER_HOLE', 'PORTCULLIS'];
  featureLib.features = featureLib.features.filter(function (f) { return BOUNDARY_IDS.indexOf(f.id) === -1; });
  roomPrefabLib.forEach(function (rp) {
    var before = rp.features.length;
    rp.features = rp.features.filter(function (f) { return BOUNDARY_IDS.indexOf(f) === -1; });
    if (rp.id === 'GATEHOUSE' && before && !rp.features.length) rp.features = ['HEARTH', 'ARMOURY_RACK'];
  });
  structurePrefabLib.forEach(function (sp) {
    normalizeStructure(sp.structure);
    if (sp.requirements && sp.requirements.length === undefined) sp.requirements.length = 1;
  });
  function saveFeatureLib() { storeSet(FEATURE_LIBRARY_KEY, JSON.stringify(featureLib)); }
  function saveRoomPrefabs() { storeSet(ROOM_PREFAB_KEY, JSON.stringify(roomPrefabLib)); }
  function saveStructurePrefabs() { storeSet(STRUCTURE_PREFAB_KEY, JSON.stringify(structurePrefabLib)); }
  saveFeatureLib(); saveRoomPrefabs(); saveStructurePrefabs();
  function featureDefOf(id) { return featureLib.features.find(function (f) { return f.id === id; }); }
  function roomPrefabDef(id) { return roomPrefabLib.find(function (r) { return r.id === id; }); }
  function featurePointsPerSegment() { return Number(featureLib.feature_points_per_segment) || 5; }
  function prefabCost(rp) { return rp.features.reduce(function (sum, f) { var d = featureDefOf(f); return sum + (d ? Number(d.cost) || 0 : 0); }, 0); }
  function staticDefenseDef(id) { return staticDefenseLib.find(function (d) { return d.id === id; }); }

  function terrainDef(id) { return terrainLib.terrains.find(function (t) { return t.id === id; }); }
  function featureDef(id) { return terrainLib.features.find(function (f) { return f.id === id; }); }
  function materialDef(id) { return terrainLib.materials.find(function (m) { return m.id === id; }); }
  function flowingMaterials() { return terrainLib.materials.filter(function (m) { return (m.traits || {}).flows > 0; }); }
  function becomesUsage(id) { return terrainLib.materials.filter(function (x) { return (x.heat_transitions || []).some(function (h) { return h.becomes === id; }); }); }
  function liquidUsage(id) { return terrainLib.terrains.filter(function (t) { return (t.liquids || []).some(function (b) { return b.material === id; }); }); }
  // Degrees saved before Decision 66, as a heat level.
  function heatOf(degrees) { return degrees <= 50 ? 1 : Math.min(10, Math.ceil(degrees / 150)); }
  function traitText(traits) { return Object.keys(traits || {}).map(function (k) { return k + ' ' + traits[k]; }).join(', '); }
  function materialUsage(id) { return terrainLib.terrains.filter(function (t) { return (t.strata || []).some(function (b) { return b.material === id; }); }); }
  function baseTerrains() { return terrainLib.terrains.filter(function (t) { return t.can_be_base; }); }
  function archetypeLabel(id) { var a = ARCHETYPES.find(function (x) { return x.id === id; }); return a ? a.label : id; }
  function roomDef(id) { return ROOM_LIBRARY.find(function (r) { return r.id === id; }); }
  function upgradeLabel(id) { var u = UPGRADE_LIBRARY.find(function (x) { return x.id === id; }); return u ? u.label : id; }

  // ---------- map state ----------
  var state = {
    mapName: 'P-f-F-c (branch test)',
    cols: 12,
    rows: 7,
    cells: {}, // "r,c" -> NODE {type:'NODE', id, node_type, is_critical_asset, owning_faction_id, structure:{archetype, columns}, fields, garrison_units}
    // "r,c" -> tile layer stack, independent of cells:
    // {terrain, feature, bridge:{owning_faction_id, hp, demolishable_by_owner}, stability, max_height, max_width, upgrade_slots, upgrades[]}
    // terrain unset = the map's base terrain; capacity fields unset = terrain default.
    tiles: {},
    roads: [], // [{a:'r,c', b:'r,c'}] - explicit connections between 8-neighbour cells
    defaultTerrain: 'FIELDS',
    ceiling: 64, // cells; ground above it is taken to continue (Decision 56)
    seed: 1, // seeds relief, strata and liquid bodies (Decision 59)
    channels: [], // [{tiles:['r,c',...], width, depth, liquid, liquid_depth}] rivers, ditches, banks (Decision 59)
    links: [], // [{a: nodeId, b: nodeId}]
    mapFactionIds: ['player'],
    faction_relations: [], // [{a, b, stance}]
    lossCriteria: [], // [{faction_id, name, rule:'ANY'|'ALL', node_ids:[]}]
    placementFactionId: '',
    selectedCell: null,
    activeTool: 'select',
    armedPreset: null,
    armedTerrain: null,
    armedFeature: null,
    linkPending: null,
    roadAnchor: null,
    bottomTab: 'paint',
    structureSel: null, // {col, level} within the selected node's structure
    eraseLayers: [], // Erase tool: layers to remove; empty = topmost layer first
    nextId: { NODE: 1 }
  };

  function key(r, c) { return r + ',' + c; }
  function parseKey(k) { return k.split(',').map(Number); }
  function cellLabel(k) { var p = parseKey(k); return 'row ' + p[0] + ' · col ' + p[1]; }
  function nodeAt(k) { return state.cells[k]; }
  function tileAt(k) { return state.tiles[k] || {}; }
  function ensureTile(k) { if (!state.tiles[k]) state.tiles[k] = {}; return state.tiles[k]; }
  function pruneTile(k) {
    var t = state.tiles[k];
    if (!t) return;
    var empty = !t.terrain && !t.feature && !t.bridge && t.elevation == null && t.stability == null && t.max_height == null && t.max_width == null && t.max_length == null && t.max_depth == null &&
      !t.upgrade_slots && !(t.upgrades && t.upgrades.length);
    if (empty) delete state.tiles[k];
  }
  function tileTerrainId(k) { return tileAt(k).terrain || state.defaultTerrain; }
  // Diagnostic views of the map (spec 24): colour each tile by elevation, ground bearing,
  // or its structure's load against capacity, instead of by terrain.
  function diagnosticColour(k) {
    var view = state.diagView || 'terrain';
    if (view === 'terrain') return null;
    var terr = terrainDef(tileTerrainId(k)) || {};
    if (view === 'elevation') {
      var e = elevationAt(k), f = Math.min(1, e / state.ceiling);
      return { colour: 'hsl(30, 15%, ' + Math.round(18 + f * 72) + '%)', title: 'Elevation ' + e + (e >= state.ceiling ? ' (capped)' : '') };
    }
    if (view === 'bearing') {
      var b = Number(terr.bearing) || 0;
      return { colour: 'hsl(' + Math.round(Math.min(1, b / 10) * 120) + ', 55%, 45%)', title: 'Bearing ' + b + ' (' + b * 8 + ' per column)' };
    }
    var node = nodeAt(k), ratio = node && window.BreachPlanner ? BreachPlanner.worstRatio(node.plan, materialMap(), plannerContext(k).bearing) : null;
    if (ratio == null) return { colour: 'rgba(255,255,255,0.55)', title: 'No structure plan' };
    return { colour: ratio > 1 ? '#111111' : ratio > 0.8 ? '#d0453a' : ratio > 0.5 ? '#e0b43a' : '#4f9a52',
      title: ratio > 1 ? 'Something fails' : 'Worst load ' + Math.round(ratio * 100) + '% of capacity' };
  }
  document.getElementById('diagView').addEventListener('change', function () { state.diagView = this.value; renderGrid(); });

  // A tile's ground height in cells: its own elevation, else its terrain's (Decision 56).
  function elevationAt(k) {
    var t = tileAt(k), d = terrainDef(tileTerrainId(k)) || {};
    return t.elevation != null ? t.elevation : Number(d.default_elevation) || 0;
  }
  function capacity(k) {
    var t = tileAt(k), d = terrainDef(tileTerrainId(k)) || {};
    return {
      stability: t.stability != null ? t.stability : Number(d.default_stability) || 0,
      max_height: t.max_height != null ? t.max_height : Number(d.default_max_height) || 0,
      max_width: t.max_width != null ? t.max_width : Number(d.default_max_width) || 0,
      max_length: t.max_length != null ? t.max_length : Number(d.default_max_length != null ? d.default_max_length : d.default_max_width) || 0,
      max_depth: t.max_depth != null ? t.max_depth : Number(d.default_max_depth) || 0 // shown to people as "dig depth"
    };
  }
  function adjacentNodeIds(k) {
    var p = parseKey(k), out = [];
    for (var dr = -1; dr <= 1; dr++) for (var dc = -1; dc <= 1; dc++) {
      var n = state.cells[key(p[0] + dr, p[1] + dc)];
      if ((dr || dc) && n) out.push(n.id);
    }
    return out;
  }
  function needsBridge(k) { var d = terrainDef(tileTerrainId(k)); return !!(d && d.needs_bridge); }
  function findNodeKeyById(id) {
    return Object.keys(state.cells).find(function (k) { return state.cells[k].id === id; });
  }
  function garrisonCount(cell) { return (cell.garrison_units || []).length; }
  function otherNodeIds(excludeId) {
    var ids = [];
    Object.keys(state.cells).forEach(function (k) {
      var d = state.cells[k];
      if (d.id && d.id !== excludeId) ids.push(d.id);
    });
    return ids;
  }
  function criticalNodes() {
    return Object.keys(state.cells).map(function (k) { return state.cells[k]; }).filter(function (d) { return d.is_critical_asset; });
  }
  function activeFactions() {
    return libraryFactions.filter(function (f) { return state.mapFactionIds.indexOf(f.id) !== -1; });
  }
  function factionName(id) { var f = libraryFactions.find(function (x) { return x.id === id; }); return f ? f.display_name : id; }
  function factionColor(id) {
    var i = libraryFactions.findIndex(function (f) { return f.id === id; });
    return i === -1 ? 'transparent' : FACTION_PALETTE[i % FACTION_PALETTE.length];
  }

  // ---------- road helpers ----------
  function isAdjacent(a, b) {
    var pa = parseKey(a), pb = parseKey(b);
    var dr = Math.abs(pa[0] - pb[0]), dc = Math.abs(pa[1] - pb[1]);
    return (dr || dc) && dr <= 1 && dc <= 1;
  }
  function roadIndex(a, b) {
    return state.roads.findIndex(function (e) { return (e.a === a && e.b === b) || (e.a === b && e.b === a); });
  }
  function roadAdjacency() {
    var adj = {};
    state.roads.forEach(function (e) {
      (adj[e.a] = adj[e.a] || []).push(e.b);
      (adj[e.b] = adj[e.b] || []).push(e.a);
    });
    return adj;
  }
  function roadBlocker(k) { return needsBridge(k) && !tileAt(k).bridge ? k : null; }
  function bridgeNotice(k) {
    var d = terrainDef(tileTerrainId(k));
    setNotice((d ? d.label : 'This terrain') + ' at ' + cellLabel(k) + ' needs a bridge before a road can cross. Use the Bridge chip.');
  }
  function tryAddRoad(a, b) {
    if (!isAdjacent(a, b) || roadIndex(a, b) !== -1) return;
    var blocked = roadBlocker(a) || roadBlocker(b);
    if (blocked) { bridgeNotice(blocked); return; }
    state.roads.push({ a: a, b: b });
    setNotice('');
    safeSave();
  }
  function hasRoadAt(k) { return state.roads.some(function (e) { return e.a === k || e.b === k; }); }
  function roadUnderWaterNotice(label) {
    setNotice('A road runs through this cell. Bridge first: erase the road here, paint ' + label + ', add a bridge, then redraw the road.');
  }
  // Tiles next to k along every channel through it; null marks a channel that only touches k.
  function channelNeighbours(k) {
    var out = [];
    state.channels.forEach(function (ch) {
      var i = ch.tiles.indexOf(k);
      if (i === -1) return;
      if (ch.tiles.length === 1) out.push(null);
      if (i > 0) out.push(ch.tiles[i - 1]);
      if (i < ch.tiles.length - 1) out.push(ch.tiles[i + 1]);
    });
    return out;
  }
  function removeChannelsAt(k) { state.channels = state.channels.filter(function (ch) { return ch.tiles.indexOf(k) === -1; }); }
  function removeRoadsAt(k) { state.roads = state.roads.filter(function (e) { return e.a !== k && e.b !== k; }); }
  function roadWarnings() {
    var w = [];
    var adj = roadAdjacency();
    Object.keys(adj).forEach(function (k) {
      if (roadBlocker(k)) w.push('road at ' + cellLabel(k) + ' crosses ' + (terrainDef(tileTerrainId(k)) || {}).label + ' with no bridge');
    });
    Object.keys(state.tiles).forEach(function (k) {
      if (state.tiles[k].bridge && !needsBridge(k)) w.push('bridge at ' + cellLabel(k) + ' sits on terrain that does not need one');
    });
    return w;
  }

  // ---------- faction relation helpers ----------
  function pairMatch(r, a, b) { return (r.a === a && r.b === b) || (r.a === b && r.b === a); }
  function defaultStance(a, b) { var r = defaultRelations.find(function (x) { return pairMatch(x, a, b); }); return r ? r.stance : ''; }
  function applyDefaultsFor(newId) {
    state.mapFactionIds.forEach(function (other) {
      if (other === newId) return;
      var stance = defaultStance(newId, other);
      if (stance && !state.faction_relations.some(function (r) { return pairMatch(r, newId, other); })) {
        state.faction_relations.push({ a: newId, b: other, stance: stance });
      }
    });
  }
  function relationsFromDefaults() {
    var out = [];
    state.mapFactionIds.forEach(function (a, i) {
      state.mapFactionIds.slice(i + 1).forEach(function (b) {
        var s = defaultStance(a, b);
        if (s) out.push({ a: a, b: b, stance: s });
      });
    });
    return out;
  }

  // ---------- load / migrate / save ----------
  function migrateNode(d) {
    d.type = 'NODE';
    if (!d.garrison_units) d.garrison_units = [];
    if (!d.hidden_from) d.hidden_from = []; // Decision 28: factions that don't know this node exists
    if (!d.fields) d.fields = {};
    if (!d.structure) {
      // v7 structure_class/floors -> segment profile; v5 had neither.
      var cls = d.structure_class || (d.node_type === 'FORT' ? 'FORT' : 'NONE');
      var h = Math.max(1, d.floors || (cls === 'FORT' ? 1 : 0));
      var cols = [];
      if (cls === 'BARRICADE') cols = [column(1)];
      else if (cls === 'WATCHTOWER') cols = [column(h)];
      else if (cls === 'FORT') cols = [column(h), column(h)];
      else if (cls === 'CASTLE') cols = [column(h), column(h), column(h)];
      d.structure = struct(cls, cols);
    }
    normalizeStructure(d.structure);
    delete d.structure_class; delete d.floors;
    // structure_slots was removed from NodeDef (Decision 25). A 0-slot node with nothing
    // built was the old way of marking a routing waypoint.
    if (d.fields.structure_slots === 0 && !structureStats(d).segments) d.node_type = 'WAYPOINT';
    delete d.fields.structure_slots;
  }

  function safeLoad() {
    try {
      // Storage key suffix -> version of the saved shape.
      var keys = [STORE_KEY].concat(LEGACY_KEYS), versions = [13, 12, 11, 10, 9, 8, 7, 6, 5], raw = null, fromVersion = 13;
      for (var ki = 0; ki < keys.length && !raw; ki++) { raw = storeGet(keys[ki]); fromVersion = versions[ki]; }
      if (!raw) return false;
      var parsed = JSON.parse(raw);
      if (!parsed || typeof parsed !== 'object') return false;
      state.mapName = parsed.mapName || state.mapName;
      state.cols = parsed.cols || state.cols;
      state.rows = parsed.rows || state.rows;
      state.cells = parsed.cells || {};
      state.tiles = parsed.tiles || {};
      state.roads = parsed.roads || [];
      Object.keys(parsed.terrain || {}).forEach(function (k) { ensureTile(k).terrain = parsed.terrain[k]; }); // v5
      if (fromVersion <= 6) {
        // v6: tile.road flags auto-joined orthogonal neighbours; keep that look as explicit edges.
        // tile.stability meant max floors, tile.footprint meant width.
        Object.keys(state.tiles).forEach(function (k) {
          var t = state.tiles[k], p = parseKey(k);
          if (t.road) {
            [[p[0] + 1, p[1]], [p[0], p[1] + 1]].forEach(function (n) {
              var nk = key(n[0], n[1]);
              if (state.tiles[nk] && state.tiles[nk].road) state.roads.push({ a: k, b: nk });
            });
          }
          if (t.stability != null) { t.max_height = t.stability; delete t.stability; }
          if (t.footprint != null) { t.max_width = t.footprint; delete t.footprint; }
        });
        Object.keys(state.tiles).forEach(function (k) { delete state.tiles[k].road; pruneTile(k); });
      }
      state.defaultTerrain = parsed.defaultTerrain || 'FIELDS';
      state.ceiling = parsed.ceiling || 64;
      state.seed = parsed.seed || 1;
      state.channels = parsed.channels || [];
      state.links = parsed.links || [];
      state.mapFactionIds = parsed.mapFactionIds || ['player'];
      state.faction_relations = parsed.faction_relations || [];
      state.lossCriteria = parsed.lossCriteria || [];
      state.nextId = parsed.nextId || state.nextId;
      Object.keys(state.cells).forEach(function (k) { migrateNode(state.cells[k]); });
      return true;
    } catch (e) { return false; }
  }

  function safeSave() {
    storeSet(STORE_KEY, JSON.stringify({
      mapName: state.mapName, cols: state.cols, rows: state.rows, cells: state.cells,
      tiles: state.tiles, roads: state.roads, defaultTerrain: state.defaultTerrain, ceiling: state.ceiling, seed: state.seed, channels: state.channels,
      links: state.links, mapFactionIds: state.mapFactionIds, faction_relations: state.faction_relations,
      lossCriteria: state.lossCriteria, nextId: state.nextId
    }));
    var s = document.getElementById('saveStatus');
    if (s) s.textContent = 'draft kept in this browser · ' + new Date().toLocaleTimeString();
    if (window.BreachDesignerHooks && window.BreachDesignerHooks.onMapChanged) window.BreachDesignerHooks.onMapChanged();
  }

  function seedExample() {
    // Deliberately Player-only: the faction library never gains a permanent entry just
    // because a demo used it. Demonstrates nodes, waypoints, links, garrisons, the tile
    // layer stack, explicit roads incl. a diagonal and a bridge, segment structures and a
    // loss-criteria group.
    state.cols = 12; state.rows = 5;
    state.mapFactionIds = ['player'];
    state.faction_relations = [];
    state.defaultTerrain = 'FIELDS'; state.ceiling = 64; state.seed = 1; state.channels = [];
    state.cells = {}; state.tiles = {}; state.roads = [];

    function node(id, type, preset, extra) {
      var p = preset ? PRESETS.find(function (x) { return x.id === preset; }) : null;
      var d = {
        type: 'NODE', id: id, node_type: type, is_critical_asset: false, owning_faction_id: '',
        structure: p ? clone(p.structure) : struct('NONE'),
        fields: p ? clone(p.fields) : {}, garrison_units: []
      };
      return Object.assign(d, extra || {});
    }
    function unit(label, extra) {
      return Object.assign({ _display_label: label, faction_id: '', can_sortie: false, patrol_route: [], delivery_target_id: '' }, extra || {});
    }

    state.cells[key(2, 1)] = node('p', 'ORIGIN', 'watchtower', { is_critical_asset: true, owning_faction_id: 'player' });
    state.cells[key(2, 4)] = node('f', 'RESOURCE', 'farm');
    state.cells[key(2, 6)] = node('F', 'FORT', 'fort_light', { garrison_units: [unit('Guard', { patrol_route: ['s2'] }), unit('Guard')] });
    state.cells[key(2, 9)] = node('c', 'ORIGIN', null, { is_critical_asset: true });
    state.cells[key(0, 3)] = node('s2', 'FORT', 'fort_heavy', { garrison_units: [unit('Knight', { can_sortie: true, patrol_route: ['F'] }), unit('Archer'), unit('Archer')] });
    state.cells[key(0, 6)] = node('w1', 'WAYPOINT', 'waypoint');

    ensureTile(key(3, 2)).terrain = 'FOREST';
    ensureTile(key(2, 6)).terrain = 'ROCKY'; // under the Fort - layers are independent
    ensureTile(key(4, 9)).terrain = 'MOUNTAIN';
    ensureTile(key(4, 9)).feature = 'ORE_VEIN';
    [key(3, 7), key(4, 7)].forEach(function (k) { ensureTile(k).terrain = 'WATER'; });
    ensureTile(key(3, 7)).bridge = { owning_faction_id: '', hp: 40, demolishable_by_owner: false };
    var farmTile = ensureTile(key(2, 4));
    farmTile.feature = 'ARABLE';
    farmTile.upgrade_slots = 1;
    farmTile.upgrades = ['GUARD_BARRACKS']; // a barracks at a farm, no fort needed

    // p -> f along row 2, plus a separate diagonal spur p -> s2 that does NOT join it,
    // and a road over the bridge at (3,7).
    state.roads = [
      { a: key(2, 1), b: key(2, 2) }, { a: key(2, 2), b: key(2, 3) }, { a: key(2, 3), b: key(2, 4) },
      { a: key(2, 1), b: key(1, 2) }, { a: key(1, 2), b: key(0, 3) },
      { a: key(3, 6), b: key(3, 7) }, { a: key(3, 7), b: key(3, 8) }
    ];

    state.links = [
      { a: 'p', b: 'f' }, { a: 'f', b: 'F' }, { a: 'F', b: 'c' },
      { a: 'p', b: 's2' }, { a: 's2', b: 'w1' }, { a: 'w1', b: 'F' }
    ];
    state.lossCriteria = [{ faction_id: 'player', name: 'Home', rule: 'ANY', node_ids: ['p'] }];
    state.nextId = { NODE: 7 };
  }

  var restored = safeLoad();
  if (!restored) {
    document.getElementById('saveStatus').textContent = 'new session';
    seedExample();
  }

  // ---------- top-level views ----------
  function showView(v) {
    document.querySelectorAll('.view-btn[data-view]').forEach(function (b) { b.setAttribute('data-active', b.getAttribute('data-view') === v ? 'true' : 'false'); });
    document.getElementById('viewLanes').hidden = v !== 'lanes';
    document.getElementById('viewFactions').hidden = v !== 'factions';
    document.getElementById('viewTerrain').hidden = v !== 'terrain';
    document.getElementById('viewDefenses').hidden = v !== 'defenses';
    document.getElementById('viewStructures').hidden = v !== 'structures';
    storeSet(VIEW_KEY, v);
    if (v === 'lanes') renderLanesAll();
    if (v === 'factions') renderFactionsView();
    if (v === 'terrain') renderTerrainView();
    if (v === 'defenses') renderDefensesView();
    if (v === 'structures') renderStructuresView();
  }
  document.querySelectorAll('.view-btn[data-view]').forEach(function (b) {
    b.addEventListener('click', function () { showView(b.getAttribute('data-view')); });
  });
  document.getElementById('gotoFactions').addEventListener('click', function () { showView('factions'); });

  // ---------- palette: presets / units ----------
  function renderPresets() {
    var list = document.getElementById('presetList');
    list.innerHTML = '';
    PRESETS.forEach(function (p) {
      var el = document.createElement('div');
      el.className = 'preset-item';
      el.tabIndex = 0;
      el.innerHTML = '<span class="name">' + esc(p.name) + '</span><span class="meta">' + esc(p.meta) + '</span>';
      el.addEventListener('click', function () { setActiveTool('NODE'); state.armedPreset = p; });
      list.appendChild(el);
    });
  }

  function renderUnits() {
    var list = document.getElementById('unitList');
    list.innerHTML = '';
    UNIT_LIBRARY.forEach(function (u) {
      var el = document.createElement('div');
      el.className = 'unit-chip';
      el.draggable = true;
      el.innerHTML = '<span>' + esc(u) + '</span><span class="qty">drag &rarr;</span>';
      el.addEventListener('dragstart', function (ev) { ev.dataTransfer.setData('text/unit', u); });
      el.addEventListener('click', function () {
        var cell = state.selectedCell ? nodeAt(state.selectedCell) : null;
        if (cell) addGarrisonUnit(cell, u);
      });
      list.appendChild(el);
    });
  }

  function addGarrisonUnit(cell, label) {
    cell.garrison_units.push({
      _display_label: label, faction_id: cell.owning_faction_id || '', can_sortie: false, patrol_route: [], delivery_target_id: ''
    });
    safeSave(); renderInspector(); renderCounts();
  }

  function renderPlacementFactionSelect() {
    var sel = document.getElementById('placementFaction');
    sel.innerHTML = ['<option value="">(none)</option>'].concat(
      activeFactions().map(function (f) { return '<option value="' + esc(f.id) + '"' + (f.id === state.placementFactionId ? ' selected' : '') + '>' + esc(f.display_name) + '</option>'; })
    ).join('');
  }
  document.getElementById('placementFaction').addEventListener('change', function () { state.placementFactionId = this.value; });

  // ---------- bottom panel: paint strip / structure editor ----------
  var noticeTimer = null;
  function setNotice(msg) {
    var el = document.getElementById('stripNotice');
    el.textContent = msg;
    clearTimeout(noticeTimer);
    if (msg) noticeTimer = setTimeout(function () { el.textContent = ''; }, 5000);
  }

  function setBottomTab(tab) {
    var node = state.selectedCell ? nodeAt(state.selectedCell) : null;
    if (node && node.node_type === 'WAYPOINT') node = null; // waypoints hold no structure
    if ((tab === 'structure' || tab === 'plan') && !node) tab = 'paint';
    if (tab === 'structure') tab = 'plan'; // retired (Decision 67)
    state.bottomTab = tab;
    var pBtn = document.getElementById('bottomTabPlan');
    pBtn.setAttribute('data-active', tab === 'plan' ? 'true' : 'false');
    pBtn.disabled = !node;
    pBtn.textContent = node ? 'Plan · ' + node.id : 'Plan';
    document.getElementById('planEditor').hidden = tab !== 'plan';
    document.getElementById('bottomTabPaint').setAttribute('data-active', tab === 'paint' ? 'true' : 'false');
    var sBtn = document.getElementById('bottomTabStructure');
    sBtn.setAttribute('data-active', tab === 'structure' ? 'true' : 'false');
    sBtn.disabled = !node;
    sBtn.hidden = true; // the side-on editor is retired; plans replace it (Decision 67)
    sBtn.textContent = node ? 'Structure · ' + node.id : 'Structure';
    document.getElementById('layersStrip').hidden = tab !== 'paint';
    document.getElementById('structureEditor').hidden = tab !== 'structure';
    if (tab !== 'structure' && tab !== 'plan') state.expanded = false;
    var ex = document.getElementById('expandBtn');
    ex.hidden = tab !== 'structure' && tab !== 'plan';
    ex.textContent = state.expanded ? 'Collapse ⤡' : 'Expand ⤢';
    document.querySelector('#viewLanes .workspace').classList.toggle('expanded', !!state.expanded);
    if (tab === 'paint') renderLayersStrip(); else if (tab === 'plan') renderPlanEditor(); else renderStructureEditor();
  }
  // The structure planner (spec 24, planner.js) for the selected node, on its tile's ground.
  // The planner's view of a node (Decision 68): its plan covers the node's footprint, in
  // cells measured from the node's own tile; each cell stands on its own tile's ground.
  function plannerContext(k) {
    var CELLS = BreachPlannerTools.CELLS_PER_TILE, own = parseKey(k), area = footprintOf(k);
    function tileOfCell(x, y) { return key(own[0] + Math.floor(y / CELLS), own[1] + Math.floor(x / CELLS)); }
    function groundOf(tk) {
      var terr = terrainDef(tileTerrainId(tk)) || {}, tile = tileAt(tk);
      return { terr: terr, bearing: Number(terr.bearing) || 0, digDepth: tile.max_depth != null ? Number(tile.max_depth) : Number(terr.dig_depth) || 0 };
    }
    var rows = area.map(function (a) { return parseKey(a)[0] - own[0]; }), cols = area.map(function (a) { return parseKey(a)[1] - own[1]; });
    var grounds = area.map(groundOf), here = groundOf(k);
    return {
      materials: terrainLib.materials,
      material: materialDef,
      materialMap: materialMap,
      bearing: function (x, y) { return groundOf(tileOfCell(x, y)).bearing; },
      groundColour: here.terr.color || '#8f9a4f',
      groundColourAt: function (x, y) { return (groundOf(tileOfCell(x, y)).terr.color) || '#8f9a4f'; },
      // The footprint's cells: bounds, and whether a cell is on one of its tiles.
      bounds: { x0: Math.min.apply(null, cols) * CELLS, y0: Math.min.apply(null, rows) * CELLS,
        x1: (Math.max.apply(null, cols) + 1) * CELLS - 1, y1: (Math.max.apply(null, rows) + 1) * CELLS - 1 },
      inFootprint: function (x, y) { return area.indexOf(tileOfCell(x, y)) !== -1; },
      // Every level from the deepest dig depth up to the map's ceiling over its ground.
      digDepth: Math.max.apply(null, grounds.map(function (g) { return g.digDepth; })),
      digDepthAt: function (x, y) { return groundOf(tileOfCell(x, y)).digDepth; },
      maxLevel: Math.max(1, state.ceiling - elevationAt(k)),
      stratumColour: function (level, x, y) {
        var terr = x == null ? here.terr : groundOf(tileOfCell(x, y)).terr, m = materialDef(stratumAt(terr, -level));
        return m ? m.color : '#6b5440';
      },
      save: function () { safeSave(); renderGrid(); }
    };
  }
  // The terrain's typical stratum `depth` cells down (1 = just below the surface): its
  // bands from the surface down, each taken at the middle of its range.
  function stratumAt(terr, depth) {
    var bands = terr.strata || [], reached = 0;
    for (var i = 0; i < bands.length; i++) {
      reached += Math.round(((Number(bands[i].min) || 0) + (Number(bands[i].max) || 0)) / 2);
      if (depth <= reached) return bands[i].material;
    }
    return bands.length ? bands[bands.length - 1].material : 'SOIL';
  }
  function materialMap() {
    var out = {};
    terrainLib.materials.forEach(function (m) { out[m.id] = { weight: Number(m.weight) || 0, strength: Number(m.strength) || 0, span: Number(m.span) || 0, flows: Number((m.traits || {}).flows) || 0 }; });
    return out;
  }
  function renderPlanEditor() {
    var k = state.selectedCell, node = k ? nodeAt(k) : null;
    if (!node || !window.BreachPlanner) return;
    BreachPlanner.render(document.getElementById('planEditor'), node, plannerContext(k));
  }
  document.getElementById('expandBtn').addEventListener('click', function () { state.expanded = !state.expanded; setBottomTab(state.bottomTab === 'plan' ? 'plan' : 'structure'); });
  document.getElementById('bottomTabPlan').addEventListener('click', function () { setBottomTab('plan'); });
  var resizeTimer = null;
  window.addEventListener('resize', function () {
    // Structure cells are sized to the available width, so re-fit them after a resize.
    clearTimeout(resizeTimer);
    resizeTimer = setTimeout(function () { if (state.bottomTab === 'structure') renderStructureEditor(); }, 150);
  });
  document.getElementById('bottomTabPaint').addEventListener('click', function () { setBottomTab('paint'); });
  document.getElementById('bottomTabStructure').addEventListener('click', function () { setBottomTab('structure'); });

  function renderLayersStrip() {
    var host = document.getElementById('layersStrip');
    var html = '<div class="layer-row"><span class="row-label">Terrain</span>';
    terrainLib.terrains.forEach(function (t) {
      var active = state.activeTool === 'TERRAIN' && state.armedTerrain === t.id;
      html += '<button type="button" class="chip" data-kind="terrain" data-id="' + esc(t.id) + '" data-active="' + active + '">' +
        '<span class="dot" style="background:' + esc(t.color) + '">' + esc(t.glyph) + '</span>' + esc(t.label) + '</button>';
    });
    html += '</div><div class="layer-row"><span class="row-label">Features</span>';
    terrainLib.features.forEach(function (f) {
      var active = state.activeTool === 'FEATURE' && state.armedFeature === f.id;
      html += '<button type="button" class="chip" data-kind="feature" data-id="' + esc(f.id) + '" data-active="' + active + '">' +
        '<span class="dot" style="background:var(--ink-dim)">' + esc(f.glyph) + '</span>' + esc(f.label) + '</button>';
    });
    html += '</div><div class="layer-row"><span class="row-label">Routes</span>' +
      '<button type="button" class="chip" data-kind="road" data-active="' + (state.activeTool === 'ROAD') + '"><span class="dot" style="background:var(--road)">&#9552;</span>Road (drag cell to cell)</button>' +
      '<button type="button" class="chip" data-kind="bridge" data-active="' + (state.activeTool === 'BRIDGE') + '"><span class="dot" style="background:var(--bridge)">&#9636;</span>Bridge (water, ravine)</button>' +
      '<button type="button" class="chip" data-kind="channel" data-active="' + (state.activeTool === 'CHANNEL') + '"><span class="dot" style="background:var(--channel)">&#8776;</span>Channel: river, ditch, bank (drag tile to tile)</button>' +
      '</div>';
    var eraseOn = state.activeTool === 'eraser';
    html += '<div class="layer-row"><span class="row-label">Erase</span>' +
      '<button type="button" class="chip" data-kind="erase-top" data-active="' + (eraseOn && !state.eraseLayers.length) + '"><span class="dot" style="background:var(--ink-dim)">&times;</span>Top layer first</button>';
    eraseLayerList().forEach(function (L) {
      html += '<button type="button" class="chip" data-kind="erase" data-id="' + L[0] + '" data-active="' + (eraseOn && state.eraseLayers.indexOf(L[0]) !== -1) + '">' + esc(L[1]) + '</button>';
    });
    html += '<span class="hint" style="margin:0;">' + (eraseOn && state.eraseLayers.length ? 'Erases only the picked layers (click or drag).' : 'Pick layers to erase only those; toggle several.') + '</span></div>';
    host.innerHTML = html;
    host.querySelectorAll('.chip').forEach(function (chip) {
      chip.addEventListener('click', function () {
        var kind = chip.getAttribute('data-kind'), id = chip.getAttribute('data-id');
        if (kind === 'erase-top' || kind === 'erase') {
          if (state.activeTool !== 'eraser') setActiveTool('eraser');
          if (kind === 'erase-top') state.eraseLayers = [];
          else {
            var at = state.eraseLayers.indexOf(id);
            if (at === -1) state.eraseLayers.push(id); else state.eraseLayers.splice(at, 1);
          }
          renderLayersStrip();
          return;
        }
        if (kind === 'terrain') { setActiveTool('TERRAIN'); state.armedTerrain = id; }
        else if (kind === 'feature') { setActiveTool('FEATURE'); state.armedFeature = id; }
        else if (kind === 'bridge') setActiveTool('BRIDGE');
        else if (kind === 'channel') setActiveTool('CHANNEL');
        else setActiveTool('ROAD');
        renderLayersStrip();
      });
    });
  }

  // ---------- structure model v4: 2D side view (Decision 27) ----------
  // Every structure fight is one side-on plane; attackers enter and leave at the left or
  // right end, and each incoming link is assigned a side (node.approaches).
  //   cells       'c,l'   column c (left -> right), level l (0 ground, negative basement)
  //   boundaries  'W:c,l' wall on the left side of column c (ends are exterior)
  //               'F:c,l' floor between level l and l+1
  //               'R:c'   roof hatch above the top of column c
  // Each boundary blocks movement / projectiles / sight "from" a set of sides. Presets are
  // written in OUTSIDE/INSIDE terms and resolved against the boundary's geometry.
  var REINFORCEMENTS = [
    { id: 'TIMBER', label: 'Timber', hp_mult: 1 },
    { id: 'STONE', label: 'Stone', hp_mult: 2 },
    { id: 'REINFORCED_STONE', label: 'Reinforced stone', hp_mult: 3 }
  ];
  var FORTIFICATIONS = [
    { id: 'NONE', label: 'Open', delay: 0 },
    { id: 'LOCKED', label: 'Locked', delay: 1 },
    { id: 'BARRED', label: 'Barred', delay: 2 },
    { id: 'REINFORCED', label: 'Reinforced', delay: 4 }
  ];
  var ROOF_TYPES = [
    { id: 'FLAT', label: 'Flat (walkable, takes emplacements)', glyph: '▬' },
    { id: 'PITCHED', label: 'Pitched (sheltered, no emplacements)', glyph: '⌂' },
    { id: 'OPEN', label: 'Open (no roof)', glyph: '·' }
  ];
  var RANGED_UNITS = ['Archer'];
  function propsList() { return [['move', 'Movement'], ['proj', 'Projectiles'], ['sight', 'Sight']]; } // function: used during load-time migration

  function boundaryPresets() {
    // A function, not a var, so load-time migration can use it before vars are assigned.
    return {
      WALL: [
        { id: 'SOLID_WALL', label: 'Solid wall', move: 'BOTH', proj: 'BOTH', sight: 'BOTH', glyph: '' },
        { id: 'ARROW_SLIT', label: 'Arrow slit', move: 'BOTH', proj: 'OUTSIDE', sight: 'OUTSIDE', glyph: '⋮' },
        { id: 'DOOR', label: 'Door', move: 'OUTSIDE', proj: 'BOTH', sight: 'BOTH', door: true, glyph: '▯' },
        { id: 'PORTCULLIS', label: 'Portcullis', move: 'BOTH', proj: 'NONE', sight: 'NONE', door: true, glyph: '▦' },
        { id: 'DOORWAY', label: 'Open doorway', move: 'NONE', proj: 'NONE', sight: 'NONE', glyph: '░' }
      ],
      FLOOR: [
        { id: 'SOLID_FLOOR', label: 'Solid floor', move: 'BOTH', proj: 'BOTH', sight: 'BOTH', glyph: '' },
        { id: 'STAIRS', label: 'Stairs', move: 'NONE', proj: 'BOTH', sight: 'NONE', opening: true, glyph: '⇕' },
        { id: 'HATCH', label: 'Hatch / ladder', move: 'NONE', proj: 'BOTH', sight: 'BOTH', opening: true, glyph: '⤒' },
        { id: 'MURDER_HOLE', label: 'Murder hole', move: 'BOTH', proj: 'OUTSIDE', sight: 'OUTSIDE', glyph: '◘' }
      ],
      ROOF: [
        { id: 'ROOF_SOLID', label: 'No way up', move: 'BOTH', proj: 'BOTH', sight: 'BOTH', glyph: '' },
        { id: 'ROOF_HATCH', label: 'Hatch / ladder up', move: 'NONE', proj: 'BOTH', sight: 'BOTH', opening: true, glyph: '⤒' },
        { id: 'ROOF_STAIRS', label: 'Stairs up', move: 'NONE', proj: 'BOTH', sight: 'NONE', opening: true, glyph: '⇕' }
      ]
    };
  }
  function presetDef(kind, id) { return boundaryPresets()[kind].find(function (p) { return p.id === id; }); }
  function bKind(bk) { return bk.charAt(0) === 'W' ? 'WALL' : bk.charAt(0) === 'F' ? 'FLOOR' : 'ROOF'; }
  function bSides(bk) { return bKind(bk) === 'WALL' ? ['LEFT', 'RIGHT'] : ['ABOVE', 'BELOW']; }
  function parseB(bk) { var p = bk.slice(2).split(',').map(Number); return { c: p[0], l: p.length > 1 ? p[1] : null }; }
  function sKey(c, l) { return c + ',' + l; }
  function sideLabel(side) { return { LEFT: '◀ left', RIGHT: 'right ▶', ABOVE: '▲ above', BELOW: '▼ below' }[side]; }

  // --- legacy helpers kept for migrating v1 columns -> v2 'c,l' -> v3 footprint -> v4 ---
  function cellKey(x, y, l) { return x + ',' + y + ',' + l; }
  function parseCell(k) { return k.split(',').map(Number); }
  function cellWalls(x, y, l) {
    return { N: 'H:' + cellKey(x, y, l), S: 'H:' + cellKey(x, y + 1, l), W: 'V:' + cellKey(x, y, l), E: 'V:' + cellKey(x + 1, y, l) };
  }
  function strongerMaterial(a, b) {
    var order = ['TIMBER', 'STONE', 'REINFORCED_STONE'];
    return order.indexOf(b) > order.indexOf(a) ? b : a;
  }
  function applyLegacyRoom(sg, room) {
    if (!room) return;
    if (room === 'ARROW_SLIT') { sg.faces.FRONT.defenses.push('ARROW_SLIT'); return; }
    var map = { BARRACKS: 'BARRACKS', STORES: 'STOREROOM', KITCHEN: 'KITCHEN', GATEHOUSE: 'GATEHOUSE' };
    var prefab = DEFAULT_ROOM_PREFABS.find(function (p) { return p.id === map[room]; });
    if (prefab) { sg.label = prefab.label; sg.features = prefab.features.slice(); }
    else sg.label = room.charAt(0) + room.slice(1).toLowerCase();
  }
  function legacyFace() { return { reinforcement: 'TIMBER', defenses: [] }; }
  function columnsToV2(st) {
    var segs = {}, roofs = {};
    st.columns.forEach(function (col, c) {
      col.forEach(function (old, l) {
        var sg = { label: '', features: [], faces: { LEFT: legacyFace(), RIGHT: legacyFace(), FRONT: legacyFace() } };
        applyLegacyRoom(sg, old.room);
        (old.features || []).forEach(function (f) { sg.features.push(f); });
        (old.defenses || []).forEach(function (d) {
          if (d.face === 'ROOF') { if (!roofs[c]) roofs[c] = { type: 'FLAT', defenses: [] }; roofs[c].defenses.push(d.def_id); }
          else if (sg.faces[d.face]) sg.faces[d.face].defenses.push(d.def_id);
        });
        segs[c + ',' + l] = sg;
      });
    });
    st.segments = segs;
    st.roofs = Object.assign(st.roofs || {}, roofs);
    delete st.columns;
    if (!st.connections) {
      var conns = [], cols = {};
      Object.keys(segs).forEach(function (k) { cols[k.split(',')[0]] = true; });
      var colList = Object.keys(cols).map(Number).sort(function (a, b) { return a - b; });
      colList.forEach(function (c) {
        for (var l = 0; segs[c + ',' + (l + 1)]; l++) conns.push({ a: c + ',' + l, b: c + ',' + (l + 1), kind: 'stairs' });
        if (segs[c + ',0'] && segs[(c + 1) + ',0']) conns.push({ a: c + ',0', b: (c + 1) + ',0', kind: 'door' });
      });
      if (colList.length) conns.push({ a: colList[0] + ',0', b: 'EXT_L', kind: 'door' });
      st.connections = conns;
    }
  }
  function v2ToV3(st) {
    var old = st.segments, segs = {}, walls = {}, floors = {}, roofs = {}, murder = [], portc = [];
    function mergeFace(wk, face) {
      if (!face) return;
      var w = walls[wk] || { material: 'TIMBER', door: null, mods: [] };
      w.material = strongerMaterial(w.material, face.reinforcement || 'TIMBER');
      (face.defenses || []).forEach(function (d) { w.mods.push(d); });
      walls[wk] = w;
    }
    Object.keys(old).forEach(function (k) {
      var p = k.split(',').map(Number), c = p[0], l = p[1], sg = old[k];
      var feats = [];
      (sg.features || []).forEach(function (f) {
        if (f === 'MURDER_HOLE') murder.push([c, l]); else if (f === 'PORTCULLIS') portc.push([c, l]); else feats.push(f);
      });
      segs[cellKey(c, 0, l)] = { label: sg.label || '', features: feats };
      if (sg.faces) {
        mergeFace('V:' + cellKey(c, 0, l), sg.faces.LEFT);
        mergeFace('V:' + cellKey(c + 1, 0, l), sg.faces.RIGHT);
        mergeFace('H:' + cellKey(c, 1, l), sg.faces.FRONT);
      }
    });
    (st.connections || []).forEach(function (x) {
      var a = x.a.split(',').map(Number), fort = x.fortification || 'NONE';
      if (x.b.indexOf('EXT') === 0) {
        var wk = 'V:' + cellKey(x.b === 'EXT_L' ? a[0] : a[0] + 1, 0, a[1]);
        var w = walls[wk] || { material: 'TIMBER', door: null, mods: [] };
        w.door = { fortification: fort, mods: [] }; walls[wk] = w;
        return;
      }
      var b = x.b.split(',').map(Number);
      if (x.kind === 'stairs') floors[cellKey(a[0], 0, Math.min(a[1], b[1]))] = { opening: 'STAIRS', fortification: fort, mods: [] };
      else {
        var wk2 = 'V:' + cellKey(Math.max(a[0], b[0]), 0, a[1]);
        var w2 = walls[wk2] || { material: 'TIMBER', door: null, mods: [] };
        w2.door = { fortification: fort, mods: [] }; walls[wk2] = w2;
      }
    });
    Object.keys(st.roofs || {}).forEach(function (c) {
      var r = st.roofs[c];
      roofs[c + ',0'] = { type: r.type || 'FLAT', access: (r.type || 'FLAT') === 'FLAT' ? 'HATCH' : 'NONE', defenses: (r.defenses || []).slice() };
    });
    murder.forEach(function (m) {
      var fk = cellKey(m[0], 0, m[1] - 1);
      if (segs[fk]) { floors[fk] = floors[fk] || { opening: 'NONE', fortification: 'NONE', mods: [] }; floors[fk].mods.push('MURDER_HOLE'); }
    });
    portc.forEach(function (pc) {
      var cw = cellWalls(pc[0], 0, pc[1]);
      var wk3 = ['W', 'E', 'S', 'N'].map(function (d) { return cw[d]; }).find(function (x) { return walls[x] && walls[x].door; });
      if (wk3) walls[wk3].door.mods.push({ def_id: 'PORTCULLIS', side: 'INNER' });
    });
    st.segments = segs; st.walls = walls; st.floors = floors; st.roofs = roofs;
    delete st.connections;
  }
  function v3ToV4(st) {
    // Collapse the compass footprint onto one side-on plane: keep the northernmost row per column.
    var rowOf = {};
    Object.keys(st.segments).forEach(function (k) { var p = parseCell(k); rowOf[p[0]] = rowOf[p[0]] === undefined ? p[1] : Math.min(rowOf[p[0]], p[1]); });
    var segs = {}, bounds = {}, roofs = {};
    Object.keys(st.segments).forEach(function (k) {
      var p = parseCell(k);
      if (p[1] !== rowOf[p[0]]) return;
      segs[sKey(p[0], p[2])] = { label: st.segments[k].label || '', features: st.segments[k].features || [], emplacements: [] };
    });
    Object.keys(st.walls || {}).forEach(function (wk) {
      if (wk.charAt(0) !== 'V') return;
      var p = wk.slice(2).split(',').map(Number), x = p[0], y = p[1], l = p[2];
      if (y !== rowOf[x] && y !== rowOf[x - 1]) return;
      var w = st.walls[wk], preset = null;
      if (w.door) preset = (w.door.mods || []).some(function (m) { return m.def_id === 'PORTCULLIS'; }) ? 'PORTCULLIS' : 'DOOR';
      else if ((w.mods || []).indexOf('ARROW_SLIT') !== -1) preset = 'ARROW_SLIT';
      (w.mods || []).forEach(function (id) {
        if (id === 'BOILING_OIL') { var inner = segs[sKey(x, l)] || segs[sKey(x - 1, l)]; if (inner) inner.emplacements.push('BOILING_OIL'); }
      });
      if (preset || (w.material && w.material !== 'TIMBER')) {
        bounds['W:' + sKey(x, l)] = { preset: preset || 'SOLID_WALL', material: w.material || 'TIMBER', fortification: w.door ? w.door.fortification || 'NONE' : 'NONE' };
      }
    });
    Object.keys(st.walls || {}).forEach(function (wk) {
      // North/south walls don't exist side-on. Move their arrow slits to the nearest free end wall.
      if (wk.charAt(0) !== 'H' || (st.walls[wk].mods || []).indexOf('ARROW_SLIT') === -1) return;
      var p = wk.slice(2).split(',').map(Number), x = p[0], l = p[2];
      if (!segs[sKey(x, l)]) return;
      [[x, !segs[sKey(x - 1, l)]], [x + 1, !segs[sKey(x + 1, l)]]].some(function (e) {
        var bk = 'W:' + sKey(e[0], l);
        if (!e[1] || bounds[bk]) return false;
        bounds[bk] = { preset: 'ARROW_SLIT', material: st.walls[wk].material || 'TIMBER', fortification: 'NONE' };
        return true;
      });
    });
    Object.keys(st.floors || {}).forEach(function (fk) {
      var p = parseCell(fk);
      if (p[1] !== rowOf[p[0]]) return;
      var f = st.floors[fk];
      // One preset per floor section: keep a way through (stairs/hatch) over a murder hole on the same section.
      var preset = f.opening === 'STAIRS' ? 'STAIRS' : f.opening === 'HATCH' ? 'HATCH' : (f.mods || []).indexOf('MURDER_HOLE') !== -1 ? 'MURDER_HOLE' : null;
      if (preset) bounds['F:' + sKey(p[0], p[2])] = { preset: preset, fortification: f.fortification || 'NONE' };
    });
    Object.keys(st.roofs || {}).forEach(function (rk) {
      var p = rk.split(',').map(Number);
      if (p[1] !== rowOf[p[0]]) return;
      var r = st.roofs[rk];
      roofs[p[0]] = { type: r.type || 'FLAT', emplacements: (r.defenses || []).filter(function (id) { return id === 'BALLISTA' || id === 'TREBUCHET'; }) };
      if (r.access === 'HATCH' || r.access === 'STAIRS') bounds['R:' + p[0]] = { preset: r.access === 'HATCH' ? 'ROOF_HATCH' : 'ROOF_STAIRS', fortification: 'NONE' };
    });
    st.segments = segs; st.boundaries = bounds; st.roofs = roofs;
    delete st.walls; delete st.floors;
  }
  function normalizeStructure(st) {
    if (!st.boundaries) {
      if (st.columns) columnsToV2(st);
      st.segments = st.segments || {};
      var keys = Object.keys(st.segments);
      if (!st.walls && (keys.some(function (k) { return st.segments[k].faces; }) || st.connections)) v2ToV3(st);
      if (st.walls) v3ToV4(st);
      else if (!st.boundaries) { st.boundaries = {}; st.roofs = {}; keys.forEach(function (k) { st.segments[k].emplacements = st.segments[k].emplacements || []; }); }
      ensureEntrance(st);
    }
    st.segments = st.segments || {}; st.boundaries = st.boundaries || {}; st.roofs = st.roofs || {};
    Object.keys(st.segments).forEach(function (k) { var sg = st.segments[k]; sg.features = sg.features || []; sg.emplacements = sg.emplacements || []; });
    colsWithTop(st).forEach(function (c) { roofOf(st, c); });
    pruneBoundaries(st);
    return st;
  }
  function ensureEntrance(st) {
    if (!Object.keys(st.segments).length) return;
    var has = Object.keys(st.boundaries).some(function (bk) {
      var info = bInfo(st, bk); return info.exists && info.exterior && info.level === 0 && isPassable(st, bk);
    });
    if (has) return;
    var ground = Object.keys(st.segments).map(function (k) { return k.split(',').map(Number); }).filter(function (p) { return p[1] === 0; }).map(function (p) { return p[0]; });
    if (!ground.length) return;
    var left = Math.min.apply(null, ground);
    st.boundaries['W:' + sKey(left, 0)] = { preset: 'DOOR', fortification: 'NONE', material: 'TIMBER' };
  }

  // --- geometry ---
  function has(st, c, l) { return !!st.segments[sKey(c, l)]; }
  function colTop(st, c) { var t = -1; Object.keys(st.segments).forEach(function (k) { var p = k.split(',').map(Number); if (p[0] === c && p[1] > t) t = p[1]; }); return t; }
  function colBottom(st, c) { var b = 0; Object.keys(st.segments).forEach(function (k) { var p = k.split(',').map(Number); if (p[0] === c && p[1] < b) b = p[1]; }); return b; }
  function colsWithTop(st) {
    var cols = {};
    Object.keys(st.segments).forEach(function (k) { var p = k.split(',').map(Number); if (p[1] >= 0) cols[p[0]] = true; });
    return Object.keys(cols).map(Number).sort(function (a, b) { return a - b; });
  }
  function roofOf(st, c) { if (!st.roofs[c]) st.roofs[c] = { type: 'FLAT', emplacements: [] }; return st.roofs[c]; }
  function bInfo(st, bk) {
    var kind = bKind(bk), p = parseB(bk);
    if (kind === 'WALL') {
      var a = has(st, p.c - 1, p.l), b = has(st, p.c, p.l);
      return { kind: kind, exists: a || b, exterior: a !== b, exteriorSide: a !== b ? (b ? 'LEFT' : 'RIGHT') : null, level: p.l, col: p.c,
        sides: [sKey(p.c - 1, p.l), sKey(p.c, p.l)] };
    }
    if (kind === 'FLOOR') return { kind: kind, exists: has(st, p.c, p.l) && has(st, p.c, p.l + 1), exterior: false, level: p.l, col: p.c, sides: [sKey(p.c, p.l + 1), sKey(p.c, p.l)] };
    var top = colTop(st, p.c);
    return { kind: kind, exists: top >= 0, exterior: true, exteriorSide: 'ABOVE', level: top, col: p.c, sides: ['ROOF:' + p.c, sKey(p.c, top)] };
  }
  function outsideSide(st, bk) {
    var info = bInfo(st, bk), stored = st.boundaries[bk];
    if (info.kind === 'WALL') {
      if (info.exterior) return info.exteriorSide;
      if (stored && stored.outside) return stored.outside;
      // internal wall: default to the side facing the nearer end of the structure
      var cols = Object.keys(st.segments).map(function (k) { return Number(k.split(',')[0]); });
      var mid = (Math.min.apply(null, cols) + Math.max.apply(null, cols) + 1) / 2;
      return info.col <= mid ? 'LEFT' : 'RIGHT';
    }
    return info.kind === 'FLOOR' ? 'BELOW' : 'ABOVE';
  }
  function defaultPreset(kind) { return kind === 'WALL' ? 'SOLID_WALL' : kind === 'FLOOR' ? 'SOLID_FLOOR' : 'ROOF_SOLID'; }
  function resolveDirs(spec, sides, outside) {
    var inside = sides[0] === outside ? sides[1] : sides[0];
    return spec === 'BOTH' ? sides.slice() : spec === 'OUTSIDE' ? [outside] : spec === 'INSIDE' ? [inside] : [];
  }
  function boundary(st, bk) {
    // Resolved view: {preset, move, proj, sight, fortification, material, outside, custom}
    var kind = bKind(bk), stored = st.boundaries[bk] || {}, sides = bSides(bk), outside = outsideSide(st, bk);
    var presetId = stored.preset || defaultPreset(kind);
    var out = { preset: presetId, fortification: stored.fortification || 'NONE', material: stored.material || 'TIMBER', outside: outside, custom: presetId === 'CUSTOM' };
    if (out.custom) propsList().forEach(function (pr) { out[pr[0]] = (stored[pr[0]] || []).slice(); });
    else {
      var def = presetDef(kind, presetId) || presetDef(kind, defaultPreset(kind));
      propsList().forEach(function (pr) { out[pr[0]] = resolveDirs(def[pr[0]], sides, outside); });
    }
    return out;
  }
  function ensureB(st, bk) { if (!st.boundaries[bk]) st.boundaries[bk] = { preset: defaultPreset(bKind(bk)), fortification: 'NONE' }; return st.boundaries[bk]; }
  function isDoorType(st, bk) { var b = boundary(st, bk), def = presetDef(bKind(bk), b.preset); return !!(def && def.door); }
  function isPassable(st, bk) {
    var b = boundary(st, bk);
    return isDoorType(st, bk) || b.move.length < bSides(bk).length;
  }
  function pruneBoundaries(st) {
    Object.keys(st.boundaries).forEach(function (bk) { if (!bInfo(st, bk).exists) delete st.boundaries[bk]; });
    Object.keys(st.roofs).forEach(function (c) { if (colTop(st, Number(c)) < 0) delete st.roofs[c]; });
  }
  function allBoundaryKeys(st) {
    var keys = {};
    Object.keys(st.segments).forEach(function (k) {
      var p = k.split(',').map(Number);
      keys['W:' + sKey(p[0], p[1])] = true; keys['W:' + sKey(p[0] + 1, p[1])] = true;
      if (has(st, p[0], p[1] + 1)) keys['F:' + sKey(p[0], p[1])] = true;
      if (p[1] >= 0 && colTop(st, p[0]) === p[1]) keys['R:' + p[0]] = true;
    });
    return Object.keys(keys);
  }

  // A walkway is a segment above ground with nothing under it, spanning between towers.
  function isWalkway(st, c, l) { return l > 0 && has(st, c, l) && !has(st, c, l - 1); }
  function supportProblems(st) {
    var out = [];
    Object.keys(st.segments).forEach(function (k) {
      var p = k.split(',').map(Number), c = p[0], l = p[1];
      if (!isWalkway(st, c, l)) return;
      // Walk each way along the walkway run; it needs a real (supported) cell at both ends.
      [-1, 1].forEach(function (dir) {
        var x = c + dir;
        while (isWalkway(st, x, l)) x += dir;
        if (!has(st, x, l)) out.push('walkway at column ' + c + ' level ' + l + ' is only anchored on its ' + (dir < 0 ? 'right' : 'left') + ' side');
      });
    });
    // Basements must connect back to a cell above ground: dug down, or tunnelled sideways.
    var seen = {}, queue = [];
    Object.keys(st.segments).forEach(function (k) {
      var p = k.split(',').map(Number);
      if (p[1] < 0 && has(st, p[0], p[1] + 1)) { seen[k] = true; queue.push(p); }
    });
    while (queue.length) {
      var q = queue.shift();
      [[q[0] - 1, q[1]], [q[0] + 1, q[1]], [q[0], q[1] - 1]].forEach(function (n) {
        var nk = sKey(n[0], n[1]);
        if (n[1] < 0 && has(st, n[0], n[1]) && !seen[nk]) { seen[nk] = true; queue.push(n); }
      });
    }
    Object.keys(st.segments).forEach(function (k) {
      if (Number(k.split(',')[1]) < 0 && !seen[k]) out.push('basement at ' + k + ' isn\'t connected to anything above ground');
    });
    return out.filter(function (x, i, a) { return a.indexOf(x) === i; });
  }
  function structureStats(node) {
    var st = node.structure || { segments: {} };
    var keys = Object.keys(st.segments || {});
    var height = 0, depth = 0, minC = Infinity, maxC = -Infinity;
    keys.forEach(function (k) {
      var p = k.split(',').map(Number);
      if (p[1] >= 0) height = Math.max(height, p[1] + 1); else depth = Math.max(depth, -p[1]);
      minC = Math.min(minC, p[0]); maxC = Math.max(maxC, p[0]);
    });
    return { segments: keys.length, height: height, depth: depth, width: keys.length ? maxC - minC + 1 : 0 };
  }
  function structureProblems(k, node, capacityOnly) {
    var s = structureStats(node), cap = capacity(k), out = [];
    if (s.segments > cap.stability) out.push(plural(s.segments, 'segment') + ' exceeds the tile\'s stability budget of ' + cap.stability);
    if (s.height > cap.max_height) out.push('height ' + s.height + ' exceeds the tile\'s max height of ' + cap.max_height);
    if (s.width > cap.max_width) out.push('width ' + s.width + ' exceeds the tile\'s max width of ' + cap.max_width);
    if (s.depth > cap.max_depth) out.push('dig depth ' + s.depth + ' exceeds the tile\'s dig depth of ' + cap.max_depth);
    return capacityOnly ? out : out.concat(interiorProblems(node));
  }
  function profileGlyphs(node) {
    var st = node.structure, s = structureStats(node), blocks = ['·', '▂', '▄', '▆', '█'];
    if (!s.segments) return '';
    return colsWithTop(st).map(function (c) { return blocks[Math.min(colTop(st, c) + 1, 4)]; }).join('') + (s.depth ? '▾' : '');
  }
  function removeFrom(st, key) {
    var p = key.split(',').map(Number);
    Object.keys(st.segments).forEach(function (x) {
      var q = x.split(',').map(Number);
      if (q[0] !== p[0]) return;
      if ((p[1] >= 0 && q[1] >= p[1]) || (p[1] < 0 && q[1] <= p[1])) delete st.segments[x];
    });
    pruneBoundaries(st);
  }
  function featureUsed(sg) { return sg.features.reduce(function (sum, f) { var d = featureDefOf(f); return sum + (d ? Number(d.cost) || 0 : 0); }, 0); }

  // ---------- reachability, firing positions, manning ----------
  function reachability(st) {
    var adj = {}, starts = [];
    function link(a, b) { (adj[a] = adj[a] || []).push(b); (adj[b] = adj[b] || []).push(a); }
    allBoundaryKeys(st).forEach(function (bk) {
      var info = bInfo(st, bk);
      if (!info.exists || !isPassable(st, bk)) return;
      if (info.kind === 'WALL' && info.exterior) { if (info.level === 0) starts.push(has(st, info.col, info.level) ? info.sides[1] : info.sides[0]); return; }
      if (info.kind === 'ROOF') { if (roofOf(st, info.col).type === 'FLAT') link(info.sides[1], 'ROOF:' + info.col); return; }
      link(info.sides[0], info.sides[1]);
    });
    // Neighbouring flat roofs at the same height are one walkable platform.
    colsWithTop(st).forEach(function (c) {
      if (roofOf(st, c).type === 'FLAT' && st.roofs[c + 1] && st.roofs[c + 1].type === 'FLAT' && colTop(st, c) === colTop(st, c + 1)) link('ROOF:' + c, 'ROOF:' + (c + 1));
    });
    var seen = {}, queue = starts.slice();
    starts.forEach(function (k) { seen[k] = true; });
    while (queue.length) {
      var cur = queue.shift();
      (adj[cur] || []).forEach(function (n) { if (!seen[n]) { seen[n] = true; queue.push(n); } });
    }
    return {
      hasEntrance: starts.length > 0,
      unreachable: Object.keys(st.segments).filter(function (k) { return !seen[k]; }),
      reached: function (k) { return !!seen[k]; }
    };
  }
  function firingPositions(st) {
    // Where a ranged unit can shoot out from: exterior walls above ground whose projectiles
    // aren't blocked from inside, reachable flat roofs, and floors that let projectiles
    // down (murder holes). Any ranged unit can use any of them.
    var r = reachability(st), out = [];
    allBoundaryKeys(st).forEach(function (bk) {
      var info = bInfo(st, bk), b = boundary(st, bk);
      if (!info.exists) return;
      if (info.kind === 'WALL' && info.exterior && info.level >= 0) {
        var inside = info.exteriorSide === 'LEFT' ? 'RIGHT' : 'LEFT';
        var from = has(st, info.col, info.level) ? info.sides[1] : info.sides[0];
        if (b.proj.indexOf(inside) === -1 && r.reached(from)) out.push({ where: bk, label: 'end wall (' + info.exteriorSide.toLowerCase() + ') level ' + info.level });
      } else if (info.kind === 'FLOOR' && b.proj.indexOf('ABOVE') === -1 && b.move.length === 2 && r.reached(info.sides[0])) {
        out.push({ where: bk, label: 'floor opening over column ' + info.col + ' level ' + info.level });
      } else if (info.kind === 'ROOF' && roofOf(st, info.col).type === 'FLAT' && r.reached('ROOF:' + info.col)) {
        out.push({ where: bk, label: 'flat roof of column ' + info.col });
      }
    });
    return out;
  }
  function mountedEmplacements(st) {
    var out = [];
    Object.keys(st.segments).forEach(function (k) { st.segments[k].emplacements.forEach(function (id) { out.push({ id: id, kind: 'ROOM', key: k }); }); });
    Object.keys(st.roofs).forEach(function (c) { st.roofs[c].emplacements.forEach(function (id) { out.push({ id: id, kind: 'ROOF', key: c }); }); });
    return out;
  }
  function manning(node) {
    var need = {}, have = {};
    mountedEmplacements(node.structure).forEach(function (m) {
      var def = staticDefenseDef(m.id);
      if (def && def.manned_by_unit && Number(def.firing_points) > 0) need[def.manned_by_unit] = (need[def.manned_by_unit] || 0) + Number(def.firing_points);
    });
    node.garrison_units.forEach(function (u) { have[u._display_label] = (have[u._display_label] || 0) + 1; });
    var out = {};
    Object.keys(need).forEach(function (u) { out[u] = { crew: need[u], in_garrison: have[u] || 0 }; });
    return out;
  }
  function rangedInGarrison(node) { return node.garrison_units.filter(function (u) { return RANGED_UNITS.indexOf(u._display_label) !== -1; }).length; }
  function interiorProblems(node) {
    var st = node.structure, out = [];
    if (!Object.keys(st.segments).length) return out;
    var r = reachability(st);
    out = out.concat(supportProblems(st));
    if (!r.hasEntrance) out.push('no passable end wall at ground level, so there is no way in');
    else if (r.unreachable.length) out.push(plural(r.unreachable.length, 'segment') + ' unreachable from an entrance');
    var budget = featurePointsPerSegment();
    Object.keys(st.segments).forEach(function (k) {
      var used = featureUsed(st.segments[k]);
      if (used > budget) out.push('segment ' + k + ' uses ' + used + ' feature points of ' + budget);
    });
    Object.keys(st.roofs).forEach(function (c) {
      var roof = st.roofs[c];
      if (!roof.emplacements.length) return;
      if (roof.type !== 'FLAT') out.push('roof of column ' + c + ' holds emplacements but isn\'t flat');
      else if (!r.reached('ROOF:' + c)) out.push('roof of column ' + c + ' has emplacements but no reachable hatch or stairs, so nobody can crew them');
    });
    mountedEmplacements(st).forEach(function (m) {
      var def = staticDefenseDef(m.id);
      if (!def) { out.push('unknown emplacement ' + m.id); return; }
      if (def.mount !== m.kind) out.push(def.label + ' goes ' + (def.mount === 'ROOF' ? 'on a flat roof' : 'in a room') + ', not ' + (m.kind === 'ROOF' ? 'on a roof' : 'in a room'));
    });
    var man = manning(node);
    Object.keys(man).forEach(function (u) {
      if (man[u].in_garrison < man[u].crew) out.push(u + ': emplacements need ' + man[u].crew + ' crew but only ' + man[u].in_garrison + ' are in the garrison');
    });
    return out;
  }

  // ---------- approaches (Decision 27: each ingress enters at the left or right end) ----------
  function linkedNodeIds(node) {
    var out = [];
    state.links.forEach(function (l) { if (l.a === node.id) out.push(l.b); else if (l.b === node.id) out.push(l.a); });
    return out.filter(function (x, i) { return out.indexOf(x) === i; });
  }
  function approachSide(node, otherId) {
    if (node.approaches && node.approaches[otherId]) return node.approaches[otherId];
    var mine = findNodeKeyById(node.id), theirs = findNodeKeyById(otherId);
    if (!mine || !theirs) return 'LEFT';
    var a = parseKey(mine), b = parseKey(theirs);
    if (b[1] !== a[1]) return b[1] < a[1] ? 'LEFT' : 'RIGHT';
    return b[0] < a[0] ? 'LEFT' : 'RIGHT';
  }

  // ---------- structure editor ----------
  function adjacentDrawbridges(k, node) {
    var p = parseKey(k), out = [];
    for (var dr = -1; dr <= 1; dr++) for (var dc = -1; dc <= 1; dc++) {
      if (!dr && !dc) continue;
      var nk = key(p[0] + dr, p[1] + dc), t = state.tiles[nk];
      if (t && t.bridge && t.bridge.drawbridge_node_id === node.id) out.push(nk);
    }
    return out;
  }
  function empOptions(mount) {
    return '<option value="">+ ' + (mount === 'ROOF' ? 'roof' : 'room') + ' emplacement</option>' + staticDefenseLib.filter(function (d) { return d.mount === mount; }).map(function (d) {
      return '<option value="' + esc(d.id) + '">' + esc(d.label + (Number(d.firing_points) ? ' (' + d.manned_by_unit + ' ×' + d.firing_points + ')' : '')) + '</option>';
    }).join('');
  }
  function empChips(ids, attr) {
    if (!ids.length) return '<span class="sd-legend">none</span>';
    return ids.map(function (id, i) { var d = staticDefenseDef(id); return '<span class="mini-chip">' + esc(d ? d.label : id) + '<button type="button" class="row-remove" ' + attr + '="' + i + '" aria-label="Remove">&times;</button></span>'; }).join('');
  }
  function boundaryHead(st, bk) {
    var info = bInfo(st, bk);
    if (info.kind === 'WALL') return info.exterior ? (info.exteriorSide === 'LEFT' ? 'Left' : 'Right') + ' end wall · level ' + info.level : 'Wall between columns ' + (info.col - 1) + ' and ' + info.col + ' · level ' + info.level;
    if (info.kind === 'FLOOR') return 'Floor between level ' + info.level + ' and ' + (info.level + 1) + ' · column ' + info.col;
    return 'Roof access · column ' + info.col;
  }

  function renderStructureEditor() {
    var host = document.getElementById('structureEditor');
    var k = state.selectedCell;
    var node = k ? nodeAt(k) : null;
    if (!node || node.node_type === 'WAYPOINT') { host.innerHTML = ''; return; }
    var st = node.structure;
    var cap = capacity(k), s = structureStats(node);
    var maxC = -1;
    Object.keys(st.segments).forEach(function (sk) { maxC = Math.max(maxC, Number(sk.split(',')[0])); });
    var W = Math.max(cap.max_width, maxC + 1, 1);
    var H = Math.max(cap.max_height, s.height, 1), D = Math.max(cap.max_depth, s.depth);
    var sel = state.structureSel;
    if (sel && sel.type === 'cell' && !st.segments[sel.key]) sel = state.structureSel = null;
    if (sel && sel.type === 'bound' && !bInfo(st, sel.key).exists) sel = state.structureSel = null;
    if (sel && sel.type === 'roof' && colTop(st, sel.col) < 0) sel = state.structureSel = null;
    var reach = reachability(st), unreachable = {};
    reach.unreachable.forEach(function (x) { unreachable[x] = true; });
    var levels = [];
    for (var lv = H - 1; lv >= -D; lv--) levels.push(lv);
    function rowOf(l) { return 3 + 2 * levels.indexOf(l); }
    function outside(c, l) { return c >= cap.max_width || (l >= 0 ? l >= cap.max_height : -l > cap.max_depth); }

    var items = '';
    for (var rc = 0; rc < W; rc++) {
      if (colTop(st, rc) < 0) continue;
      var roof = roofOf(st, rc), rt = ROOF_TYPES.find(function (x) { return x.id === roof.type; });
      var rTop = colTop(st, rc), onSlot = rTop < H - 1; // sit in the empty slot just above the top room
      items += '<button type="button" class="sx-roof' + (onSlot ? ' on-slot' : '') + '" style="grid-column:' + (2 * rc + 2) + ';grid-row:' + (onSlot ? rowOf(rTop + 1) : 1) + ';" data-roof="' + rc + '"' + (sel && sel.type === 'roof' && sel.col === rc ? ' data-selected="true"' : '') +
        ' title="Roof: ' + esc(rt.label) + '">' + rt.glyph + roof.emplacements.map(function (id) { var d = staticDefenseDef(id); return ' ' + esc(d ? d.abbr : id); }).join('') + '</button>';
    }
    levels.forEach(function (l) {
      for (var c = 0; c < W; c++) {
        var ck = sKey(c, l), sg = st.segments[ck];
        var supported = l === 0 || has(st, c - 1, l) || has(st, c + 1, l) || (l > 0 ? has(st, c, l - 1) : has(st, c, l + 1));
        var walk = sg && isWalkway(st, c, l);
        var inner = '';
        if (sg) {
          inner += '<span class="lbl">' + esc(sg.label || (walk ? 'walkway' : '·')) + '</span>';
          if (sg.features.length) inner += '<span>' + featureUsed(sg) + '/' + featurePointsPerSegment() + ' pts</span>';
          if (sg.emplacements.length) inner += '<span class="marks">' + sg.emplacements.map(function (id) { var d = staticDefenseDef(id); return esc(d ? d.abbr : id); }).join(' ') + '</span>';
        } else if (!outside(c, l) && supported) inner = '<span class="hint-plus">+</span>';
        items += '<button type="button" class="sx-cell" style="grid-column:' + (2 * c + 2) + ';grid-row:' + rowOf(l) + ';" data-seg="' + ck + '" data-filled="' + !!sg + '"' +
          (sg ? ' data-state="' + (unreachable[ck] ? 'unreachable' : 'ok') + '"' : '') + (outside(c, l) ? ' data-over="true"' : '') + (!sg && !supported ? ' data-unsupported="true"' : '') +
          (l < 0 ? ' data-below="true"' : '') + (walk ? ' data-walkway="true"' : '') + (sel && sel.type === 'cell' && sel.key === ck ? ' data-selected="true"' : '') +
          ' aria-label="Column ' + c + ', level ' + l + (sg ? ', built' : ', empty') + '">' + inner + '</button>';
      }
    });
    items += '<div class="sx-groundline" style="grid-column:1 / span ' + (2 * W + 1) + ';grid-row:' + (rowOf(0) + 1) + ';"></div>';
    function boundBtn(bk, col, row, orient) {
      var info = bInfo(st, bk);
      if (!info.exists) return '';
      var b = boundary(st, bk), def = presetDef(info.kind, b.preset);
      var glyph = b.custom ? '✱' : (def ? def.glyph : '');
      return '<button type="button" class="sx-bound ' + orient + '" style="grid-column:' + col + ';grid-row:' + row + ';" data-bound="' + bk + '" data-preset="' + esc(b.preset) + '"' +
        (info.exterior ? ' data-ext="true"' : '') + (isPassable(st, bk) ? ' data-pass="true"' : '') + (b.fortification !== 'NONE' ? ' data-fort="true"' : '') +
        (sel && sel.type === 'bound' && sel.key === bk ? ' data-selected="true"' : '') + ' title="' + esc(boundaryHead(st, bk) + ': ' + (b.custom ? 'custom' : def ? def.label : b.preset)) + '" aria-label="' + esc(boundaryHead(st, bk)) + '">' + glyph + '</button>';
    }
    levels.forEach(function (l) { for (var b = 0; b <= W; b++) items += boundBtn('W:' + sKey(b, l), 2 * b + 1, rowOf(l), 'v'); });
    for (var c3 = 0; c3 < W; c3++) {
      for (var i3 = 0; i3 + 1 < levels.length; i3++) items += boundBtn('F:' + sKey(c3, levels[i3 + 1]), 2 * c3 + 2, rowOf(levels[i3]) + 1, 'h');
      var top = colTop(st, c3);
      if (top >= 0) items += boundBtn('R:' + c3, 2 * c3 + 2, rowOf(top) - 1, 'h');
    }
    // Cells grow to fill the space left of the side column (bigger still when expanded).
    var sideW = 400, avail = (host.clientWidth || host.parentNode.clientWidth || 0) - sideW - 40;
    var cellW = Math.max(72, Math.min(state.expanded ? 240 : 150, Math.floor((avail - 12 * (W + 1)) / W)));
    var cellH = Math.max(48, Math.min(state.expanded ? 110 : 64, Math.round(cellW * 0.45)));
    var rows = '26px 12px ' + levels.map(function (l, i) { return cellH + 'px' + (i < levels.length - 1 ? ' 12px' : ''); }).join(' ') + ' 6px';
    var links = linkedNodeIds(node);
    var leftFrom = links.filter(function (o) { return approachSide(node, o) === 'LEFT'; }), rightFrom = links.filter(function (o) { return approachSide(node, o) === 'RIGHT'; });
    var endsHtml = '<div class="ends"><span>' + (leftFrom.length ? '→ from ' + esc(leftFrom.join(', ')) : '<span title="No linked route enters this end. Fine if intended; set sides under Approaches.">left end: no route enters</span>') + '</span><span>' + (rightFrom.length ? 'from ' + esc(rightFrom.join(', ')) + ' ←' : '<span title="No linked route enters this end. Fine if intended; set sides under Approaches.">right end: no route enters</span>') + '</span></div>';
    var gridHtml = endsHtml + '<div class="sx-grid" style="--cw:' + cellW + 'px;--ch:' + cellH + 'px;grid-template-columns:12px repeat(' + W + ', ' + cellW + 'px 12px);grid-template-rows:' + rows + ';">' + items + '</div>';

    // ----- selection panel -----
    var detail = '<div class="sd-card">';
    if (sel && sel.type === 'cell') {
      var sgS = st.segments[sel.key], p = sel.key.split(',').map(Number);
      var used = featureUsed(sgS), budget = featurePointsPerSegment();
      var walkS = isWalkway(st, p[0], p[1]);
      detail += '<div class="sd-head">Column ' + p[0] + ' · ' + (p[1] < 0 ? 'basement ' + (-p[1]) : p[1] === 0 ? 'ground floor' : 'level ' + p[1]) + (walkS ? ' · walkway span' : '') + '</div>' +
        '<div class="insp-row"><label for="sd_label">' + (walkS ? 'Name' : 'Room name') + '</label><input type="text" id="sd_label" value="' + esc(sgS.label) + '" placeholder="' + (walkS ? 'e.g. East bridge' : 'e.g. Barracks') + '" /></div>' +
        (walkS ? '<p class="sd-legend">A walkway is a narrow span between towers with nothing underneath: no furniture or emplacements. It needs a tower at both ends. Build the column\'s ground floor to turn it into a normal room.</p>' : '') +
        (walkS ? '' : 
        '<div class="insp-row"><label for="sd_prefab">Room prefab</label><div class="patrol-add"><select id="sd_prefab">' +
        roomPrefabLib.map(function (rp) { return '<option value="' + esc(rp.id) + '">' + esc(rp.label + ' (' + prefabCost(rp) + ' pts)') + '</option>'; }).join('') +
        '</select><button type="button" class="btn small" id="sd_prefabApply">Apply</button></div></div>' +
        '<div class="insp-row"><label>Room features · <span class="' + (used > budget ? 'over-text' : '') + '">' + used + '/' + budget + ' points</span></label><div class="chip-list">' +
        (sgS.features.length ? sgS.features.map(function (f, i) { var d = featureDefOf(f); return '<span class="mini-chip">' + esc((d ? d.label : f) + ' · ' + (d ? d.cost : '?')) + '<button type="button" class="row-remove" data-rmfeat="' + i + '" aria-label="Remove feature">&times;</button></span>'; }).join('') : '<span class="sd-legend">none</span>') +
        '</div><div class="patrol-add"><select id="sd_featAdd" aria-label="Feature">' + featureLib.features.map(function (f) { return '<option value="' + esc(f.id) + '">' + esc(f.label + ' · ' + f.cost + ' pt · ' + f.effect) + '</option>'; }).join('') + '</select>' +
        '<button type="button" class="btn small" id="sd_featAddBtn">Add</button></div></div>' +
        '<div class="insp-row"><label>Emplacements in this room</label><div class="chip-list">' + empChips(sgS.emplacements, 'data-rmemp') + '</div>' +
        '<select id="sd_empAdd" aria-label="Room emplacement">' + empOptions('ROOM') + '</select></div>') +
        '<p class="sd-legend">Walls, floors and hatches are the gaps around this cell. Click one to set what it blocks.</p>' +
        '<div><button type="button" class="btn small danger" id="sd_removeFrom">' + (p[1] < 0 ? 'Remove this and everything below' : 'Remove this and everything above') + '</button></div>';
    } else if (sel && sel.type === 'bound') {
      var bk = sel.key, info = bInfo(st, bk), b = boundary(st, bk), kind = info.kind, sides = bSides(bk);
      var presets = boundaryPresets()[kind];
      var def = presetDef(kind, b.preset);
      detail += '<div class="sd-head">' + esc(boundaryHead(st, bk)) + '</div>' +
        '<div class="insp-row"><label for="sd_preset">Type</label><select id="sd_preset">' + presets.map(function (pr) { return '<option value="' + pr.id + '"' + (pr.id === b.preset ? ' selected' : '') + '>' + esc(pr.label) + '</option>'; }).join('') +
        (b.custom ? '<option value="CUSTOM" selected>Custom</option>' : '') + '</select></div>';
      if (kind === 'WALL') {
        detail += '<div class="insp-row"><label for="sd_mat">Material</label><select id="sd_mat">' + REINFORCEMENTS.map(function (r) { return '<option value="' + r.id + '"' + (r.id === b.material ? ' selected' : '') + '>' + esc(r.label + ' · HP ×' + r.hp_mult) + '</option>'; }).join('') + '</select></div>';
        if (!info.exterior) detail += '<div class="insp-row"><label for="sd_outside">Outside is the…</label><select id="sd_outside"><option value="LEFT"' + (b.outside === 'LEFT' ? ' selected' : '') + '>left side</option><option value="RIGHT"' + (b.outside === 'RIGHT' ? ' selected' : '') + '>right side</option></select></div>';
      }
      detail += '<div class="insp-row"><label>Blocks… (from which side)</label><div class="prop-table">';
      propsList().forEach(function (pr) {
        detail += '<span class="prop-name">' + pr[1] + '</span>' + sides.map(function (sd) {
          return '<label class="prop-check"><input type="checkbox" data-prop="' + pr[0] + '" data-side="' + sd + '"' + (b[pr[0]].indexOf(sd) !== -1 ? ' checked' : '') + ' /> from ' + sideLabel(sd) + (sd === b.outside ? ' <em>(outside)</em>' : '') + '</label>';
        }).join('');
      });
      detail += '</div><span class="sd-legend">Ticking a box makes this boundary Custom. Defenders can always pass a door-type boundary (they open it); attackers must break it.</span></div>';
      if ((def && (def.door || def.opening)) || b.custom) {
        detail += '<div class="insp-row"><label for="sd_fort">Fortification</label><select id="sd_fort">' + FORTIFICATIONS.map(function (f) { return '<option value="' + f.id + '"' + (f.id === b.fortification ? ' selected' : '') + '>' + esc(f.label + (f.delay ? ' · holds attackers ' + f.delay + ' tick' + (f.delay === 1 ? '' : 's') + ' (stub)' : '')) + '</option>'; }).join('') + '</select></div>';
      }
    } else if (sel && sel.type === 'roof') {
      var roof2 = roofOf(st, sel.col);
      detail += '<div class="sd-head">Roof of column ' + sel.col + '</div>' +
        '<div class="insp-row"><label for="sd_roofType">Roof type</label><select id="sd_roofType">' + ROOF_TYPES.map(function (r) { return '<option value="' + r.id + '"' + (r.id === roof2.type ? ' selected' : '') + '>' + esc(r.label) + '</option>'; }).join('') + '</select></div>' +
        '<div class="insp-row"><label>Roof emplacements</label><div class="chip-list">' + empChips(roof2.emplacements, 'data-rmroofemp') + '</div>' +
        (roof2.type === 'FLAT' ? '<select id="sd_roofEmp" aria-label="Roof emplacement">' + empOptions('ROOF') + '</select>' : '<span class="sd-legend">Only a flat roof takes emplacements.</span>') + '</div>' +
        '<p class="sd-legend">Set the hatch or stairs up to this roof on the gap just above the column\'s top cell.</p>';
    }
    detail = sel ? detail + '</div>' : ''; // nothing selected: no details card, the grid keeps the space

    // ----- summary -----
    function stat(label, usedN, max) { return '<span' + (usedN > max ? ' class="over"' : '') + '>' + label + ' ' + usedN + '/' + max + '</span>'; }
    var t = tileAt(k), terr = terrainDef(tileTerrainId(k)) || {};
    var positions = firingPositions(st), ranged = rangedInGarrison(node);
    var man = manning(node);
    var manLines = Object.keys(man).map(function (u) {
      var m = man[u];
      return '<div class="sd-legend">' + esc(u) + ': emplacements need ' + m.crew + ' crew, ' + m.in_garrison + ' in garrison</div>';
    }).join('');
    var problems = interiorProblems(node).concat(structureProblems(k, node, true));
    var bridges = adjacentDrawbridges(k, node);
    var summary = '<div class="sd-card"><div class="sd-head">Structure</div>' +
      '<div class="insp-row"><label>Approaches (which end each route enters)</label>' +
      (links.length ? links.map(function (o) {
        var side = approachSide(node, o);
        return '<div class="appr-row"><span class="mono">' + esc(o) + '</span><select data-appr="' + esc(o) + '" aria-label="Approach side for ' + esc(o) + '"><option value="LEFT"' + (side === 'LEFT' ? ' selected' : '') + '>enters at the left</option><option value="RIGHT"' + (side === 'RIGHT' ? ' selected' : '') + '>enters at the right</option></select></div>';
      }).join('') : '<span class="sd-legend">No links to this node yet.</span>') + '</div>' +
      '<div class="insp-row"><label for="sd_arch">Archetype (art only)</label><select id="sd_arch">' + ARCHETYPES.map(function (x) { return '<option value="' + x.id + '"' + (x.id === st.archetype ? ' selected' : '') + '>' + esc(x.label) + '</option>'; }).join('') + '</select></div>' +
      '<div class="insp-row"><label>Tile capacity (' + esc(terr.label || 'terrain') + ' defaults; blank = default)</label><div class="cap-grid">' +
      [['sd_capStab', 'stability', 'Stability', 'default_stability'], ['sd_capH', 'max_height', 'Height', 'default_max_height'], ['sd_capW', 'max_width', 'Width', 'default_max_width'], ['sd_capD', 'max_depth', 'Dig depth', 'default_max_depth']].map(function (f) {
        return '<label class="cap-cell">' + f[2] + '<input type="number" min="0" id="' + f[0] + '" data-capkey="' + f[1] + '" value="' + (t[f[1]] == null ? '' : esc(t[f[1]])) + '" placeholder="' + esc(terr[f[3]] || 0) + '" /></label>';
      }).join('') + '</div></div>' +
      '<div class="budget">' + stat('segments', s.segments, cap.stability) + stat('height', s.height, cap.max_height) + stat('width', s.width, cap.max_width) + stat('dig', s.depth, cap.max_depth) + '</div>' +
      '<div class="insp-row"><label>Defenders</label><div class="sd-legend">Ranged firing positions: <b>' + positions.length + '</b> (any ranged unit) · ranged units in garrison: <b>' + ranged + '</b>' +
      (positions.length ? ' → ' + Math.min(positions.length, ranged) + ' firing' + (ranged > positions.length ? ', ' + (ranged - positions.length) + ' without a position' : '') : '') + '</div>' + manLines + '</div>' +
      (problems.length ? '<div class="warn-inline" style="margin:0;">' + problems.map(function (x) { return '&bull; ' + esc(x); }).join('<br>') + '</div>' : '<div class="sd-legend" style="color:var(--good);">Every segment is reachable, features fit, and every emplacement is crewed.</div>') +
      (bridges.length ? '<div class="insp-row"><label>Drawbridges</label>' + bridges.map(function (bk2) {
        var br = state.tiles[bk2].bridge;
        return '<div class="sd-legend" style="display:flex;gap:6px;align-items:center;">Bridge at ' + esc(cellLabel(bk2)) + ' · ' + (br.raised ? 'raised' : 'lowered') +
          ' <button type="button" class="btn small" data-drawbridge="' + bk2 + '">' + (br.raised ? 'Lower' : 'Raise') + '</button></div>';
      }).join('') + '</div>' : '') +
      '<div class="insp-row"><label>Prefabs</label><div class="patrol-add"><input type="text" id="sd_prefabName" placeholder="Name to save as" aria-label="Prefab name" />' +
      '<button type="button" class="btn small" id="sd_prefabSave">Save as prefab</button></div>' +
      (structurePrefabLib.length ? '<div class="patrol-add"><select id="sd_prefabLoad" aria-label="Structure prefab">' + structurePrefabLib.map(function (sp) {
        var r = sp.requirements; return '<option value="' + esc(sp.id) + '">' + esc(sp.name + ' (' + r.segments + ' seg · ' + r.height + 'h · ' + r.width + 'w · dig ' + r.depth + ')') + '</option>';
      }).join('') + '</select><button type="button" class="btn small" id="sd_prefabLoadBtn">Load</button></div>' : '<span class="sd-legend">No saved structure prefabs yet.</span>') + '</div>' +
      '<div><button type="button" class="btn small ghost" id="sd_clear">Clear structure</button></div></div>';

    host.innerHTML = '<div class="sd-wrap"><div class="sd-grid-host"><span class="sd-axis">side view · node ' + esc(node.id) + ' · ' + esc(cellLabel(k)) + '</span>' + gridHtml + '</div>' +
      '<div class="sd-side">' + (detail ? '<div class="sd-detail">' + detail + '</div>' : '') + summary + '</div></div>';

    function commit() { pruneBoundaries(st); safeSave(); renderStructureEditor(); renderGrid(); renderInspector(); }
    host.querySelectorAll('.sx-cell').forEach(function (btn) {
      btn.addEventListener('click', function () {
        var p = btn.getAttribute('data-seg').split(',').map(Number), c = p[0], l = p[1];
        if (has(st, c, l)) { state.structureSel = { type: 'cell', key: sKey(c, l) }; renderStructureEditor(); return; }
        if (outside(c, l)) { setNotice('Outside this tile\'s capacity (height ' + cap.max_height + ', width ' + cap.max_width + ', dig depth ' + cap.max_depth + '). Raise it in Tile capacity.'); return; }
        var list = [], mode = 'column', note = '';
        var sideNb = has(st, c - 1, l) ? c - 1 : has(st, c + 1, l) ? c + 1 : null;
        if (l > 0 && !has(st, c, l - 1) && colTop(st, c) < 0 && sideNb !== null) {
          mode = 'walkway'; list = [l];
          note = 'Built a walkway span. Build this column\'s ground floor to make it a full room instead.';
        } else if (l >= 0) {
          // Fill down to the ground; if a walkway already spans this column higher up, fill up to meet it.
          var upTo = l, above = colTop(st, c);
          if (above > l) { upTo = l; for (var a2 = l + 1; a2 <= above; a2++) { if (has(st, c, a2)) { upTo = a2 - 1; break; } } }
          for (var k2 = 0; k2 <= upTo; k2++) if (!has(st, c, k2)) list.push(k2);
        } else if (has(st, c, l + 1)) {
          list = [l];
        } else if (has(st, c, 0)) {
          for (var k3 = colBottom(st, c) - 1; k3 >= l; k3--) if (!has(st, c, k3)) list.push(k3);
        } else if (sideNb !== null) {
          mode = 'tunnel'; list = [l];
        } else {
          setNotice('Dig down from a ground-floor room, or extend sideways from a neighbouring basement.');
          return;
        }
        if (s.segments + list.length > cap.stability) { setNotice('Not enough stability: ' + s.segments + ' + ' + list.length + ' segments exceeds the budget of ' + cap.stability + '.'); return; }
        var wasEmpty = !s.segments, newColumn = colTop(st, c) < 0 && !has(st, c, 0);
        list.forEach(function (lv3) { st.segments[sKey(c, lv3)] = { label: '', features: [], emplacements: [] }; });
        list.forEach(function (lv3) {
          // Stairs between vertically stacked cells where one of them is new.
          [lv3 - 1, lv3].forEach(function (lower) {
            if (has(st, c, lower) && has(st, c, lower + 1)) { var fb = ensureB(st, 'F:' + sKey(c, lower)); if (fb.preset === 'SOLID_FLOOR') fb.preset = 'STAIRS'; }
          });
        });
        if (mode === 'walkway') {
          [c, c + 1].forEach(function (wc) { if (has(st, wc - 1, l) && has(st, wc, l)) ensureB(st, 'W:' + sKey(wc, l)).preset = 'DOORWAY'; });
        } else if (mode === 'tunnel') {
          ensureB(st, 'W:' + sKey(sideNb < c ? c : c + 1, l)).preset = 'DOOR';
        } else if (newColumn && l >= 0) {
          if (wasEmpty) ensureB(st, 'W:' + sKey(c, 0)).preset = 'DOOR';
          else if (has(st, c - 1, 0)) ensureB(st, 'W:' + sKey(c, 0)).preset = 'DOOR';
          else if (has(st, c + 1, 0)) ensureB(st, 'W:' + sKey(c + 1, 0)).preset = 'DOOR';
        }
        setNotice(note);
        if (l >= 0) roofOf(st, c);
        state.structureSel = { type: 'cell', key: sKey(c, l) };
        commit();
      });
    });
    host.querySelectorAll('.sx-roof').forEach(function (b2) { b2.addEventListener('click', function () { state.structureSel = { type: 'roof', col: Number(b2.getAttribute('data-roof')) }; renderStructureEditor(); }); });
    host.querySelectorAll('.sx-bound').forEach(function (b3) { b3.addEventListener('click', function () { state.structureSel = { type: 'bound', key: b3.getAttribute('data-bound') }; renderStructureEditor(); }); });
    host.querySelectorAll('[data-appr]').forEach(function (sl) {
      sl.addEventListener('change', function () { node.approaches = node.approaches || {}; node.approaches[sl.getAttribute('data-appr')] = this.value; commit(); });
    });
    document.getElementById('sd_arch').addEventListener('change', function () { st.archetype = this.value; safeSave(); renderGrid(); renderInspector(); });
    host.querySelectorAll('[data-capkey]').forEach(function (inp) {
      inp.addEventListener('change', function () {
        var tt = ensureTile(k), prop = inp.getAttribute('data-capkey');
        if (this.value === '') delete tt[prop]; else tt[prop] = Math.max(0, Number(this.value) || 0);
        pruneTile(k); commit();
      });
    });
    host.querySelectorAll('[data-drawbridge]').forEach(function (b4) {
      b4.addEventListener('click', function () { var br = state.tiles[b4.getAttribute('data-drawbridge')].bridge; br.raised = !br.raised; commit(); renderLinks(); });
    });
    document.getElementById('sd_prefabSave').addEventListener('click', function () {
      var name = document.getElementById('sd_prefabName').value.trim();
      if (!name) { setNotice('Give the prefab a name first.'); return; }
      if (!s.segments) { setNotice('Nothing built to save yet.'); return; }
      var id = name.toUpperCase().replace(/[^A-Z0-9]+/g, '_') + '_' + Date.now().toString(36);
      structurePrefabLib.push({ id: id, name: name, structure: clone(st), requirements: { segments: s.segments, height: s.height, width: s.width, depth: s.depth } });
      saveStructurePrefabs();
      setNotice('Saved "' + name + '" to the Structures library.');
      renderStructureEditor();
    });
    var loadBtn = document.getElementById('sd_prefabLoadBtn');
    if (loadBtn) loadBtn.addEventListener('click', function () {
      var sp = structurePrefabLib.find(function (x) { return x.id === document.getElementById('sd_prefabLoad').value; });
      if (!sp) return;
      var r = sp.requirements, bad = [];
      if (r.segments > cap.stability) bad.push('stability ' + r.segments + ' > ' + cap.stability);
      if (r.height > cap.max_height) bad.push('height ' + r.height + ' > ' + cap.max_height);
      if (r.width > cap.max_width) bad.push('width ' + r.width + ' > ' + cap.max_width);
      if (r.depth > cap.max_depth) bad.push('dig depth ' + r.depth + ' > ' + cap.max_depth);
      if (bad.length) { setNotice('"' + sp.name + '" doesn\'t fit this tile: ' + bad.join(', ') + '.'); return; }
      node.structure = normalizeStructure(clone(sp.structure));
      state.structureSel = null;
      setNotice('Loaded "' + sp.name + '".');
      commit();
    });
    document.getElementById('sd_clear').addEventListener('click', function () { st.segments = {}; st.boundaries = {}; st.roofs = {}; state.structureSel = null; commit(); });

    if (!sel) return;
    function onChange(id, fn) { var el = document.getElementById(id); if (el) el.addEventListener('change', fn); }
    function onRemove(attr, arr) { host.querySelectorAll('[' + attr + ']').forEach(function (b5) { b5.addEventListener('click', function () { arr.splice(Number(b5.getAttribute(attr)), 1); commit(); }); }); }
    if (sel.type === 'cell') {
      var cS = st.segments[sel.key];
      onChange('sd_label', function () { cS.label = this.value; commit(); });
      if (document.getElementById('sd_prefabApply')) document.getElementById('sd_prefabApply').addEventListener('click', function () {
        var rp = roomPrefabDef(document.getElementById('sd_prefab').value);
        if (!rp) return;
        if (prefabCost(rp) > featurePointsPerSegment()) { setNotice(rp.label + ' needs ' + prefabCost(rp) + ' feature points; a segment has ' + featurePointsPerSegment() + '.'); return; }
        cS.label = rp.label; cS.features = rp.features.slice(); setNotice(''); commit();
      });
      if (document.getElementById('sd_featAddBtn')) document.getElementById('sd_featAddBtn').addEventListener('click', function () {
        var fid = document.getElementById('sd_featAdd').value, d = featureDefOf(fid);
        if (!d) return;
        var left = featurePointsPerSegment() - featureUsed(cS);
        if ((Number(d.cost) || 0) > left) { setNotice(left < 0 ? 'This room is already over its feature points. Remove a feature first.' : 'Not enough feature points: ' + d.label + ' costs ' + d.cost + ', ' + left + ' left.'); return; }
        cS.features.push(fid); setNotice(''); commit();
      });
      onRemove('data-rmfeat', cS.features);
      onChange('sd_empAdd', function () { if (this.value) { cS.emplacements.push(this.value); commit(); } });
      onRemove('data-rmemp', cS.emplacements);
      document.getElementById('sd_removeFrom').addEventListener('click', function () { removeFrom(st, sel.key); state.structureSel = null; commit(); });
    } else if (sel.type === 'bound') {
      var bkS = sel.key;
      onChange('sd_preset', function () {
        if (this.value === 'CUSTOM') return;
        var sb = ensureB(st, bkS); sb.preset = this.value; propsList().forEach(function (pr) { delete sb[pr[0]]; });
        commit();
      });
      onChange('sd_mat', function () { ensureB(st, bkS).material = this.value; commit(); });
      onChange('sd_outside', function () { ensureB(st, bkS).outside = this.value; commit(); });
      onChange('sd_fort', function () { ensureB(st, bkS).fortification = this.value; commit(); });
      host.querySelectorAll('[data-prop]').forEach(function (cb) {
        cb.addEventListener('change', function () {
          var resolved = boundary(st, bkS), sb = ensureB(st, bkS);
          if (sb.preset !== 'CUSTOM') { propsList().forEach(function (pr) { sb[pr[0]] = resolved[pr[0]].slice(); }); sb.preset = 'CUSTOM'; }
          var prop = cb.getAttribute('data-prop'), side = cb.getAttribute('data-side');
          sb[prop] = (sb[prop] || []).filter(function (x) { return x !== side; });
          if (cb.checked) sb[prop].push(side);
          commit();
        });
      });
    } else if (sel.type === 'roof') {
      var roofS = roofOf(st, sel.col);
      onChange('sd_roofType', function () {
        roofS.type = this.value;
        if (roofS.type !== 'FLAT' && roofS.emplacements.length) { roofS.emplacements = []; setNotice('Removed roof emplacements: only a flat roof can take them.'); }
        commit();
      });
      onChange('sd_roofEmp', function () { if (this.value) { roofS.emplacements.push(this.value); commit(); } });
      onRemove('data-rmroofemp', roofS.emplacements);
    }
  }

  // ---------- Static defenses view (emplacements) ----------
  var selectedDefenseId = null;
  var defenseNotice = '';
  var MOUNTS = [
    { id: 'ROOM', label: 'Inside a room' },
    { id: 'ROOF', label: 'On a flat roof' }
  ];
  function defenseUsage(id) {
    var n = 0;
    Object.keys(state.cells).forEach(function (k) {
      mountedEmplacements(state.cells[k].structure).forEach(function (m) { if (m.id === id) n++; });
    });
    return n;
  }
  function renderDefensesView() {
    if (!selectedDefenseId || !staticDefenseDef(selectedDefenseId)) selectedDefenseId = staticDefenseLib.length ? staticDefenseLib[0].id : null;
    var list = document.getElementById('defenseLibList');
    list.innerHTML = staticDefenseLib.length ? '' : '<p class="empty-note" style="margin:0;">none yet</p>';
    staticDefenseLib.forEach(function (d) {
      var el = document.createElement('div');
      el.className = 'terrain-item lib-row';
      el.tabIndex = 0;
      el.setAttribute('data-selected', d.id === selectedDefenseId ? 'true' : 'false');
      el.innerHTML = '<span style="display:flex;align-items:center;gap:6px;" class="name"><span class="glyph-dot" style="background:var(--accent);color:var(--accent-ink);font-size:8px;width:26px;">' + esc(d.abbr) + '</span>' + esc(d.label) + '</span>' +
        '<span class="mono" style="font-size:10px;color:var(--ink-dim);">' + esc(d.mount.toLowerCase()) + ' · ' + esc(d.manned_by_unit) + ' ×' + esc(d.firing_points) + '</span>';
      var pick = function () { selectedDefenseId = d.id; defenseNotice = ''; renderDefensesView(); };
      el.addEventListener('click', pick);
      el.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') pick(); });
      list.appendChild(el);
    });
    var detail = document.getElementById('defenseDetail');
    var d = staticDefenseDef(selectedDefenseId);
    if (!d) { detail.innerHTML = '<p class="empty-note">Add an emplacement to start.</p>'; return; }
    var used = defenseUsage(d.id);
    detail.innerHTML =
      '<div class="detail-title"><span class="glyph-dot" style="width:34px;height:26px;font-size:10px;background:var(--accent);color:var(--accent-ink);">' + esc(d.abbr) + '</span><h2>' + esc(d.label) + '</h2><span class="mono" style="color:var(--ink-dim);">' + esc(d.id) + '</span></div>' +
      '<p class="usage">On the current map: placed ' + plural(used, 'time') + '.</p>' +
      '<div class="form-grid">' +
      '<div class="insp-row"><label for="sdv_label">Label</label><input type="text" id="sdv_label" value="' + esc(d.label) + '" /></div>' +
      '<div class="insp-row"><label for="sdv_abbr">Abbreviation (3 letters)</label><input type="text" id="sdv_abbr" maxlength="3" value="' + esc(d.abbr) + '" /></div>' +
      '<div class="insp-row"><label for="sdv_mount">Placed</label><select id="sdv_mount">' + MOUNTS.map(function (m) { return '<option value="' + m.id + '"' + (d.mount === m.id ? ' selected' : '') + '>' + esc(m.label) + '</option>'; }).join('') + '</select></div>' +
      '<div class="insp-row"><label for="sdv_unit">Crewed by</label><select id="sdv_unit">' + UNIT_LIBRARY.map(function (u) { return '<option' + (u === d.manned_by_unit ? ' selected' : '') + '>' + esc(u) + '</option>'; }).join('') + '</select></div>' +
      '<div class="insp-row"><label for="sdv_fp">Crew needed</label><input type="number" min="0" id="sdv_fp" value="' + esc(d.firing_points) + '" /></div>' +
      '<div class="insp-row"><label for="sdv_dmg">Damage (stub)</label><input type="number" min="0" id="sdv_dmg" value="' + esc(d.stub_damage) + '" /></div>' +
      '<div class="insp-row"><label for="sdv_range">Range in cells (stub)</label><input type="number" min="0" id="sdv_range" value="' + esc(d.stub_range) + '" /></div>' +
      '<div class="insp-row full"><label for="sdv_notes">Notes</label><input type="text" id="sdv_notes" value="' + esc(d.notes || '') + '" /></div>' +
      '</div>' +
      '<p class="hint">Emplacements are immobile crewed weapons placed in a room or on a flat roof, crewed automatically from the structure\'s garrison. Arrow slits, murder holes and portcullises aren\'t here: they are boundary types, set on a wall, floor or doorway in the structure editor (what it blocks, and from which side). Stubs, not simulated.</p>' +
      '<div class="danger-zone"><button type="button" class="btn small danger" id="sdv_remove">Remove from library</button>' +
      (defenseNotice ? '<span class="warn-inline" style="margin:0;">' + esc(defenseNotice) + '</span>' : '') + '</div>';
    [['sdv_label', 'label'], ['sdv_abbr', 'abbr'], ['sdv_mount', 'mount'], ['sdv_unit', 'manned_by_unit'], ['sdv_notes', 'notes'],
      ['sdv_fp', 'firing_points', true], ['sdv_dmg', 'stub_damage', true], ['sdv_range', 'stub_range', true]].forEach(function (b) {
      document.getElementById(b[0]).addEventListener('change', function () {
        d[b[1]] = b[2] ? Math.max(0, Number(this.value) || 0) : this.value;
        saveDefenseLib(); renderDefensesView();
      });
    });
    document.getElementById('sdv_remove').addEventListener('click', function () {
      if (defenseUsage(d.id)) { defenseNotice = 'Placed on the current map. Remove it from those structures first.'; renderDefensesView(); return; }
      staticDefenseLib = staticDefenseLib.filter(function (x) { return x.id !== d.id; });
      defenseNotice = ''; selectedDefenseId = null;
      saveDefenseLib(); renderDefensesView();
    });
  }
  document.getElementById('addDefenseBtn').addEventListener('click', function () {
    var input = document.getElementById('newDefenseId');
    var id = input.value.trim().toUpperCase().replace(/\s+/g, '_');
    if (!id) return;
    if (!staticDefenseDef(id)) {
      var label = id.charAt(0) + id.slice(1).toLowerCase().replace(/_/g, ' ');
      staticDefenseLib.push({ id: id, label: label, abbr: id.slice(0, 3), mount: 'ROOM', manned_by_unit: UNIT_LIBRARY[0], firing_points: 1, stub_damage: 0, stub_range: 1, notes: '' });
      saveDefenseLib();
    }
    input.value = '';
    selectedDefenseId = id;
    renderDefensesView();
  });

  // ---------- Structures view (prefabs + feature library) ----------
  var structuresSel = { kind: 'feature', id: null };
  var structuresNotice = '';
  function featureUsageCount(id) {
    var n = 0;
    Object.keys(state.cells).forEach(function (k) {
      var st = state.cells[k].structure;
      Object.keys(st.segments).forEach(function (sk) { st.segments[sk].features.forEach(function (f) { if (f === id) n++; }); });
    });
    return n;
  }
  function miniPreview(st) {
    var s = structureStats({ structure: st });
    if (!s.segments) return '<span class="sd-legend">empty</span>';
    var cols = Object.keys(st.segments).map(function (k) { return Number(k.split(',')[0]); });
    var minC = Math.min.apply(null, cols), maxC = Math.max.apply(null, cols);
    var html = '<div class="mini-prev" style="grid-template-columns:repeat(' + (maxC - minC + 1) + ', 18px);">';
    for (var l = s.height - 1; l >= -s.depth; l--) {
      for (var c = minC; c <= maxC; c++) html += '<span' + (st.segments[sKey(c, l)] ? ' data-on="true"' : '') + (l < 0 ? ' data-below="true"' : '') + '></span>';
    }
    return html + '</div>';
  }
  function renderStructuresView() {
    document.getElementById('featurePoints').value = featurePointsPerSegment();
    function libList(hostId, items, kind, labelFn, metaFn) {
      var host = document.getElementById(hostId);
      host.innerHTML = items.length ? '' : '<p class="empty-note" style="margin:0;">none yet</p>';
      items.forEach(function (it) {
        var el = document.createElement('div');
        el.className = 'terrain-item lib-row';
        el.tabIndex = 0;
        el.setAttribute('data-selected', structuresSel.kind === kind && structuresSel.id === it.id ? 'true' : 'false');
        el.innerHTML = '<span class="name">' + esc(labelFn(it)) + '</span><span class="mono" style="font-size:10px;color:var(--ink-dim);">' + esc(metaFn(it)) + '</span>';
        var pick = function () { structuresSel = { kind: kind, id: it.id }; structuresNotice = ''; renderStructuresView(); };
        el.addEventListener('click', pick);
        el.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') pick(); });
        host.appendChild(el);
      });
    }
    libList('structurePrefabList', structurePrefabLib, 'structure', function (x) { return x.name; }, function (x) { var r = x.requirements; return r.segments + ' seg · ' + r.height + 'h ' + r.width + 'w dig ' + r.depth; });
    libList('roomPrefabList', roomPrefabLib, 'room', function (x) { return x.label; }, function (x) { return prefabCost(x) + ' pts'; });
    libList('featureLibList2', featureLib.features, 'feature', function (x) { return x.label; }, function (x) { return x.cost + ' pt'; });

    var detail = document.getElementById('structuresDetail');
    var html = '';
    if (structuresSel.kind === 'structure') {
      var sp = structurePrefabLib.find(function (x) { return x.id === structuresSel.id; });
      if (sp) {
        var r = sp.requirements;
        html = '<div class="detail-title"><h2>' + esc(sp.name) + '</h2><span class="mono" style="color:var(--ink-dim);">structure prefab</span></div>' +
          '<p class="usage">Needs a tile with stability ≥ ' + r.segments + ', height ≥ ' + r.height + ', width ≥ ' + r.width + ', dig depth ≥ ' + r.depth + '. Archetype: ' + esc(archetypeLabel(sp.structure.archetype)) + '.</p>' +
          miniPreview(sp.structure) +
          '<div class="form-grid" style="margin-top:12px;"><div class="insp-row full"><label for="sv_name">Name</label><input type="text" id="sv_name" value="' + esc(sp.name) + '" /></div></div>' +
          '<p class="hint">Load it onto a node from Lanes → Structure → Prefabs. Loading checks the tile\'s capacity.</p>' +
          '<div class="danger-zone"><button type="button" class="btn small danger" id="sv_remove">Delete prefab</button></div>';
      }
    } else if (structuresSel.kind === 'room') {
      var rp = roomPrefabDef(structuresSel.id);
      if (rp) {
        var cost = prefabCost(rp), budget = featurePointsPerSegment();
        html = '<div class="detail-title"><h2>' + esc(rp.label) + '</h2><span class="mono" style="color:var(--ink-dim);">room prefab</span></div>' +
          '<p class="usage">Uses <span class="' + (cost > budget ? 'over-text' : '') + '">' + cost + ' of ' + budget + '</span> feature points.</p>' +
          '<div class="form-grid"><div class="insp-row full"><label for="sv_rlabel">Label</label><input type="text" id="sv_rlabel" value="' + esc(rp.label) + '" /></div></div>' +
          '<div class="insp-row" style="max-width:520px;"><label>Features</label><div class="chip-list">' +
          (rp.features.length ? rp.features.map(function (f, i) { var d = featureDefOf(f); return '<span class="mini-chip">' + esc((d ? d.label : f) + ' · ' + (d ? d.cost : '?')) + '<button type="button" class="row-remove" data-rprm="' + i + '" aria-label="Remove">&times;</button></span>'; }).join('') : '<span class="sd-legend">none</span>') +
          '</div><div class="patrol-add"><select id="sv_rfeat" aria-label="Feature">' + featureLib.features.map(function (f) { return '<option value="' + esc(f.id) + '">' + esc(f.label + ' · ' + f.cost + ' pt') + '</option>'; }).join('') + '</select><button type="button" class="btn small" id="sv_rfeatAdd">Add</button></div></div>' +
          '<p class="hint">Applying a room prefab sets a segment\'s name and furniture, e.g. Barracks = 5 Bunks. Murder holes, arrow slits and portcullises are boundary types, not room features.</p>' +
          '<div class="danger-zone"><button type="button" class="btn small danger" id="sv_remove">Delete room prefab</button></div>';
      }
    } else {
      var f = featureDefOf(structuresSel.id) || featureLib.features[0];
      if (f) {
        structuresSel = { kind: 'feature', id: f.id };
        var n = featureUsageCount(f.id);
        html = '<div class="detail-title"><h2>' + esc(f.label) + '</h2><span class="mono" style="color:var(--ink-dim);">' + esc(f.id) + '</span></div>' +
          '<p class="usage">On the current map: used ' + plural(n, 'time') + '.</p>' +
          '<div class="form-grid">' +
          '<div class="insp-row"><label for="sv_flabel">Label</label><input type="text" id="sv_flabel" value="' + esc(f.label) + '" /></div>' +
          '<div class="insp-row"><label for="sv_fcost">Cost (feature points)</label><input type="number" min="0" id="sv_fcost" value="' + esc(f.cost) + '" /></div>' +
          '<div class="insp-row full"><label for="sv_feffect">Effect (stub)</label><input type="text" id="sv_feffect" value="' + esc(f.effect) + '" placeholder="e.g. rest 4 units" /></div>' +
          '</div>' +
          '<p class="hint">Room features are furniture. Any feature fits any room, limited only by the segment\'s feature points.</p>' +
          '<div class="danger-zone"><button type="button" class="btn small danger" id="sv_remove">Remove feature</button></div>';
      }
    }
    detail.innerHTML = (html || '<p class="empty-note">Pick a prefab or feature to edit it.</p>') +
      (structuresNotice ? '<p class="warn-inline">' + esc(structuresNotice) + '</p>' : '');

    var rm = document.getElementById('sv_remove');
    if (structuresSel.kind === 'structure' && document.getElementById('sv_name')) {
      var spS = structurePrefabLib.find(function (x) { return x.id === structuresSel.id; });
      document.getElementById('sv_name').addEventListener('change', function () { spS.name = this.value.trim() || spS.name; saveStructurePrefabs(); renderStructuresView(); });
      rm.addEventListener('click', function () { structurePrefabLib = structurePrefabLib.filter(function (x) { return x !== spS; }); structuresSel = { kind: 'feature', id: null }; saveStructurePrefabs(); renderStructuresView(); });
    } else if (structuresSel.kind === 'room' && document.getElementById('sv_rlabel')) {
      var rpS = roomPrefabDef(structuresSel.id);
      document.getElementById('sv_rlabel').addEventListener('change', function () { rpS.label = this.value.trim() || rpS.label; saveRoomPrefabs(); renderStructuresView(); });
      document.getElementById('sv_rfeatAdd').addEventListener('click', function () { rpS.features.push(document.getElementById('sv_rfeat').value); saveRoomPrefabs(); renderStructuresView(); });
      detail.querySelectorAll('[data-rprm]').forEach(function (b) { b.addEventListener('click', function () { rpS.features.splice(Number(b.getAttribute('data-rprm')), 1); saveRoomPrefabs(); renderStructuresView(); }); });
      rm.addEventListener('click', function () { roomPrefabLib = roomPrefabLib.filter(function (x) { return x !== rpS; }); structuresSel = { kind: 'feature', id: null }; saveRoomPrefabs(); renderStructuresView(); });
    } else if (structuresSel.kind === 'feature' && document.getElementById('sv_flabel')) {
      var fS = featureDefOf(structuresSel.id);
      [['sv_flabel', 'label'], ['sv_feffect', 'effect'], ['sv_fcost', 'cost', true]].forEach(function (b) {
        document.getElementById(b[0]).addEventListener('change', function () { fS[b[1]] = b[2] ? Math.max(0, Number(this.value) || 0) : this.value; saveFeatureLib(); renderStructuresView(); });
      });
      rm.addEventListener('click', function () {
        if (featureUsageCount(fS.id)) { structuresNotice = 'Used on the current map. Remove it from those segments first.'; renderStructuresView(); return; }
        if (roomPrefabLib.some(function (rp2) { return rp2.features.indexOf(fS.id) !== -1; })) { structuresNotice = 'Used by a room prefab. Remove it there first.'; renderStructuresView(); return; }
        featureLib.features = featureLib.features.filter(function (x) { return x !== fS; });
        structuresSel = { kind: 'feature', id: null }; structuresNotice = '';
        saveFeatureLib(); renderStructuresView();
      });
    }
  }
  function libIdFrom(inputId) {
    var input = document.getElementById(inputId), id = input.value.trim().toUpperCase().replace(/\s+/g, '_');
    input.value = '';
    return id;
  }
  document.getElementById('addRoomPrefabBtn').addEventListener('click', function () {
    var id = libIdFrom('newRoomPrefabId');
    if (!id) return;
    if (!roomPrefabDef(id)) { roomPrefabLib.push({ id: id, label: id.charAt(0) + id.slice(1).toLowerCase().replace(/_/g, ' '), features: [] }); saveRoomPrefabs(); }
    structuresSel = { kind: 'room', id: id }; renderStructuresView();
  });
  document.getElementById('addFeatureDefBtn').addEventListener('click', function () {
    var id = libIdFrom('newFeatureDefId');
    if (!id) return;
    if (!featureDefOf(id)) { featureLib.features.push({ id: id, label: id.charAt(0) + id.slice(1).toLowerCase().replace(/_/g, ' '), cost: 1, effect: '' }); saveFeatureLib(); }
    structuresSel = { kind: 'feature', id: id }; renderStructuresView();
  });
  document.getElementById('featurePoints').addEventListener('change', function () {
    featureLib.feature_points_per_segment = Math.max(1, Number(this.value) || 5); saveFeatureLib(); renderStructuresView();
  });

  function structureExport(k, node) {
    var st = node.structure, s = structureStats(node), r = reachability(st);
    function ref(ck) { var p = ck.split(',').map(Number); return { col: p[0], level: p[1] }; }
    return {
      archetype: st.archetype,
      requirements: { stability: s.segments, height: s.height, width: s.width, dig_depth: s.depth },
      approaches: linkedNodeIds(node).map(function (o) { return { from_node: o, side: approachSide(node, o) }; }),
      segments: Object.keys(st.segments).map(function (ck) { var p = ck.split(',').map(Number), sg = st.segments[ck]; return { col: p[0], level: p[1], label: sg.label || null, features: sg.features, emplacements: sg.emplacements }; }),
      boundaries: allBoundaryKeys(st).map(function (bk) {
        var info = bInfo(st, bk), b = boundary(st, bk);
        return {
          kind: info.kind === 'ROOF' ? 'ROOF_ACCESS' : info.kind, col: info.col, level: info.level,
          exterior_side: info.exterior ? info.exteriorSide : null, outside: b.outside, preset: b.preset,
          material: info.kind === 'WALL' ? b.material : undefined,
          blocks_movement_from: b.move, blocks_projectiles_from: b.proj, blocks_sight_from: b.sight, fortification: b.fortification
        };
      }),
      roofs: Object.keys(st.roofs).map(function (c) { return { col: Number(c), top_level: colTop(st, Number(c)), type: st.roofs[c].type, emplacements: st.roofs[c].emplacements }; }),
      ranged_positions: firingPositions(st).length,
      manning: manning(node),
      unreachable_segments: s.segments ? r.unreachable.map(ref) : []
    };
  }

  function setActiveTool(tool) {
    state.activeTool = tool;
    state.linkPending = null;
    state.roadAnchor = null;
    state.armedPreset = null;
    if (tool !== 'TERRAIN') state.armedTerrain = null;
    if (tool !== 'FEATURE') state.armedFeature = null;
    document.querySelectorAll('.tool').forEach(function (t) {
      t.setAttribute('data-active', t.getAttribute('data-tool') === tool ? 'true' : 'false');
    });
    document.getElementById('linkHint').textContent = tool === 'link'
      ? 'Click a first node, then a second node to link them. Clicking two linked nodes unlinks them.'
      : 'Pick "Link two nodes" (Place tab), then click two placed nodes (click two linked nodes to unlink). A link is an intended route: its path is found across the terrain (cheaper on roads, blocked by unbridged water and other nodes). Unlinked nodes never get a route. Use Waypoint nodes to force a route through a point.';
    if (state.bottomTab === 'paint') renderLayersStrip();
    renderGrid();
  }

  document.querySelectorAll('.tool').forEach(function (t) {
    var tool = t.getAttribute('data-tool');
    t.addEventListener('click', function () { setActiveTool(tool); });
    t.addEventListener('keydown', function (ev) { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); setActiveTool(tool); } });
    if (t.draggable) t.addEventListener('dragstart', function (ev) { ev.dataTransfer.setData('text/tool', tool); });
  });

  // ---------- left sub-tabs ----------
  document.querySelectorAll('.tab-bar .tab-btn').forEach(function (btn) {
    btn.addEventListener('click', function () {
      var tab = btn.getAttribute('data-tab');
      document.querySelectorAll('.tab-bar .tab-btn').forEach(function (b) { b.setAttribute('data-active', b === btn ? 'true' : 'false'); });
      document.getElementById('tabPlace').hidden = tab !== 'place';
      document.getElementById('tabWorld').hidden = tab !== 'world';
    });
  });

  // ---------- World tab: map roster ----------
  function renderRoster() {
    var list = document.getElementById('factionRoster');
    list.innerHTML = '';
    activeFactions().forEach(function (f) {
      var el = document.createElement('div');
      el.className = 'faction-row';
      el.style.flexDirection = 'row';
      el.innerHTML = '<span style="display:flex;align-items:center;gap:6px;"><span class="swatch" style="background:' + factionColor(f.id) + '"></span><b>' + esc(f.display_name) + '</b>' +
        '<span class="mono" style="color:var(--ink-dim);font-size:10px;">' + esc(f.id) + '</span></span>';
      if (f.id !== 'player') {
        var rm = document.createElement('button');
        rm.type = 'button'; rm.className = 'row-remove'; rm.textContent = '×'; rm.setAttribute('aria-label', 'Remove ' + f.display_name + ' from this map');
        rm.addEventListener('click', function () {
          state.mapFactionIds = state.mapFactionIds.filter(function (id) { return id !== f.id; });
          state.faction_relations = state.faction_relations.filter(function (r) { return r.a !== f.id && r.b !== f.id; });
          state.lossCriteria = state.lossCriteria.filter(function (g) { return g.faction_id !== f.id; });
          dropHiddenFrom(f.id);
          safeSave(); renderMapFactionDependents();
        });
        el.appendChild(rm);
      } else {
        var tag = document.createElement('span');
        tag.className = 'mono'; tag.style.cssText = 'font-size:10px;color:var(--ink-dim);'; tag.textContent = 'always';
        el.appendChild(tag);
      }
      list.appendChild(el);
    });
    var addable = libraryFactions.filter(function (f) { return state.mapFactionIds.indexOf(f.id) === -1; });
    var sel = document.getElementById('rosterAddSel');
    sel.innerHTML = addable.length
      ? addable.map(function (f) { return '<option value="' + esc(f.id) + '">' + esc(f.display_name) + '</option>'; }).join('')
      : '<option value="">every library faction is on this map</option>';
    document.getElementById('rosterAddBtn').disabled = !addable.length;
  }
  document.getElementById('rosterAddBtn').addEventListener('click', function () {
    var id = document.getElementById('rosterAddSel').value;
    if (!id || state.mapFactionIds.indexOf(id) !== -1) return;
    state.mapFactionIds.push(id);
    applyDefaultsFor(id);
    safeSave(); renderMapFactionDependents();
  });

  function dropHiddenFrom(factionId) {
    Object.keys(state.cells).forEach(function (k) {
      var d = state.cells[k];
      d.hidden_from = (d.hidden_from || []).filter(function (x) { return x !== factionId; });
    });
    if (state.viewAs === factionId) state.viewAs = '';
  }
  function hiddenFromView(d) { return !!(state.viewAs && d && (d.hidden_from || []).indexOf(state.viewAs) !== -1); }
  function renderViewAsSelect() {
    // "View as" previews one faction's knowledge: nodes hidden from them fade out.
    var sel = document.getElementById('viewAs');
    if (!sel) return;
    if (state.viewAs && state.mapFactionIds.indexOf(state.viewAs) === -1) state.viewAs = '';
    sel.innerHTML = '<option value="">designer (everything)</option>' + activeFactions().map(function (f) {
      return '<option value="' + esc(f.id) + '"' + (f.id === state.viewAs ? ' selected' : '') + '>' + esc(f.display_name) + '</option>';
    }).join('');
  }
  document.getElementById('viewAs').addEventListener('change', function () { state.viewAs = this.value; renderGrid(); });

  function renderMapFactionDependents() {
    renderRoster(); renderRelations(); renderRelationSelects(); renderPlacementFactionSelect(); renderLoss(); renderInspector(); renderGrid();
  }

  function renderRelationSelects() {
    var options = activeFactions().map(function (f) { return '<option value="' + esc(f.id) + '">' + esc(f.display_name) + '</option>'; }).join('');
    ['relFactionA', 'relFactionB'].forEach(function (id) { document.getElementById(id).innerHTML = options; });
  }

  function renderRelations() {
    var list = document.getElementById('relationList');
    list.innerHTML = '';
    if (state.faction_relations.length === 0) {
      list.innerHTML = '<p class="empty-note" style="margin:0;">none declared</p>';
      return;
    }
    state.faction_relations.forEach(function (r, i) {
      var el = document.createElement('div');
      el.className = 'relation-row';
      var def = defaultStance(r.a, r.b);
      el.innerHTML = '<span>' + esc(factionName(r.a)) + ' &harr; ' + esc(factionName(r.b)) + ' &middot; ' + esc(r.stance) +
        (def && def !== r.stance ? ' <span style="color:var(--ink-dim);">(default ' + esc(def) + ')</span>' : '') + '</span>';
      var rm = document.createElement('button');
      rm.className = 'row-remove'; rm.type = 'button'; rm.textContent = '×'; rm.setAttribute('aria-label', 'Remove relation');
      rm.addEventListener('click', function () { state.faction_relations.splice(i, 1); safeSave(); renderRelations(); });
      el.appendChild(rm);
      list.appendChild(el);
    });
  }

  document.getElementById('addRelationBtn').addEventListener('click', function () {
    var a = document.getElementById('relFactionA').value;
    var b = document.getElementById('relFactionB').value;
    var stance = document.getElementById('relStance').value;
    if (!a || !b || a === b) return;
    state.faction_relations = state.faction_relations.filter(function (r) { return !pairMatch(r, a, b); });
    state.faction_relations.push({ a: a, b: b, stance: stance });
    safeSave(); renderRelations();
  });
  document.getElementById('resetRelationsBtn').addEventListener('click', function () {
    state.faction_relations = relationsFromDefaults();
    safeSave(); renderRelations();
  });

  // ---------- World tab: loss criteria ----------
  function lossWarnings() {
    var warnings = [];
    var grouped = {};
    state.lossCriteria.forEach(function (g) {
      g.node_ids.forEach(function (id) {
        grouped[id] = true;
        var k = findNodeKeyById(id);
        var n = k ? state.cells[k] : null;
        if (!n) warnings.push("loss group '" + g.name + "' (" + factionName(g.faction_id) + "): node '" + id + "' no longer exists");
        else if (!n.is_critical_asset) warnings.push("loss group '" + g.name + "': node '" + id + "' is not flagged as a critical asset");
        else if (n.owning_faction_id && n.owning_faction_id !== g.faction_id) warnings.push("loss group '" + g.name + "' belongs to " + factionName(g.faction_id) + " but node '" + id + "' is owned by " + factionName(n.owning_faction_id));
      });
      if (!g.node_ids.length) warnings.push("loss group '" + g.name + "' (" + factionName(g.faction_id) + ") has no nodes");
    });
    criticalNodes().forEach(function (n) {
      if (!grouped[n.id]) warnings.push("critical asset '" + n.id + "' is not in any loss group, so losing it has no effect");
    });
    return warnings;
  }

  function renderLoss() {
    var host = document.getElementById('lossHost');
    host.innerHTML = '';
    var crit = criticalNodes();
    activeFactions().forEach(function (f) {
      var box = document.createElement('div');
      box.className = 'loss-faction';
      box.innerHTML = '<div class="lf-head"><span class="swatch" style="background:' + factionColor(f.id) + '"></span>' + esc(f.display_name) + '</div>';
      var groups = state.lossCriteria.filter(function (g) { return g.faction_id === f.id; });
      groups.forEach(function (g) {
        var gEl = document.createElement('div');
        gEl.className = 'loss-group';
        var chips = g.node_ids.map(function (id, i) {
          return '<span class="node-chip">' + esc(id) + '<button type="button" class="row-remove" data-rm="' + i + '" aria-label="Remove ' + esc(id) + '">&times;</button></span>';
        }).join('') || '<span class="empty-note" style="font-size:10.5px;">no nodes yet</span>';
        var addable = crit.filter(function (n) { return g.node_ids.indexOf(n.id) === -1; });
        gEl.innerHTML =
          '<div class="lg-head"><input type="text" data-role="name" value="' + esc(g.name) + '" aria-label="Group name" />' +
          '<select data-role="rule" aria-label="Rule"><option value="ANY"' + (g.rule === 'ANY' ? ' selected' : '') + '>lose if any</option>' +
          '<option value="ALL"' + (g.rule === 'ALL' ? ' selected' : '') + '>lose if all</option></select>' +
          '<button type="button" class="row-remove" data-role="del" aria-label="Delete group">&times;</button></div>' +
          '<div class="node-chips">' + chips + '</div>' +
          '<div class="lg-head"><select data-role="addSel" aria-label="Critical node to add">' +
          (addable.length ? addable.map(function (n) { return '<option value="' + esc(n.id) + '">' + esc(n.id) + (n.owning_faction_id ? ' (' + esc(factionName(n.owning_faction_id)) + ')' : '') + '</option>'; }).join('') : '<option value="">no unassigned critical nodes</option>') +
          '</select><button type="button" class="btn small" data-role="add"' + (addable.length ? '' : ' disabled') + '>Add node</button></div>';
        gEl.querySelector('[data-role="name"]').addEventListener('change', function () { g.name = this.value.trim() || g.name; safeSave(); renderLoss(); });
        gEl.querySelector('[data-role="rule"]').addEventListener('change', function () { g.rule = this.value; safeSave(); renderLoss(); });
        gEl.querySelector('[data-role="del"]').addEventListener('click', function () {
          state.lossCriteria.splice(state.lossCriteria.indexOf(g), 1); safeSave(); renderLoss();
        });
        gEl.querySelector('[data-role="add"]').addEventListener('click', function () {
          var v = gEl.querySelector('[data-role="addSel"]').value;
          if (v) { g.node_ids.push(v); safeSave(); renderLoss(); }
        });
        gEl.querySelectorAll('[data-rm]').forEach(function (b) {
          b.addEventListener('click', function () { g.node_ids.splice(Number(b.getAttribute('data-rm')), 1); safeSave(); renderLoss(); });
        });
        box.appendChild(gEl);
      });
      var summary = document.createElement('div');
      summary.className = 'loss-summary';
      summary.textContent = groups.length
        ? 'Knocked out when ' + groups.map(function (g) {
          return g.name + ' (' + (g.rule === 'ALL' ? 'all of ' : 'any of ') + (g.node_ids.join(', ') || 'nothing') + ')';
        }).join(' OR ') + ' is lost.'
        : 'No loss criteria. This faction cannot be knocked out.';
      box.appendChild(summary);
      var addBtn = document.createElement('button');
      addBtn.type = 'button'; addBtn.className = 'btn small'; addBtn.style.marginTop = '6px'; addBtn.textContent = '+ Add group';
      addBtn.addEventListener('click', function () {
        state.lossCriteria.push({ faction_id: f.id, name: 'Group ' + (groups.length + 1), rule: 'ANY', node_ids: [] });
        safeSave(); renderLoss();
      });
      box.appendChild(addBtn);
      host.appendChild(box);
    });
    var warns = lossWarnings();
    if (warns.length) {
      var w = document.createElement('div');
      w.className = 'warn-inline';
      w.innerHTML = warns.map(function (x) { return '&bull; ' + esc(x); }).join('<br>');
      host.appendChild(w);
    }
    if (!crit.length) {
      var p = document.createElement('p');
      p.className = 'hint';
      p.textContent = 'No nodes are flagged as critical assets yet. Tick "Critical asset" on a node in the Inspector first.';
      host.appendChild(p);
    }
  }

  // ---------- links ----------
  function renderLinks() {
    var list = document.getElementById('linkList');
    list.innerHTML = '';
    var routes = computeRoutes();
    if (state.links.length === 0) {
      list.innerHTML = '<p class="empty-note" style="margin:0;">none yet</p>';
    } else {
      state.links.forEach(function (l, i) {
        var el = document.createElement('div');
        el.className = 'link-row';
        var rt = routes[i];
        el.innerHTML = '<span>' + esc(l.a) + ' &harr; ' + esc(l.b) + ' <span class="mono" style="font-size:10px;color:' + (rt.found ? 'var(--ink-dim)' : 'var(--danger)') + ';">' +
          (rt.found ? '· ' + plural(rt.cells.length - 1, 'step') + ' · cost ' + rt.cost : '· no passable route') + '</span></span>';
        var rm = document.createElement('button');
        rm.className = 'row-remove'; rm.type = 'button'; rm.textContent = '×'; rm.setAttribute('aria-label', 'Remove link');
        rm.addEventListener('click', function () { state.links.splice(i, 1); linksChanged(); });
        el.appendChild(rm);
        list.appendChild(el);
      });
    }
    renderLinkOverlay(routes);
  }

  function cellCenterPx(k) {
    var p = parseKey(k);
    return { x: p[1] * (CELL + GAP) + CELL / 2, y: p[0] * (CELL + GAP) + CELL / 2 };
  }

  // ---------- routes (Decision 26) ----------
  // A link is an authored, intended route: it alone decides WHICH nodes connect. Its
  // geometry is pathfound here across terrain cost - cheaper along drawn roads, water/
  // ravine impassable without a bridge, other nodes' cells not enterable. Unlinked node
  // pairs never get a route. Unit-class blocking (blocks_unit_classes) is ignored: routes
  // are unit-agnostic for now.
  var IMPASSABLE = 99;
  function enterCost(from, to, target) {
    if (to !== target && state.cells[to]) return Infinity;
    var d = terrainDef(tileTerrainId(to)) || {};
    var base;
    var br = tileAt(to).bridge;
    if (d.needs_bridge) base = br && !br.raised ? 1 : Infinity; // a raised drawbridge is no bridge
    else base = Number(d.move_cost) >= IMPASSABLE ? Infinity : (Number(d.move_cost) || 1);
    if (base === Infinity) return Infinity;
    if (roadIndex(from, to) !== -1) base *= Number(terrainLib.road_move_multiplier) || 1;
    var a = parseKey(from), b = parseKey(to);
    return (a[0] !== b[0] && a[1] !== b[1]) ? base * Math.SQRT2 : base;
  }
  function findRoute(startKey, targetKey) {
    // Dijkstra over 8-neighbour cells; the grid is at most 24x16, so a linear scan is fine.
    var dist = {}, prev = {}, done = {};
    dist[startKey] = 0;
    var open = [startKey];
    while (open.length) {
      var bi = 0;
      for (var i = 1; i < open.length; i++) if (dist[open[i]] < dist[open[bi]]) bi = i;
      var cur = open.splice(bi, 1)[0];
      if (done[cur]) continue;
      done[cur] = true;
      if (cur === targetKey) break;
      var p = parseKey(cur);
      for (var dr = -1; dr <= 1; dr++) {
        for (var dc = -1; dc <= 1; dc++) {
          if (!dr && !dc) continue;
          var r = p[0] + dr, c = p[1] + dc;
          if (r < 0 || c < 0 || r >= state.rows || c >= state.cols) continue;
          var nk = key(r, c);
          if (done[nk]) continue;
          var cost = enterCost(cur, nk, targetKey);
          if (cost === Infinity) continue;
          var nd = dist[cur] + cost;
          if (dist[nk] === undefined || nd < dist[nk]) { dist[nk] = nd; prev[nk] = cur; open.push(nk); }
        }
      }
    }
    if (dist[targetKey] === undefined) return { found: false, cells: [], cost: null };
    var cells = [targetKey];
    while (cells[0] !== startKey) cells.unshift(prev[cells[0]]);
    return { found: true, cells: cells, cost: Math.round(dist[targetKey] * 100) / 100 };
  }
  function computeRoutes() {
    return state.links.map(function (l) {
      var ka = findNodeKeyById(l.a), kb = findNodeKeyById(l.b);
      if (!ka || !kb) return { found: false, cells: [], cost: null, missing: true };
      return findRoute(ka, kb);
    });
  }

  function computePathCells(routes) {
    var covered = {};
    routes.forEach(function (rt) { rt.cells.forEach(function (k) { covered[k] = true; }); });
    return covered;
  }

  function renderLinkOverlay(routes) {
    routes = routes || computeRoutes();
    var svg = document.getElementById('linkOverlay');
    var w = state.cols * (CELL + GAP) - GAP, h = state.rows * (CELL + GAP) - GAP;
    svg.setAttribute('width', w); svg.setAttribute('height', h);
    svg.setAttribute('viewBox', '0 0 ' + w + ' ' + h);
    svg.innerHTML = '';
    var css = getComputedStyle(document.body);
    var good = css.getPropertyValue('--good').trim() || '#4c7a52';
    var danger = css.getPropertyValue('--danger').trim() || '#a3372a';
    state.links.forEach(function (l, i) {
      var rt = routes[i];
      var ka = findNodeKeyById(l.a), kb = findNodeKeyById(l.b);
      if (!ka || !kb) return;
      var el = document.createElementNS('http://www.w3.org/2000/svg', 'polyline');
      if (rt.found) {
        el.setAttribute('points', rt.cells.map(function (k) { var c = cellCenterPx(k); return c.x + ',' + c.y; }).join(' '));
        el.setAttribute('stroke', good);
      } else {
        // No passable route: show the intent as a dashed straight line.
        var pa = cellCenterPx(ka), pb = cellCenterPx(kb);
        el.setAttribute('points', pa.x + ',' + pa.y + ' ' + pb.x + ',' + pb.y);
        el.setAttribute('stroke', danger);
        el.setAttribute('stroke-dasharray', '5 4');
      }
      if (hiddenFromView(state.cells[ka]) || hiddenFromView(state.cells[kb])) { el.setAttribute('stroke-dasharray', '2 5'); el.setAttribute('opacity', '0.35'); }
      el.setAttribute('fill', 'none');
      el.setAttribute('stroke-width', '2.5');
      el.setAttribute('stroke-linecap', 'round');
      el.setAttribute('stroke-linejoin', 'round');
      svg.appendChild(el);
    });
  }

  function routeWarnings(routes) {
    var w = [];
    state.links.forEach(function (l, i) {
      if (!routes[i].found && !routes[i].missing) {
        w.push("link '" + l.a + "' <-> '" + l.b + "' has no passable route (water without a bridge, impassable terrain, or boxed in by other nodes)");
      }
    });
    return w;
  }

  // ---------- grid ----------
  var isPainting = false;
  document.addEventListener('mouseup', function () { isPainting = false; state.roadAnchor = null; state.drawingChannel = null; });

  function renderGrid() {
    renderViewAsSelect();
    var grid = document.getElementById('grid');
    grid.style.gridTemplateColumns = 'repeat(' + state.cols + ', ' + CELL + 'px)';
    grid.innerHTML = '';
    var routes = computeRoutes();
    var onPath = computePathCells(routes);
    var adj = roadAdjacency();
    var base = terrainDef(state.defaultTerrain);
    for (var r = 0; r < state.rows; r++) {
      for (var c = 0; c < state.cols; c++) {
        var k = key(r, c);
        var cellEl = document.createElement('div');
        cellEl.className = 'cell';
        cellEl.dataset.key = k;
        if (state.selectedCell === k) cellEl.setAttribute('data-selected', 'true');
        var areaOf = areaOwner(k);
        if (areaOf && footprintOf(areaOf).length > 1) {
          cellEl.setAttribute('data-area', state.cells[areaOf].id);
          if (areaOf === state.selectedCell) cellEl.setAttribute('data-area-selected', 'true');
        }
        if (state.linkPending === k) cellEl.setAttribute('data-link-pending', 'true');
        if (onPath[k]) cellEl.setAttribute('data-on-path', 'true');

        var t = state.tiles[k];
        var tDef = t && t.terrain ? terrainDef(t.terrain) : null;
        var isBase = !(t && t.terrain);
        var shown = isBase ? base : tDef;
        if (shown) {
          var tLayer = document.createElement('div');
          tLayer.className = 'terrain-layer' + (isBase ? ' base' : '');
          tLayer.style.background = shown.color;
          tLayer.textContent = shown.glyph;
          cellEl.appendChild(tLayer);
        }
        if (t && t.bridge) {
          var br = document.createElement('div'); br.className = 'bridge-layer'; br.title = 'Bridge';
          cellEl.appendChild(br);
        }
        channelNeighbours(k).forEach(function (nk) {
          var cHub = document.createElement('div'); cHub.className = 'channel-seg hub'; cellEl.appendChild(cHub);
          if (!nk) return;
          var ca = parseKey(k), cb = parseKey(nk);
          var cdx = (cb[1] - ca[1]) * (CELL + GAP), cdy = (cb[0] - ca[0]) * (CELL + GAP);
          var cHalf = document.createElement('div');
          cHalf.className = 'channel-seg half';
          cHalf.style.width = (Math.hypot(cdx, cdy) / 2) + 'px';
          cHalf.style.transform = 'rotate(' + (Math.atan2(cdy, cdx) * 180 / Math.PI) + 'deg)';
          cellEl.appendChild(cHalf);
        });
        if (adj[k]) {
          var hub = document.createElement('div'); hub.className = 'road-seg hub'; cellEl.appendChild(hub);
          adj[k].forEach(function (nk) {
            var a = parseKey(k), b = parseKey(nk);
            var dx = (b[1] - a[1]) * (CELL + GAP), dy = (b[0] - a[0]) * (CELL + GAP);
            var half = document.createElement('div');
            half.className = 'road-seg half';
            half.style.width = (Math.hypot(dx, dy) / 2) + 'px';
            half.style.transform = 'rotate(' + (Math.atan2(dy, dx) * 180 / Math.PI) + 'deg)';
            cellEl.appendChild(half);
          });
        }
        if (t && t.feature) {
          var fDef = featureDef(t.feature);
          var fb = document.createElement('div');
          fb.className = 'feature-badge'; fb.textContent = fDef ? fDef.glyph : '?';
          fb.title = fDef ? fDef.label : t.feature;
          cellEl.appendChild(fb);
        }
        var diag = diagnosticColour(k);
        if (diag) { var dl = document.createElement('div'); dl.className = 'diag-layer'; dl.style.background = diag.colour; dl.title = diag.title; cellEl.appendChild(dl); }
        var elev = elevationAt(k);
        if (elev !== Number((base || {}).default_elevation || 0)) { // only where the ground differs from the base terrain's
          var eb = document.createElement('div');
          eb.className = 'elev-badge' + (elev >= state.ceiling ? ' capped' : '');
          eb.textContent = '▲' + elev;
          eb.title = 'Elevation ' + elev + ' cells' + (t && t.elevation != null ? '' : ' (terrain default)') + (elev >= state.ceiling ? ' - at the ceiling' : '');
          cellEl.appendChild(eb);
        }
        if (t && t.upgrades && t.upgrades.length) {
          var ub = document.createElement('div');
          ub.className = 'upg-badge'; ub.textContent = '+' + t.upgrades.length;
          cellEl.appendChild(ub);
        }

        var data = state.cells[k];
        if (data) {
          var isWaypoint = data.node_type === 'WAYPOINT';
          var tile = document.createElement('div');
          tile.className = 'tile ' + data.node_type + (isWaypoint ? ' waypoint' : '');
          tile.textContent = isWaypoint ? '' : data.node_type.charAt(0);
          tile.style.borderColor = data.owning_faction_id ? factionColor(data.owning_faction_id) : 'transparent';
          if (data.is_critical_asset) {
            var badge = document.createElement('div');
            badge.className = 'home-badge'; badge.textContent = '★';
            cellEl.appendChild(badge);
          }
          if ((data.hidden_from || []).length) {
            var hb = document.createElement('div');
            hb.className = 'hidden-badge'; hb.textContent = '◌';
            hb.title = 'Hidden from ' + data.hidden_from.map(factionName).join(', ');
            cellEl.appendChild(hb);
          }
          if (hiddenFromView(data)) cellEl.setAttribute('data-hidden-view', 'true');
          if (structureStats(data).segments) {
            var pip = document.createElement('div');
            pip.className = 'structure-pip';
            pip.textContent = profileGlyphs(data);
            pip.title = archetypeLabel(data.structure.archetype) + ' profile';
            cellEl.appendChild(pip);
          }
          cellEl.appendChild(tile);
          if (data.id) {
            var idBadge = document.createElement('div');
            idBadge.className = 'sub-badge'; idBadge.textContent = data.id;
            cellEl.appendChild(idBadge);
          }
        }

        cellEl.addEventListener('dragover', function (ev) { ev.preventDefault(); });
        cellEl.addEventListener('drop', function (ev) {
          ev.preventDefault();
          var tool = ev.dataTransfer.getData('text/tool');
          if (tool) applyTool(this.dataset.key, tool);
        });
        cellEl.addEventListener('mousedown', function (ev) { ev.preventDefault(); isPainting = true; handleCellAction(this.dataset.key, true); });
        cellEl.addEventListener('mouseenter', function () {
          if (!isPainting) return;
          var tool = state.activeTool;
          if (tool !== 'select' && tool !== 'link' && tool !== 'NODE' && tool !== 'ROAD' && tool !== 'CHANNEL') handleCellAction(this.dataset.key, false);
        });
        cellEl.addEventListener('mousemove', function (ev) {
          // Roads only register a cell near its centre, so a diagonal stroke through a
          // shared corner doesn't also catch the two side cells.
          if (!isPainting || (state.activeTool !== 'ROAD' && state.activeTool !== 'CHANNEL') || this.dataset.key === state.roadAnchor) return;
          var rect = this.getBoundingClientRect();
          var dx = ev.clientX - (rect.left + rect.width / 2), dy = ev.clientY - (rect.top + rect.height / 2);
          if (Math.abs(dx) < rect.width * 0.3 && Math.abs(dy) < rect.height * 0.3) handleCellAction(this.dataset.key, false);
        });

        grid.appendChild(cellEl);
      }
    }
    renderLinkOverlay(routes);
  }

  function selectCell(k) {
    if (state.selectedCell !== k) state.structureSel = null;
    state.selectedCell = k;
    var sn = nodeAt(k);
    if (state.bottomTab === 'structure' && (!sn || sn.node_type === 'WAYPOINT')) state.bottomTab = 'paint';
    setBottomTab(state.bottomTab);
    renderInspector();
  }

  // ---------- node areas (Decision 68) ----------
  // A node covers a footprint of tiles, any connected shape including its own tile.
  function footprintOf(k) { var d = state.cells[k]; return d && d.footprint && d.footprint.length ? d.footprint : [k]; }
  // The key of the node whose footprint covers tile k, or null.
  function areaOwner(k) {
    var owner = null;
    Object.keys(state.cells).forEach(function (nk) { if (footprintOf(nk).indexOf(k) !== -1) owner = nk; });
    return owner;
  }
  function connected(keys, from) {
    var seen = {}, todo = [from];
    seen[from] = true;
    while (todo.length) {
      var p = parseKey(todo.pop());
      [[1, 0], [-1, 0], [0, 1], [0, -1]].forEach(function (d) {
        var n = key(p[0] + d[0], p[1] + d[1]);
        if (keys.indexOf(n) !== -1 && !seen[n]) { seen[n] = true; todo.push(n); }
      });
    }
    return Object.keys(seen).length === keys.length;
  }
  // Adds or removes tile k from the selected node's footprint, keeping it connected.
  function paintArea(k, adding) {
    var nk = state.selectedCell, d = nk ? state.cells[nk] : null;
    if (!d) { setNotice('Select a node first, then paint the tiles it covers.'); return; }
    var area = footprintOf(nk).slice(), at = area.indexOf(k);
    if (adding && at === -1) {
      var owner = areaOwner(k);
      if (owner && owner !== nk) { setNotice('That tile belongs to ' + state.cells[owner].id + '.'); return; }
      area.push(k);
      if (!connected(area, nk)) { setNotice('A node area must be one connected shape.'); return; }
    } else if (!adding && at !== -1) {
      if (k === nk) { setNotice('A node always covers its own tile.'); return; }
      area.splice(at, 1);
      if (!connected(area, nk)) { setNotice('Removing that tile would split the area.'); return; }
    } else return;
    d.footprint = area.length > 1 ? area : undefined;
    setNotice(''); safeSave(); renderGrid();
  }

  function handleCellAction(k, isDown) {
    if (state.activeTool === 'AREA') {
      if (isDown) state.areaAdding = !state.selectedCell || footprintOf(state.selectedCell).indexOf(k) === -1;
      paintArea(k, state.areaAdding);
      return;
    }
    if (state.activeTool === 'select') {
      // Any cell is selectable - empty or not - so its tile layers can be edited.
      selectCell(k); renderGrid();
      return;
    }
    if (state.activeTool === 'link') {
      var cell = state.cells[k];
      if (!cell) return;
      if (!state.linkPending) {
        state.linkPending = k; renderGrid();
      } else if (state.linkPending !== k) {
        var a = state.cells[state.linkPending].id, b = cell.id;
        var at = state.links.findIndex(function (l) { return (l.a === a && l.b === b) || (l.a === b && l.b === a); });
        if (at !== -1) { state.links.splice(at, 1); setNotice('Unlinked ' + a + ' \u2194 ' + b + '.'); }
        else if (a !== b) { state.links.push({ a: a, b: b }); setNotice(''); }
        state.linkPending = null;
        linksChanged();
      }
      return;
    }
    if (state.activeTool === 'CHANNEL') {
      // A drag draws one channel through the tiles it crosses; a click alone makes a pond.
      if (isDown) {
        state.channels.push({ tiles: [k], width: 4, depth: 2, liquid: 'WATER', liquid_depth: 1 });
        state.drawingChannel = state.channels.length - 1;
      } else if (state.drawingChannel != null) {
        var drawn = state.channels[state.drawingChannel], last = parseKey(drawn.tiles[drawn.tiles.length - 1]), here = parseKey(k);
        if (drawn.tiles.indexOf(k) === -1 && Math.max(Math.abs(last[0] - here[0]), Math.abs(last[1] - here[1])) === 1) drawn.tiles.push(k);
      }
      state.roadAnchor = k;
      safeSave(); renderGrid();
      if (state.selectedCell) renderInspector();
      return;
    }
    if (state.activeTool === 'ROAD') {
      // Roads join only the cells you drag between - never auto-joined to neighbours.
      if (isDown) {
        state.roadAnchor = k;
        if (roadBlocker(k)) bridgeNotice(k); // say so straight away, before any drag
        return;
      }
      if (state.roadAnchor && state.roadAnchor !== k) tryAddRoad(state.roadAnchor, k);
      state.roadAnchor = k;
      renderGrid(); renderCounts(); renderLinks(); // roads change link routes and costs
      if (state.selectedCell) renderInspector();
      return;
    }
    applyTool(k, state.activeTool);
  }

  function refreshSelectionPanels() {
    if (state.selectedCell) renderInspector();
    if (state.bottomTab === 'structure') renderStructureEditor();
  }
  function pruneApproaches() {
    Object.keys(state.cells).forEach(function (k) {
      var d = state.cells[k];
      if (!d.approaches) return;
      var linked = linkedNodeIds(d);
      Object.keys(d.approaches).forEach(function (o) { if (linked.indexOf(o) === -1) delete d.approaches[o]; });
      if (!Object.keys(d.approaches).length) delete d.approaches;
    });
  }
  function linksChanged() {
    pruneApproaches();
    safeSave(); renderGrid(); renderLinks(); renderCounts(); renderLoss();
    refreshSelectionPanels();
  }

  function removeNodeRefs(id) {
    state.links = state.links.filter(function (l) { return l.a !== id && l.b !== id; });
    state.lossCriteria.forEach(function (g) { g.node_ids = g.node_ids.filter(function (x) { return x !== id; }); });
  }

  // Layers the Erase tool can target on their own (the layer strip's Erase row).
  function eraseLayerList() {
    return [['NODE', 'Node'], ['UPGRADES', 'Upgrades'], ['ROADS', 'Roads'], ['CHANNELS', 'Channels'], ['BRIDGE', 'Bridge'],
      ['FEATURE', 'Feature'], ['TERRAIN', 'Terrain'], ['CAPACITY', 'Capacity overrides']];
  }
  function eraseLayersAt(k, layers) {
    var t = state.tiles[k];
    function on(id) { return layers.indexOf(id) !== -1; }
    if (on('NODE') && state.cells[k]) { removeNodeRefs(state.cells[k].id); delete state.cells[k]; }
    if (on('ROADS')) removeRoadsAt(k);
    if (on('CHANNELS')) removeChannelsAt(k);
    if (t) {
      if (on('UPGRADES')) { delete t.upgrades; delete t.upgrade_slots; }
      if (on('BRIDGE')) delete t.bridge;
      if (on('FEATURE')) delete t.feature;
      if (on('TERRAIN')) delete t.terrain;
      if (on('CAPACITY')) ['stability', 'max_height', 'max_width', 'max_length', 'max_depth'].forEach(function (f) { delete t[f]; });
    }
    pruneTile(k);
  }

  function applyTool(k, tool) {
    var linkCount = state.links.length;
    applyToolInner(k, tool);
    if (state.links.length !== linkCount) linksChanged();
    else renderLinks(); // terrain, roads and bridges all change link routes and costs
  }

  function applyToolInner(k, tool) {
    if (tool === 'eraser' && state.eraseLayers.length) {
      eraseLayersAt(k, state.eraseLayers);
      renderLinks(); renderLoss();
      if (state.selectedCell === k) { setBottomTab(state.bottomTab); renderInspector(); }
    } else if (tool === 'eraser') {
      // Topmost layer first: node, upgrades, roads, bridge, feature, terrain (+ overrides).
      var removed = state.cells[k];
      var t = state.tiles[k];
      var hasRoad = state.roads.some(function (e) { return e.a === k || e.b === k; });
      if (removed) {
        removeNodeRefs(removed.id);
        delete state.cells[k];
      } else if (t && t.upgrades && t.upgrades.length) {
        t.upgrades = []; delete t.upgrade_slots;
      } else if (hasRoad) {
        removeRoadsAt(k);
      } else if (t && t.bridge) {
        delete t.bridge;
      } else if (t && t.feature) {
        delete t.feature;
      } else if (t) {
        delete state.tiles[k];
      }
      pruneTile(k);
      renderLinks(); renderLoss();
      if (state.selectedCell === k) { setBottomTab(state.bottomTab); renderInspector(); }
    } else if (tool === 'TERRAIN') {
      var paintDef = terrainDef(state.armedTerrain || 'WATER');
      if (paintDef && paintDef.needs_bridge && !tileAt(k).bridge && hasRoadAt(k)) { roadUnderWaterNotice(paintDef.label); return; }
      ensureTile(k).terrain = state.armedTerrain || 'WATER';
      if (state.selectedCell === k) renderInspector();
    } else if (tool === 'FEATURE') {
      ensureTile(k).feature = state.armedFeature;
      if (state.selectedCell === k) renderInspector();
    } else if (tool === 'BRIDGE') {
      if (!needsBridge(k)) {
        setNotice('Bridges go on terrain that needs one (e.g. water, ravine). ' + cellLabel(k) + ' is ' + ((terrainDef(tileTerrainId(k)) || {}).label || 'land') + '.');
        return;
      }
      if (!tileAt(k).bridge) ensureTile(k).bridge = { owning_faction_id: state.placementFactionId || '', hp: 40, demolishable_by_owner: false };
      setNotice('');
      if (state.selectedCell === k) renderInspector();
    } else if (tool === 'NODE') {
      if (!state.cells[k]) {
        var preset = state.armedPreset;
        state.cells[k] = {
          type: 'NODE', id: 'n' + (state.nextId.NODE++),
          node_type: preset ? preset.node_type : 'NEUTRAL',
          is_critical_asset: false,
          owning_faction_id: state.placementFactionId || '',
          structure: preset ? clone(preset.structure) : struct('NONE'),
          fields: preset ? clone(preset.fields) : {},
          garrison_units: []
        };
      }
      selectCell(k);
    }
    safeSave();
    renderGrid();
    renderCounts();
  }

  // ---------- inspector ----------
  function fieldRow(labelText, inputHtml) { return '<div class="insp-row"><label>' + labelText + '</label>' + inputHtml + '</div>'; }
  function selectInput(id, options, current, includeNone, labelFn, noneLabel) {
    var opts = includeNone ? ['<option value="">' + esc(noneLabel || '(none)') + '</option>'] : [];
    options.forEach(function (o) {
      var text = labelFn ? labelFn(o) : o;
      opts.push('<option value="' + esc(o) + '"' + (o === current ? ' selected' : '') + '>' + esc(text) + '</option>');
    });
    return '<select id="' + id + '">' + opts.join('') + '</select>';
  }
  function factionSelect(id, current) {
    var options = activeFactions();
    return selectInput(id, options.map(function (f) { return f.id; }), current, true, function (fid) { return factionName(fid); });
  }
  function checkboxRow(id, labelText, checked) {
    return '<div class="insp-row"><label class="checkbox-row"><input type="checkbox" id="' + id + '"' + (checked ? ' checked' : '') + ' /> ' + labelText + '</label></div>';
  }
  function numInput(id, val, placeholder) {
    return '<input type="number" min="0" id="' + id + '" value="' + (val === undefined || val === null ? '' : esc(val)) + '"' + (placeholder ? ' placeholder="' + esc(placeholder) + '"' : '') + ' />';
  }
  function warn(text) { return '<p class="warn-inline">' + esc(text) + '</p>'; }

  function tileSectionHtml(k) {
    var t = tileAt(k);
    var base = terrainDef(state.defaultTerrain);
    var terr = terrainDef(tileTerrainId(k)) || {};
    var roadCount = state.roads.filter(function (e) { return e.a === k || e.b === k; }).length;
    var html = '<div class="insp-section-label" style="margin-top:0;border-top:none;padding-top:0;">Ground layers</div>';
    html += fieldRow('Terrain', selectInput('t_terrain', terrainLib.terrains.map(function (x) { return x.id; }), t.terrain || '', true,
      function (id) { var d = terrainDef(id); return d ? d.glyph + ' ' + d.label : id; }, 'Map base (' + (base ? base.label : state.defaultTerrain) + ')'));
    html += fieldRow('Elevation (cells)', numInput('t_elev', t.elevation, 'def ' + (terr.default_elevation || 0))) +
      (elevationAt(k) >= state.ceiling ? warn('At or above the map ceiling (' + state.ceiling + '): impassable, drawn capped.') : '');
    html += fieldRow('Natural feature', selectInput('t_feature', terrainLib.features.map(function (x) { return x.id; }), t.feature || '', true,
      function (id) { var d = featureDef(id); return d ? d.glyph + ' ' + d.label : id; }));

    // Bridge + roads
    if (terr.needs_bridge || t.bridge) {
      html += '<div class="insp-row"><label>Bridge</label>';
      if (t.bridge) {
        html += '<div class="garrison-unit-card" style="margin:0;">' +
          '<div class="gu-row"><label>Owner</label>' + factionSelect('t_bOwner', t.bridge.owning_faction_id) + '</div>' +
          '<div class="gu-row"><label>HP (stub)</label>' + numInput('t_bHp', t.bridge.hp) + '</div>' +
          '<label class="checkbox-row" style="text-transform:none;letter-spacing:0;font-size:12px;"><input type="checkbox" id="t_bDemo"' + (t.bridge.demolishable_by_owner ? ' checked' : '') + ' /> Owner can demolish it (defensive tactic)</label>' +
          '<div class="gu-row" style="margin-top:6px;"><label>Drawbridge controlled by</label>' + selectInput('t_bDraw', adjacentNodeIds(k), t.bridge.drawbridge_node_id || '', true, null, '(not a drawbridge)') + '</div>' +
          (t.bridge.drawbridge_node_id ? '<label class="checkbox-row" style="text-transform:none;letter-spacing:0;font-size:12px;"><input type="checkbox" id="t_bRaised"' + (t.bridge.raised ? ' checked' : '') + ' /> Raised (blocks routes over it)</label>' : '') +
          '<div style="margin-top:6px;"><button type="button" class="btn small danger" id="t_bRemove">Remove bridge</button></div></div>';
        if (!terr.needs_bridge) html += warn('This terrain doesn\'t need a bridge.');
      } else {
        html += '<p class="empty-note" style="margin:0 0 4px;">' + esc(terr.label || 'This terrain') + ' needs a bridge before a road can cross.</p>' +
          '<button type="button" class="btn small" id="t_bAdd">Place bridge</button>';
      }
      html += '</div>';
    }
    html += '<p class="hint" style="margin:0 0 10px;">' + (roadCount ? plural(roadCount, 'road connection') + ' from this tile.' : 'No roads.') +
      (roadCount ? ' <button type="button" class="btn small ghost" id="t_clearRoads">Remove roads here</button>' : ' Draw them with the Road tool.') + '</p>';
    if (roadBlocker(k) && roadCount) html += warn('A road crosses this tile without a bridge.');

    html += '<div class="insp-row"><label>Build capacity</label><div class="two" style="grid-template-columns:repeat(4, minmax(0, 1fr));">' +
      fieldRow('Stability', numInput('t_stab', t.stability, 'def ' + (terr.default_stability || 0))) +
      fieldRow('Max height', numInput('t_mh', t.max_height, 'def ' + (terr.default_max_height || 0))) +
      fieldRow('Width', numInput('t_mw', t.max_width, 'def ' + (terr.default_max_width || 0))) +
      fieldRow('Dig depth', numInput('t_md', t.max_depth, 'def ' + (terr.default_max_depth || 0))) + '</div></div>';
    html += '<p class="hint" style="margin-top:-4px;">Stability is the total segment budget; height and width cap the shape. Blank = the ' + esc(terr.label || 'terrain') + ' default. Prototype-only.</p>';

    var through = state.channels.filter(function (ch) { return ch.tiles.indexOf(k) !== -1; });
    if (through.length) {
      html += '<div class="insp-section-label">Channels</div>';
      through.forEach(function (ch) {
        var i = state.channels.indexOf(ch);
        var liquids = '<option value="">(dry)</option>' + flowingMaterials().map(function (m) { return '<option value="' + esc(m.id) + '"' + (m.id === ch.liquid ? ' selected' : '') + '>' + esc(m.label) + '</option>'; }).join('');
        html += '<div class="garrison-unit-card" style="margin:0 0 8px;"><div class="gu-row"><label>' + (ch.tiles.length === 1 ? 'Pond (one tile)' : 'Along ' + plural(ch.tiles.length, 'tile')) + (ch.depth < 0 ? ', a bank' : '') + '</label></div>' +
          '<div class="band-row"><span>width</span><input type="number" min="1" data-channel="' + i + '" data-key="width" value="' + esc(ch.width) + '" aria-label="Width in cells" />' +
          '<span>depth</span><input type="number" data-channel="' + i + '" data-key="depth" value="' + esc(ch.depth) + '" aria-label="Depth in cells (negative raises a bank)" /></div>' +
          '<div class="band-row"><select data-channel="' + i + '" data-key="liquid" aria-label="Liquid">' + liquids + '</select>' +
          (ch.liquid ? '<input type="number" min="0" data-channel="' + i + '" data-key="liquid_depth" value="' + esc(ch.liquid_depth) + '" aria-label="Liquid depth in cells" /><span>deep</span>' : '') +
          '<button type="button" class="btn small danger" data-channel-remove="' + i + '">Remove</button></div>' +
          (ch.liquid && ch.liquid_depth > ch.depth ? warn('The liquid is deeper than the channel.') : '') + '</div>';
      });
      html += '<p class="hint" style="margin-top:-2px;">Cells across and down; a negative depth raises a bank or mound (Decision 59).</p>';
    }
    html += '<div class="insp-section-label">Upgrades</div>';
    var ups = t.upgrades || [];
    var slots = t.upgrade_slots || 0;
    html += fieldRow('Upgrade slots', numInput('t_uslots', slots));
    html += '<div class="upgrade-list" id="upgradeList"></div>';
    html += '<div class="patrol-add">' + selectInput('t_upAdd', UPGRADE_LIBRARY.map(function (u) { return u.id; }), '', false, upgradeLabel) +
      '<button class="btn small" type="button" id="t_upAddBtn">Add upgrade</button></div>';
    if (ups.length > slots) html += warn(ups.length + ' upgrades but only ' + plural(slots, 'slot') + ' on this tile.');
    return html;
  }

  function bindTileSection(k) {
    function refresh() { pruneTile(k); safeSave(); renderInspector(); renderGrid(); renderCounts(); if (state.bottomTab === 'structure') renderStructureEditor(); }
    document.getElementById('t_terrain').addEventListener('change', function () {
      var nd = terrainDef(this.value || state.defaultTerrain);
      if (nd && nd.needs_bridge && !tileAt(k).bridge && hasRoadAt(k)) { roadUnderWaterNotice(nd.label); renderInspector(); return; }
      var t = ensureTile(k);
      if (this.value) t.terrain = this.value; else delete t.terrain;
      refresh();
    });
    document.getElementById('t_feature').addEventListener('change', function () {
      var t = ensureTile(k);
      if (this.value) t.feature = this.value; else delete t.feature;
      refresh();
    });
    var bAdd = document.getElementById('t_bAdd');
    if (bAdd) bAdd.addEventListener('click', function () { ensureTile(k).bridge = { owning_faction_id: '', hp: 40, demolishable_by_owner: false }; refresh(); });
    var bRm = document.getElementById('t_bRemove');
    if (bRm) bRm.addEventListener('click', function () { delete ensureTile(k).bridge; removeRoadsAt(k); refresh(); });
    var bOwner = document.getElementById('t_bOwner');
    if (bOwner) bOwner.addEventListener('change', function () { tileAt(k).bridge.owning_faction_id = this.value; safeSave(); });
    var bHp = document.getElementById('t_bHp');
    if (bHp) bHp.addEventListener('change', function () { tileAt(k).bridge.hp = Math.max(0, Number(this.value) || 0); safeSave(); });
    var bDraw = document.getElementById('t_bDraw');
    if (bDraw) bDraw.addEventListener('change', function () { tileAt(k).bridge.drawbridge_node_id = this.value; if (!this.value) tileAt(k).bridge.raised = false; refresh(); renderLinks(); });
    var bRaised = document.getElementById('t_bRaised');
    if (bRaised) bRaised.addEventListener('change', function () { tileAt(k).bridge.raised = this.checked; refresh(); renderLinks(); });
    var bDemo = document.getElementById('t_bDemo');
    if (bDemo) bDemo.addEventListener('change', function () { tileAt(k).bridge.demolishable_by_owner = this.checked; safeSave(); });
    var clr = document.getElementById('t_clearRoads');
    if (clr) clr.addEventListener('click', function () { removeRoadsAt(k); refresh(); });
    [['t_elev', 'elevation'], ['t_stab', 'stability'], ['t_mh', 'max_height'], ['t_mw', 'max_width'], ['t_md', 'max_depth']].forEach(function (pair) {
      document.getElementById(pair[0]).addEventListener('change', function () {
        var t = ensureTile(k);
        if (this.value === '') delete t[pair[1]]; else t[pair[1]] = Math.max(0, Number(this.value) || 0);
        refresh();
      });
    });
    document.querySelectorAll('[data-channel]').forEach(function (el) {
      el.addEventListener('change', function () {
        var ch = state.channels[Number(el.getAttribute('data-channel'))], field = el.getAttribute('data-key');
        if (field === 'liquid') { ch.liquid = el.value; if (ch.liquid && !ch.liquid_depth) ch.liquid_depth = 1; }
        else ch[field] = field === 'width' ? Math.max(1, Number(el.value) || 1) : Number(el.value) || 0;
        refresh();
      });
    });
    document.querySelectorAll('[data-channel-remove]').forEach(function (el) {
      el.addEventListener('click', function () { state.channels.splice(Number(el.getAttribute('data-channel-remove')), 1); refresh(); });
    });
    document.getElementById('t_uslots').addEventListener('change', function () {
      ensureTile(k).upgrade_slots = Math.max(0, Number(this.value) || 0);
      refresh();
    });
    document.getElementById('t_upAddBtn').addEventListener('click', function () {
      var t = ensureTile(k);
      if (!t.upgrades) t.upgrades = [];
      t.upgrades.push(document.getElementById('t_upAdd').value);
      refresh();
    });
    var list = document.getElementById('upgradeList');
    var ups = tileAt(k).upgrades || [];
    if (!ups.length) list.innerHTML = '<p class="empty-note" style="margin:0;font-size:11px;">no upgrades</p>';
    ups.forEach(function (u, i) {
      var row = document.createElement('div');
      row.className = 'patrol-row';
      row.innerHTML = '<span>' + esc(upgradeLabel(u)) + '</span>';
      var rm = document.createElement('button');
      rm.type = 'button'; rm.className = 'row-remove'; rm.textContent = '×'; rm.setAttribute('aria-label', 'Remove upgrade');
      rm.addEventListener('click', function () { ups.splice(i, 1); refresh(); });
      row.appendChild(rm);
      list.appendChild(row);
    });
  }

  // The inspector lists its sections (Ground layers, Upgrades, Node, Links, Structure,
  // Resource, Garrison, Reward) as tabs, one visible at a time; see tabifySections.
  function renderInspector() {
    renderInspectorContent();
    var cell = state.selectedCell ? nodeAt(state.selectedCell) : null;
    tabifySections(document.getElementById('inspectorBody'), 'inspector', cell ? 'Node' : 'Ground layers');
  }

  // Splits a panel's flat run of .insp-section-label sections into tabs. Elements are moved,
  // not re-created, so listeners bound during rendering keep working. The last tab chosen
  // per panel is remembered (falling back to `preferred`, then the first); a tab whose
  // hidden section holds a warning is flagged so problems aren't missed.
  var sectionTabs = {};
  function tabifySections(container, panelKey, preferred) {
    var children = Array.prototype.slice.call(container.children);
    var sections = [], current = null;
    children.forEach(function (el) {
      if (el.classList.contains('insp-section-label')) {
        current = document.createElement('div');
        current.className = 'insp-sec';
        current.setAttribute('data-sec', el.textContent.trim());
        container.insertBefore(current, el);
        el.remove();
        sections.push(current);
      } else if (current) {
        current.appendChild(el);
      }
    });
    if (sections.length < 2) return;
    var names = sections.map(function (sec) { return sec.getAttribute('data-sec'); });
    var active = names.indexOf(sectionTabs[panelKey]) !== -1 ? sectionTabs[panelKey]
      : names.indexOf(preferred) !== -1 ? preferred : names[0];
    var bar = document.createElement('div');
    bar.className = 'insp-tabs';
    bar.setAttribute('role', 'tablist');
    function show(name) {
      sections.forEach(function (sec) { sec.hidden = sec.getAttribute('data-sec') !== name; });
      bar.querySelectorAll('.tab-btn').forEach(function (b) { b.setAttribute('data-active', b.getAttribute('data-sec') === name ? 'true' : 'false'); });
    }
    sections.forEach(function (sec, i) {
      var name = names[i];
      var btn = document.createElement('button');
      btn.type = 'button';
      btn.className = 'tab-btn';
      btn.setAttribute('role', 'tab');
      btn.setAttribute('data-sec', name);
      btn.textContent = name;
      if (sec.querySelector('.warn-inline, .warnings')) {
        btn.setAttribute('data-warn', 'true');
        btn.title = name + ' has a problem to look at';
      }
      btn.addEventListener('click', function () { sectionTabs[panelKey] = name; show(name); });
      bar.appendChild(btn);
    });
    container.insertBefore(bar, sections[0]);
    show(active);
  }

  function renderInspectorContent() {
    var body = document.getElementById('inspectorBody');
    var k = state.selectedCell;
    if (!k) {
      body.innerHTML = '<p class="empty-note">Select any cell to edit its ground layers (terrain, feature, bridge, capacity, upgrades) and, if there is one, its node.</p>';
      return;
    }
    var cell = nodeAt(k);
    var html = '<div class="insp-head">' +
      (cell ? '<div class="glyph tile ' + cell.node_type + '">' + cell.node_type.charAt(0) + '</div><div class="kind">Node <span class="mono" style="text-transform:none;">' + esc(cell.id) + '</span></div>'
            : '<div class="kind">Empty tile</div>') +
      '<span class="coord">' + esc(cellLabel(k)) + '</span></div>';
    html += tileSectionHtml(k);

    if (!cell) {
      html += '<div class="insp-section-label">Node</div>';
      html += '<p class="empty-note" style="margin:0 0 8px;">No node on this tile.</p>';
      html += '<button class="btn small" type="button" id="placeNodeHere">Place node here</button>';
      body.innerHTML = html;
      bindTileSection(k);
      document.getElementById('placeNodeHere').addEventListener('click', function () { state.armedPreset = null; applyTool(k, 'NODE'); });
      return;
    }

    html += '<div class="insp-section-label">Node</div>';
    html += fieldRow('Id', '<input type="text" id="f_id" value="' + esc(cell.id) + '" />');
    html += fieldRow('Node type', selectInput('f_nodeType', NODE_TYPES, cell.node_type, false));
    html += checkboxRow('f_critical', 'Critical asset (see Loss criteria in World)', !!cell.is_critical_asset);
    html += fieldRow('Owning faction', factionSelect('f_owningFaction', cell.owning_faction_id));
    html += '<div class="insp-row"><label>Hidden from (secret objective)</label><div class="chip-list">' + activeFactions().map(function (f) {
      return '<label class="checkbox-row" style="text-transform:none;letter-spacing:0;font-size:12px;font-weight:500;"><input type="checkbox" data-hidefrom="' + esc(f.id) + '"' +
        ((cell.hidden_from || []).indexOf(f.id) !== -1 ? ' checked' : '') + ' /> ' + esc(f.display_name) + '</label>';
    }).join('') + '</div><p class="hint" style="margin:2px 0 0;">Those factions don\'t know this node exists at map start. How it gets revealed (scouting, events) is not designed yet. Preview with "View as" in the toolbar.</p></div>';

    var isWp = cell.node_type === 'WAYPOINT';
    if (isWp) {
      html += '<p class="hint">Routing waypoint: a link’s route is forced through this cell. It holds no structure, resources or garrison.</p>';
    }

    // Links (Decision 26: the authored routes). Unlinking here is the direct way to drop one;
    // editing terrain only reroutes a link, it never removes it.
    var myLinks = state.links.map(function (l, i) { return { l: l, i: i }; }).filter(function (x) { return x.l.a === cell.id || x.l.b === cell.id; });
    html += '<div class="insp-section-label">Links</div>';
    html += myLinks.length
      ? '<div class="link-list">' + myLinks.map(function (x) {
        var other = x.l.a === cell.id ? x.l.b : x.l.a;
        return '<div class="link-row"><span>&harr; ' + esc(other) + '</span><button class="row-remove" type="button" data-unlink="' + x.i + '" aria-label="Unlink ' + esc(other) + '">&times;</button></div>';
      }).join('') + '</div>'
      : '<p class="empty-note" style="margin:0 0 6px;font-size:12px;">not linked</p>';

    // Structure (prototype-only segment profile)
    var s = structureStats(cell), cap = capacity(k);
    if (!isWp) {
    html += '<div class="insp-section-label">Structure</div>';
    html += '<p class="empty-note" style="margin:0 0 6px;font-size:12px;"><b>' + esc(archetypeLabel(cell.structure.archetype)) + '</b> · ' +
      (s.segments ? plural(s.segments, 'segment') + ', ' + s.height + ' high, ' + s.width + ' wide' : 'nothing built') +
      '<br><span style="font-size:11px;">Tile allows ' + cap.stability + ' segments, ' + cap.max_height + ' high, ' + cap.max_width + ' wide.</span></p>';
    structureProblems(k, cell).forEach(function (p) { html += warn(p + '.'); });
    html += '<button type="button" class="btn small" id="f_editStructure">Edit structure below the map</button>';
    }

    if (cell.node_type === 'RESOURCE') {
      html += '<div class="insp-section-label">Resource</div>';
      html += '<div class="insp-row"><div class="two">' +
        fieldRow('Yield/tick', numInput('f_yield', cell.fields.yield_food_per_tick)) +
        fieldRow('Ravage lump', numInput('f_ravage', cell.fields.ravage_yield_food)) + '</div></div>';
      html += fieldRow('Resource type', selectInput('f_resourceType', RESOURCE_TYPES, cell.fields.resource_type || 'FOOD', false));
      html += checkboxRow('f_inexhaustible', 'Inexhaustible', cell.fields.is_inexhaustible !== false);
      if (cell.fields.is_inexhaustible === false) {
        html += fieldRow('Total reserves', numInput('f_reserves', cell.fields.total_reserves));
        html += '<p class="hint" style="margin-top:-4px;">A limited stockpile. Intended rule (not wired up yet): the decay-floor amount keeps flowing forever; only harvest above that baseline draws the stockpile down.</p>';
      } else {
        html += '<p class="hint" style="margin-top:-4px;">Never runs out. Untick to give it a limited stockpile instead.</p>';
      }
    }

    if (!isWp) {
    html += '<div class="insp-section-label">Garrison</div>';
    html += '<div class="insp-row"><div class="two">' +
      fieldRow('Garrison HP', numInput('f_ghp', cell.fields.garrison_hp)) +
      fieldRow('Garrison Dmg', numInput('f_gdmg', cell.fields.garrison_dmg)) + '</div></div>';
    html += '<div id="garrisonUnitsHost"></div>';
    html += '<div class="drop-hint" id="garrisonDrop">drop a unit chip here to add an assignment</div>';

    html += '<div class="insp-section-label">Reward</div>';
    html += fieldRow('Capture reward', '<input type="text" id="f_reward" value="' + esc(cell.fields.capture_reward || '') + '" placeholder="e.g. unlock:scout_unit" />');
    }

    body.innerHTML = html;
    bindTileSection(k);

    var editBtn = document.getElementById('f_editStructure');
    if (editBtn) editBtn.addEventListener('click', function () { setBottomTab('structure'); });
    body.querySelectorAll('[data-unlink]').forEach(function (b) {
      b.addEventListener('click', function () {
        state.links.splice(Number(b.getAttribute('data-unlink')), 1);
        linksChanged();
      });
    });
    document.getElementById('f_id').addEventListener('change', function () {
      var oldId = cell.id, newId = this.value.trim();
      if (!newId || newId === oldId) { this.value = oldId; return; }
      if (findNodeKeyById(newId)) { this.value = oldId; return; } // ids must stay unique
      cell.id = newId;
      state.links.forEach(function (l) { if (l.a === oldId) l.a = newId; if (l.b === oldId) l.b = newId; });
      state.lossCriteria.forEach(function (g) { g.node_ids = g.node_ids.map(function (x) { return x === oldId ? newId : x; }); });
      Object.keys(state.cells).forEach(function (kk) {
        state.cells[kk].garrison_units.forEach(function (u) {
          u.patrol_route = u.patrol_route.map(function (x) { return x === oldId ? newId : x; });
          if (u.delivery_target_id === oldId) u.delivery_target_id = newId;
        });
      });
      safeSave(); renderInspector(); renderGrid(); renderLinks(); renderLoss(); setBottomTab(state.bottomTab);
    });
    document.getElementById('f_nodeType').addEventListener('change', function () {
      cell.node_type = this.value;
      if (cell.node_type === 'WAYPOINT' && state.bottomTab === 'structure') state.bottomTab = 'paint';
      safeSave(); renderInspector(); renderGrid(); renderCounts(); setBottomTab(state.bottomTab);
    });
    document.getElementById('f_critical').addEventListener('change', function () { cell.is_critical_asset = this.checked; safeSave(); renderGrid(); renderLoss(); });
    document.getElementById('f_owningFaction').addEventListener('change', function () { cell.owning_faction_id = this.value; safeSave(); renderGrid(); renderLoss(); });
    body.querySelectorAll('[data-hidefrom]').forEach(function (cb) {
      cb.addEventListener('change', function () {
        var fid = cb.getAttribute('data-hidefrom');
        cell.hidden_from = (cell.hidden_from || []).filter(function (x) { return x !== fid; });
        if (cb.checked) cell.hidden_from.push(fid);
        safeSave(); renderGrid();
      });
    });

    bindNum('f_yield', cell.fields, 'yield_food_per_tick');
    bindNum('f_ravage', cell.fields, 'ravage_yield_food');
    bindSelect('f_resourceType', cell.fields, 'resource_type');
    var inexInput = document.getElementById('f_inexhaustible');
    if (inexInput) inexInput.addEventListener('change', function () { cell.fields.is_inexhaustible = this.checked; safeSave(); renderInspector(); });
    bindNum('f_reserves', cell.fields, 'total_reserves');
    bindNum('f_ghp', cell.fields, 'garrison_hp');
    bindNum('f_gdmg', cell.fields, 'garrison_dmg');
    bindText('f_reward', cell.fields, 'capture_reward');

    renderGarrisonUnits(cell);
    var dz = document.getElementById('garrisonDrop');
    if (!dz) return;
    dz.addEventListener('dragover', function (ev) { ev.preventDefault(); this.setAttribute('data-over', 'true'); });
    dz.addEventListener('dragleave', function () { this.removeAttribute('data-over'); });
    dz.addEventListener('drop', function (ev) {
      ev.preventDefault(); this.removeAttribute('data-over');
      var unit = ev.dataTransfer.getData('text/unit');
      if (unit) addGarrisonUnit(cell, unit);
    });
  }

  function bindNum(id, obj, prop, after) {
    var el = document.getElementById(id);
    if (el) el.addEventListener('change', function () { obj[prop] = Math.max(0, Number(this.value) || 0); safeSave(); if (after) after(); });
  }
  function bindSelect(id, obj, prop) { var el = document.getElementById(id); if (el) el.addEventListener('change', function () { obj[prop] = this.value; safeSave(); }); }
  function bindText(id, obj, prop) { var el = document.getElementById(id); if (el) el.addEventListener('change', function () { obj[prop] = this.value; safeSave(); }); }

  function renderGarrisonUnits(cell) {
    var host = document.getElementById('garrisonUnitsHost');
    if (!host) return;
    host.innerHTML = '';
    if (cell.garrison_units.length === 0) {
      host.innerHTML = '<p class="empty-note" style="margin:0 0 6px;">no units assigned</p>';
      return;
    }
    cell.garrison_units.forEach(function (unit, i) {
      var card = document.createElement('div');
      card.className = 'garrison-unit-card';
      var uid = 'gu' + i;
      card.innerHTML =
        '<div class="gu-head"><span class="name">' + esc(unit._display_label) + ' #' + (i + 1) + '</span>' +
        '<button class="row-remove" type="button" data-role="remove" aria-label="Remove unit">&times;</button></div>' +
        '<div class="gu-row"><label>Faction</label>' + factionSelect(uid + '_faction', unit.faction_id) + '</div>' +
        '<label class="checkbox-row" style="margin-bottom:6px;"><input type="checkbox" id="' + uid + '_sortie"' + (unit.can_sortie ? ' checked' : '') + ' /> Can sortie</label>' +
        '<div class="gu-row"><label>Delivery target</label>' + selectInput(uid + '_delivery', otherNodeIds(cell.id), unit.delivery_target_id, true) + '</div>' +
        '<div class="gu-row"><label>Patrol route</label><div class="patrol-list" id="' + uid + '_patrolList"></div>' +
        '<div class="patrol-add">' + selectInput(uid + '_patrolAdd', otherNodeIds(cell.id), '', true) +
        '<button class="btn small" type="button" id="' + uid + '_patrolAddBtn">Add stop</button></div></div>';
      host.appendChild(card);

      card.querySelector('[data-role="remove"]').addEventListener('click', function () {
        cell.garrison_units.splice(i, 1); safeSave(); renderInspector(); renderCounts();
      });
      document.getElementById(uid + '_faction').addEventListener('change', function () { unit.faction_id = this.value; safeSave(); });
      document.getElementById(uid + '_sortie').addEventListener('change', function () { unit.can_sortie = this.checked; safeSave(); });
      document.getElementById(uid + '_delivery').addEventListener('change', function () { unit.delivery_target_id = this.value; safeSave(); });
      renderPatrolList(unit, uid);
      document.getElementById(uid + '_patrolAddBtn').addEventListener('click', function () {
        var sel = document.getElementById(uid + '_patrolAdd');
        if (sel.value) { unit.patrol_route.push(sel.value); safeSave(); renderInspector(); }
      });
    });
  }

  function renderPatrolList(unit, uid) {
    var list = document.getElementById(uid + '_patrolList');
    if (!list) return;
    list.innerHTML = '';
    var route = unit.patrol_route || [];
    if (route.length === 0) { list.innerHTML = '<p class="empty-note" style="margin:0;font-size:10.5px;">static</p>'; return; }
    route.forEach(function (stopId, i) {
      var row = document.createElement('div');
      row.className = 'patrol-row';
      row.innerHTML = '<span>' + (i + 1) + '. ' + esc(stopId) + '</span>';
      var controls = document.createElement('span'); controls.className = 'controls';
      if (i > 0) {
        var up = document.createElement('button'); up.type = 'button'; up.textContent = '↑';
        up.addEventListener('click', function () { var t = route[i - 1]; route[i - 1] = route[i]; route[i] = t; safeSave(); renderInspector(); });
        controls.appendChild(up);
      }
      if (i < route.length - 1) {
        var down = document.createElement('button'); down.type = 'button'; down.textContent = '↓';
        down.addEventListener('click', function () { var t = route[i + 1]; route[i + 1] = route[i]; route[i] = t; safeSave(); renderInspector(); });
        controls.appendChild(down);
      }
      var rm = document.createElement('button'); rm.type = 'button'; rm.className = 'remove'; rm.textContent = '×';
      rm.addEventListener('click', function () { route.splice(i, 1); safeSave(); renderInspector(); });
      controls.appendChild(rm);
      row.appendChild(controls);
      list.appendChild(row);
    });
  }

  // ---------- counts ----------
  function renderCounts() {
    var counts = { ORIGIN: 0, RESOURCE: 0, FORT: 0, NEUTRAL: 0, WAYPOINT: 0 };
    Object.keys(state.cells).forEach(function (k) {
      var d = state.cells[k];
      counts[d.node_type]++;
    });
    var terrainCount = 0, featureCount = 0, bridgeCount = 0;
    Object.keys(state.tiles).forEach(function (k) {
      var t = state.tiles[k];
      if (t.terrain) terrainCount++;
      if (t.feature) featureCount++;
      if (t.bridge) bridgeCount++;
    });
    var typeColor = { ORIGIN: 'var(--accent)', RESOURCE: 'var(--good)', FORT: 'var(--structure)', NEUTRAL: 'var(--ink-dim)', WAYPOINT: 'var(--ink-dim)' };
    var html = NODE_TYPES.map(function (t) {
      return '<span class="count-chip"><span class="swatch" style="background:' + typeColor[t] + '"></span>' + t + ' &times; ' + counts[t] + '</span>';
    }).join('');
    html += '<span class="count-chip">Terrain &times; ' + terrainCount + '</span>';
    html += '<span class="count-chip">Features &times; ' + featureCount + '</span>';
    html += '<span class="count-chip">Road segments &times; ' + state.roads.length + '</span>';
    html += '<span class="count-chip">Bridges &times; ' + bridgeCount + '</span>';
    html += '<span class="count-chip">Links &times; ' + state.links.length + '</span>';
    document.getElementById('countChips').innerHTML = html;
  }

  // ---------- Factions view ----------
  var selectedFactionId = null;
  var pendingRemoveFaction = null;

  function renderFactionsView() {
    if (!selectedFactionId || !libraryFactions.some(function (f) { return f.id === selectedFactionId; })) {
      selectedFactionId = libraryFactions.length ? libraryFactions[0].id : null;
    }
    var list = document.getElementById('factionLibList');
    list.innerHTML = '';
    libraryFactions.forEach(function (f) {
      var el = document.createElement('div');
      el.className = 'faction-row lib-row';
      el.style.flexDirection = 'row';
      el.tabIndex = 0;
      el.setAttribute('data-selected', f.id === selectedFactionId ? 'true' : 'false');
      var onMap = state.mapFactionIds.indexOf(f.id) !== -1;
      el.innerHTML = '<span style="display:flex;align-items:center;gap:6px;font-weight:600;"><span class="swatch" style="background:' + factionColor(f.id) + '"></span>' +
        esc(f.display_name) + ' <span class="mono" style="color:var(--ink-dim);font-size:10px;">' + esc(f.id) + '</span></span>' +
        '<span class="mono" style="font-size:10px;color:var(--ink-dim);">' + (onMap ? 'on this map' : '') + '</span>';
      var pick = function () { selectedFactionId = f.id; pendingRemoveFaction = null; renderFactionsView(); };
      el.addEventListener('click', pick);
      el.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') pick(); });
      list.appendChild(el);
    });

    var detail = document.getElementById('factionDetail');
    var f = libraryFactions.find(function (x) { return x.id === selectedFactionId; });
    if (!f) { detail.innerHTML = '<p class="empty-note">Add a faction to the library to start.</p>'; return; }
    var ownedNodes = Object.keys(state.cells).filter(function (k) { return state.cells[k].owning_faction_id === f.id; }).length;
    var groups = state.lossCriteria.filter(function (g) { return g.faction_id === f.id; }).length;
    var others = libraryFactions.filter(function (x) { return x.id !== f.id; });
    var relRows = others.length ? others.map(function (o, i) {
      var cur = defaultStance(f.id, o.id);
      return '<div class="relation-row"><span style="display:flex;align-items:center;gap:6px;"><span class="swatch" style="background:' + factionColor(o.id) + '"></span>' + esc(o.display_name) + '</span>' +
        '<select id="dr_' + i + '" data-other="' + esc(o.id) + '" aria-label="Default stance toward ' + esc(o.display_name) + '"><option value="">(no default)</option>' +
        STANCES.map(function (st) { return '<option value="' + st + '"' + (st === cur ? ' selected' : '') + '>' + st.charAt(0) + st.slice(1).toLowerCase() + '</option>'; }).join('') +
        '</select></div>';
    }).join('') : '<p class="empty-note" style="margin:0;">Add another faction to the library to set default relations.</p>';

    detail.innerHTML =
      '<div class="detail-title"><span class="swatch" style="width:16px;height:16px;background:' + factionColor(f.id) + '"></span><h2>' + esc(f.display_name) + '</h2><span class="mono" style="color:var(--ink-dim);">' + esc(f.id) + '</span></div>' +
      '<p class="usage">On the current map: ' + (state.mapFactionIds.indexOf(f.id) !== -1 ? 'in play' : 'not in play') + ' · owns ' + plural(ownedNodes, 'node') + ' · ' + plural(groups, 'loss group') + '.</p>' +
      '<div class="form-grid">' +
      '<div class="insp-row full"><label for="fp_name">Display name</label><input type="text" id="fp_name" value="' + esc(f.display_name) + '" /></div>' +
      '<div class="insp-row"><label for="fp_ramp">Default suspicion ramp (stub)</label><input type="text" id="fp_ramp" value="' + esc(f.stub_suspicion_ramp || '') + '" placeholder="e.g. 3/tick" /></div>' +
      '<div class="insp-row"><label for="fp_units">Available unit types (stub)</label><input type="text" id="fp_units" value="' + esc(f.stub_unit_types || '') + '" placeholder="e.g. Militia, Guard, Knight" /></div>' +
      '<div class="insp-row full"><label for="fp_strategy">Strategy notes (stub)</label><input type="text" id="fp_strategy" value="' + esc(f.stub_strategy_notes || '') + '" placeholder="e.g. turtles until Full Alert" /></div>' +
      '</div>' +
      '<p class="hint">Stub fields aren\'t used anywhere yet. They sketch a future FactionDef profile: default suspicion ramp, unit roster and strategy.</p>' +
      '<div class="insp-section-label">Default relations</div>' +
      '<div class="relation-list" style="max-width:520px;">' + relRows + '</div>' +
      '<p class="hint">Filled in automatically when both factions are on a map. Each map can still change them in Lanes &rarr; World. Prototype-only: real relations are per map.</p>' +
      '<div class="danger-zone">' +
      (f.id === 'player' ? '<span class="hint" style="margin:0;">Player is a permanent library entry.</span>'
        : '<button type="button" class="btn small danger" id="fp_remove">' + (pendingRemoveFaction === f.id ? 'Click again to remove from library' : 'Remove from library') + '</button>') +
      '</div>';
    [['fp_ramp', 'stub_suspicion_ramp'], ['fp_units', 'stub_unit_types'], ['fp_strategy', 'stub_strategy_notes']].forEach(function (p) {
      document.getElementById(p[0]).addEventListener('input', function () { f[p[1]] = this.value; saveLibrary(); });
    });
    document.getElementById('fp_name').addEventListener('change', function () {
      f.display_name = this.value.trim() || f.display_name; saveLibrary(); renderFactionsView();
    });
    detail.querySelectorAll('select[id^="dr_"]').forEach(function (sel) {
      sel.addEventListener('change', function () {
        var other = sel.getAttribute('data-other');
        defaultRelations = defaultRelations.filter(function (r) { return !pairMatch(r, f.id, other); });
        if (sel.value) defaultRelations.push({ a: f.id, b: other, stance: sel.value });
        saveDefaultRelations();
      });
    });
    var rm = document.getElementById('fp_remove');
    if (rm) rm.addEventListener('click', function () {
      if (pendingRemoveFaction !== f.id) { pendingRemoveFaction = f.id; renderFactionsView(); return; }
      libraryFactions = libraryFactions.filter(function (x) { return x.id !== f.id; });
      defaultRelations = defaultRelations.filter(function (r) { return r.a !== f.id && r.b !== f.id; });
      state.mapFactionIds = state.mapFactionIds.filter(function (id) { return id !== f.id; });
      state.faction_relations = state.faction_relations.filter(function (r) { return r.a !== f.id && r.b !== f.id; });
      state.lossCriteria = state.lossCriteria.filter(function (g) { return g.faction_id !== f.id; });
      dropHiddenFrom(f.id);
      pendingRemoveFaction = null; selectedFactionId = null;
      saveLibrary(); saveDefaultRelations(); safeSave(); renderFactionsView();
    });
  }

  document.getElementById('addFactionBtn').addEventListener('click', function () {
    var idInput = document.getElementById('newFactionId');
    var nameInput = document.getElementById('newFactionName');
    var id = idInput.value.trim();
    if (!id || libraryFactions.some(function (f) { return f.id === id; })) return;
    // Library only - putting it on a map is done from Lanes -> World.
    libraryFactions.push({ id: id, display_name: nameInput.value.trim() || id });
    idInput.value = ''; nameInput.value = '';
    selectedFactionId = id;
    saveLibrary(); renderFactionsView();
  });

  // ---------- Terrain view ----------
  var selectedTerrainItem = { kind: 'terrain', id: 'FIELDS' };
  var terrainNotice = '';

  function terrainUsage(id) {
    var n = Object.keys(state.tiles).filter(function (k) { return state.tiles[k].terrain === id; }).length;
    return { tiles: n, isBase: state.defaultTerrain === id };
  }
  function featureUsage(id) { return Object.keys(state.tiles).filter(function (k) { return state.tiles[k].feature === id; }).length; }

  function renderTerrainView() {
    document.getElementById('roadMult').value = terrainLib.road_move_multiplier;
    function libList(hostId, items, kind) {
      var host = document.getElementById(hostId);
      host.innerHTML = '';
      items.forEach(function (it) {
        var el = document.createElement('div');
        el.className = 'terrain-item lib-row';
        el.tabIndex = 0;
        el.setAttribute('data-selected', selectedTerrainItem.kind === kind && selectedTerrainItem.id === it.id ? 'true' : 'false');
        var bg = kind === 'feature' ? 'var(--ink-dim)' : esc(it.color);
        el.innerHTML = '<span style="display:flex;align-items:center;gap:6px;" class="name"><span class="glyph-dot" style="background:' + bg + '">' + esc(it.glyph) + '</span>' + esc(it.label) + '</span>' +
          '<span class="mono" style="font-size:10px;color:var(--ink-dim);">' + (kind === 'terrain'
            ? 'move ×' + esc(it.move_cost) + (it.can_be_base ? ' · base' : '') + (it.needs_bridge ? ' · bridge' : '')
            : kind === 'material' ? esc(traitText(it.traits) || 'no traits') + (it.heat !== 1 ? ' · heat ' + esc(it.heat) : '')
            : esc(it.id)) + '</span>';
        var pick = function () { selectedTerrainItem = { kind: kind, id: it.id }; terrainNotice = ''; renderTerrainView(); };
        el.addEventListener('click', pick);
        el.addEventListener('keydown', function (ev) { if (ev.key === 'Enter') pick(); });
        host.appendChild(el);
      });
    }
    libList('terrainLibList', terrainLib.terrains, 'terrain');
    libList('featureLibList', terrainLib.features, 'feature');
    libList('materialLibList', terrainLib.materials, 'material');

    var detail = document.getElementById('terrainDetail');
    var isTerrain = selectedTerrainItem.kind === 'terrain';
    var isMaterial = selectedTerrainItem.kind === 'material';
    var item = isTerrain ? terrainDef(selectedTerrainItem.id) : isMaterial ? materialDef(selectedTerrainItem.id) : featureDef(selectedTerrainItem.id);
    if (!item) { detail.innerHTML = '<p class="empty-note">Pick a terrain type or feature to edit it.</p>'; return; }

    var html = '<div class="detail-title"><span class="glyph-dot" style="width:26px;height:26px;font-size:15px;background:' + (isTerrain || isMaterial ? esc(item.color) : 'var(--ink-dim)') + '">' + esc(item.glyph || '') + '</span><h2>' + esc(item.label) + '</h2><span class="mono" style="color:var(--ink-dim);">' + esc(item.id) + '</span></div>';
    if (isTerrain) {
      var u = terrainUsage(item.id);
      html += '<p class="usage">On the current map: painted on ' + plural(u.tiles, 'tile') + (u.isBase ? ' · map base terrain' : '') + '.</p>';
      html += '<div class="form-grid">' +
        '<div class="insp-row"><label for="td_label">Label</label><input type="text" id="td_label" value="' + esc(item.label) + '" /></div>' +
        '<div class="insp-row"><label for="td_glyph">Glyph</label><input type="text" id="td_glyph" maxlength="2" value="' + esc(item.glyph) + '" /></div>' +
        '<div class="insp-row"><label for="td_color">Colour</label><input type="color" id="td_color" value="' + esc(item.color) + '" /></div>' +
        '<div class="insp-row"><label class="checkbox-row"><input type="checkbox" id="td_base"' + (item.can_be_base ? ' checked' : '') + ' /> Can be a map\'s base terrain</label>' +
        '<label class="checkbox-row"><input type="checkbox" id="td_bridge"' + (item.needs_bridge ? ' checked' : '') + ' /> Roads need a bridge to cross</label></div>' +
        '<div class="insp-row"><label for="td_move">Movement cost × (stub)</label><input type="number" step="0.5" min="0" id="td_move" value="' + esc(item.move_cost) + '" /></div>' +
        '<div class="insp-row"><label for="td_blocks">Blocks unit classes (stub)</label><input type="text" id="td_blocks" value="' + esc(item.blocks_unit_classes || '') + '" placeholder="e.g. siege, cavalry" /></div>' +
        '</div>' + groundFields(item) + '<div class="panel-sub">Side-on editor (legacy)</div><div class="form-grid">' +
        '<div class="insp-row"><label for="td_stab">Default stability (segment budget)</label><input type="number" min="0" id="td_stab" value="' + esc(item.default_stability) + '" /></div>' +
        '<div class="insp-row"><label for="td_mh">Default max height</label><input type="number" min="0" id="td_mh" value="' + esc(item.default_max_height) + '" /></div>' +
        '<div class="insp-row"><label for="td_mw">Default max width</label><input type="number" min="0" id="td_mw" value="' + esc(item.default_max_width) + '" /></div>' +
        '<div class="insp-row"><label for="td_md">Default dig depth (basements)</label><input type="number" min="0" id="td_md" value="' + esc(item.default_max_depth) + '" /></div>' +
        '</div>' +
        '<p class="hint">Stubs, not simulated. Movement cost and blocked unit classes sketch the terrain movement-resistance idea on file, where units path the least-resistant route and roads cut the cost. Stability, height and width are the side-on structure editor\'s capacity, kept until the plan editor (Decision 52) replaces it; each tile can override them in the Inspector.</p>';
    } else if (isMaterial) {
      var mu = materialUsage(item.id), lu = liquidUsage(item.id);
      html += '<p class="usage">' + ((item.traits || {}).flows ? 'A liquid (flows ' + esc(item.traits.flows) + '). ' : '') +
        'In the strata of ' + (mu.length ? mu.map(function (t) { return esc(t.label); }).join(', ') : 'no terrain') +
        (lu.length ? '; a liquid body under ' + lu.map(function (t) { return esc(t.label); }).join(', ') : '') + '.</p>';
      html += '<div class="form-grid">' +
        '<div class="insp-row"><label for="td_label">Label</label><input type="text" id="td_label" value="' + esc(item.label) + '" /></div>' +
        '<div class="insp-row"><label for="td_color">Colour</label><input type="color" id="td_color" value="' + esc(item.color) + '" /></div>' +
        '<div class="insp-row"><label for="md_strength">Strength (load an eighth carries)</label><input type="number" min="0" id="md_strength" value="' + esc(item.strength || 0) + '" /></div>' +
        '<div class="insp-row"><label for="md_temp">Heat (0-10)</label><input type="number" min="0" max="10" id="md_temp" value="' + esc(item.heat) + '" /></div>' +
        '<div class="insp-row"><label for="md_weight">Weight per cell</label><input type="number" min="0" id="md_weight" value="' + esc(item.weight) + '" /></div>' +
        '<div class="insp-row"><label for="md_span">Unsupported span (cells)</label><input type="number" min="0" id="md_span" value="' + esc(item.span) + '" /></div>' +
        '</div>' +
        '<p class="hint">Weight loads what is beneath; strength is the load each eighth of a cell carries (Decision 65); span is how far it bridges unsupported before it falls (Decision 57); heat is the level it gives off, 0-10 (1 ambient, 5 open fire, 8 lava; Decision 66).</p>' +
        traitsAndHeat(item);
    } else {
      var fu = featureUsage(item.id);
      html += '<p class="usage">On the current map: on ' + plural(fu, 'tile') + '.</p>';
      html += '<div class="form-grid">' +
        '<div class="insp-row"><label for="td_label">Label</label><input type="text" id="td_label" value="' + esc(item.label) + '" /></div>' +
        '<div class="insp-row"><label for="td_glyph">Glyph</label><input type="text" id="td_glyph" maxlength="2" value="' + esc(item.glyph) + '" /></div>' +
        '<div class="insp-row full"><label for="td_effect">What it offers (stub)</label><input type="text" id="td_effect" value="' + esc(item.stub_effect || '') + '" /></div>' +
        '</div>' +
        '<p class="hint">A natural feature sits on top of terrain and under any structure. It\'s what a mine, farm or quarry node would exploit.</p>';
    }
    html += '<div class="danger-zone"><button type="button" class="btn small danger" id="td_remove">Remove from library</button>' +
      (terrainNotice ? '<span class="warn-inline" style="margin:0;">' + esc(terrainNotice) + '</span>' : '') + '</div>';
    detail.innerHTML = html;

    function bindT(id, prop, numeric) {
      var el = document.getElementById(id);
      if (!el) return;
      el.addEventListener('change', function () {
        item[prop] = el.type === 'checkbox' ? el.checked : numeric ? Math.max(0, Number(el.value) || 0) : el.value;
        saveTerrainLib(); renderTerrainView();
      });
    }
    bindT('td_label', 'label'); bindT('td_glyph', 'glyph');
    if (isTerrain) {
      bindT('td_color', 'color'); bindT('td_base', 'can_be_base'); bindT('td_bridge', 'needs_bridge');
      bindT('td_move', 'move_cost', true); bindT('td_blocks', 'blocks_unit_classes');
      bindGround(item);
      bindT('td_stab', 'default_stability', true); bindT('td_mh', 'default_max_height', true); bindT('td_mw', 'default_max_width', true); bindT('td_md', 'default_max_depth', true); bindT('td_ml', 'default_max_length', true);
    } else if (isMaterial) {
      bindT('td_color', 'color'); bindT('md_weight', 'weight', true); bindT('md_span', 'span', true); bindT('md_strength', 'strength', true);
      document.getElementById('md_temp').addEventListener('change', function () { item.heat = Math.max(0, Math.min(10, Math.round(Number(this.value) || 0))); saveTerrainLib(); renderTerrainView(); });
      bindTraitsAndHeat(item);
    } else {
      bindT('td_effect', 'stub_effect');
    }
    document.getElementById('td_remove').addEventListener('click', function () {
      if (isTerrain) {
        var u2 = terrainUsage(item.id);
        if (u2.tiles || u2.isBase) { terrainNotice = 'In use on the current map. Repaint those tiles or change the base terrain first.'; renderTerrainView(); return; }
        if (item.can_be_base && baseTerrains().length <= 1) { terrainNotice = 'At least one terrain must be usable as a base.'; renderTerrainView(); return; }
        terrainLib.terrains = terrainLib.terrains.filter(function (t) { return t.id !== item.id; });
      } else if (isMaterial) {
        var users = materialUsage(item.id).concat(liquidUsage(item.id));
        if (users.length) { terrainNotice = 'In the ground of ' + users.map(function (t) { return t.label; }).join(', ') + '. Remove it there first.'; renderTerrainView(); return; }
        var changes = becomesUsage(item.id);
        if (changes.length) { terrainNotice = changes.map(function (x) { return x.label; }).join(', ') + ' turn into it with heat. Change those first.'; renderTerrainView(); return; }
        terrainLib.materials = terrainLib.materials.filter(function (m) { return m.id !== item.id; });
      } else {
        if (featureUsage(item.id)) { terrainNotice = 'In use on the current map. Erase it from those tiles first.'; renderTerrainView(); return; }
        terrainLib.features = terrainLib.features.filter(function (f) { return f.id !== item.id; });
      }
      terrainNotice = '';
      selectedTerrainItem = { kind: 'terrain', id: terrainLib.terrains[0].id };
      saveTerrainLib(); renderTerrainView();
    });
  }

  function addLibItem(inputId, kind) {
    var input = document.getElementById(inputId);
    var id = input.value.trim().toUpperCase().replace(/\s+/g, '_');
    if (!id) return;
    var exists = kind === 'terrain' ? terrainDef(id) : kind === 'material' ? materialDef(id) : featureDef(id);
    if (exists) { selectedTerrainItem = { kind: kind, id: id }; renderTerrainView(); return; }
    var label = id.charAt(0) + id.slice(1).toLowerCase().replace(/_/g, ' ');
    if (kind === 'terrain') {
      terrainLib.terrains.push(Object.assign({ id: id, label: label, glyph: '?', color: '#7d7466', can_be_base: false, needs_bridge: false, move_cost: 1, blocks_unit_classes: '', default_stability: 2, default_max_height: 1, default_max_width: 2, default_max_length: 2, default_max_depth: 1 }, clone(GENERIC_GROUND)));
    } else if (kind === 'material') {
      terrainLib.materials.push({ id: id, label: label, color: '#7d7466', weight: 1, span: 1, strength: 1, heat: 1, traits: { dig_difficulty: 1 }, heat_transitions: [] });
    } else {
      terrainLib.features.push({ id: id, label: label, glyph: '?', stub_effect: '' });
    }
    input.value = '';
    selectedTerrainItem = { kind: kind, id: id };
    saveTerrainLib(); renderTerrainView();
  }
  document.getElementById('addTerrainBtn').addEventListener('click', function () { addLibItem('newTerrainId', 'terrain'); });
  document.getElementById('addFeatureBtn').addEventListener('click', function () { addLibItem('newFeatureId', 'feature'); });
  document.getElementById('addMaterialBtn').addEventListener('click', function () { addLibItem('newMaterialId', 'material'); });

  // Traits and heat transitions on a material (Decisions 63, 64): a trait is a
  // name with a level; a transition is "above/below a heat level: becomes X / gains trait".
  function traitsAndHeat(item) {
    var traits = Object.keys(item.traits || {}).map(function (k, i) {
      return '<div class="band-row"><input type="text" style="flex:1;" data-trait="' + i + '" data-key="name" value="' + esc(k) + '" aria-label="Trait name" />' +
        '<span>level</span><input type="number" min="1" data-trait="' + i + '" data-key="level" value="' + esc(item.traits[k]) + '" aria-label="Trait level" />' +
        '<button type="button" class="btn small" data-trait-remove="' + i + '" aria-label="Remove trait">×</button></div>';
    }).join('');
    var targets = terrainLib.materials.filter(function (x) { return x.id !== item.id; });
    var heats = (item.heat_transitions || []).map(function (h, i) {
      var into = targets.map(function (x) { return '<option value="' + esc(x.id) + '"' + (h.becomes === x.id ? ' selected' : '') + '>becomes ' + esc(x.label) + '</option>'; }).join('');
      return '<div class="band-row"><select data-heat="' + i + '" data-key="dir" style="flex:0 0 auto;" aria-label="Above or below"><option value="above"' + (h.above != null ? ' selected' : '') + '>above heat</option><option value="below"' + (h.below != null ? ' selected' : '') + '>below heat</option></select>' +
        '<input type="number" data-heat="' + i + '" data-key="temp" value="' + esc(h.above != null ? h.above : h.below) + '" aria-label="Heat level" /><span>:</span>' +
        '<select data-heat="' + i + '" data-key="outcome" aria-label="Outcome"><option value="">gains trait…</option>' + into + '</select>' +
        (h.becomes ? '' : '<input type="text" style="width:96px;" data-heat="' + i + '" data-key="gains" value="' + esc(h.gains || '') + '" placeholder="burning" aria-label="Trait gained" />') +
        '<button type="button" class="btn small" data-heat-remove="' + i + '" aria-label="Remove transition">×</button></div>';
    }).join('');
    return '<div class="panel-sub">Traits</div>' + traits + '<button type="button" class="btn small" id="tr_add">Add trait</button>' +
      '<p class="hint">Each with a level, met in pairs (Decision 64): dig_difficulty against burrower, climb_difficulty against climber, glows N (light) against light_blind or darksight. flows N makes it a liquid; loose falls and settles.</p>' +
      '<div class="panel-sub">Heat</div>' + heats + '<button type="button" class="btn small" id="ht_add">Add heat transition</button>' +
      '<p class="hint">Heat levels 0-10: timber gains burning above 3; rock becomes lava above 7; lava becomes rock below 6.</p>';
  }
  function bindTraitsAndHeat(item) {
    var save = function () { saveTerrainLib(); renderTerrainView(); };
    var names = Object.keys(item.traits || {});
    document.querySelectorAll('#terrainDetail [data-trait]').forEach(function (el) {
      el.addEventListener('change', function () {
        var old = names[Number(el.getAttribute('data-trait'))], level = item.traits[old];
        if (el.getAttribute('data-key') === 'level') { item.traits[old] = Math.max(1, Number(el.value) || 1); }
        else { var renamed = el.value.trim(); delete item.traits[old]; if (renamed) item.traits[renamed] = level; }
        save();
      });
    });
    document.querySelectorAll('#terrainDetail [data-trait-remove]').forEach(function (el) {
      el.addEventListener('click', function () { delete item.traits[names[Number(el.getAttribute('data-trait-remove'))]]; save(); });
    });
    document.getElementById('tr_add').addEventListener('click', function () {
      item.traits = item.traits || {}; var n = 'trait'; var i = 2; while (item.traits[n]) n = 'trait' + i++;
      item.traits[n] = 1; save();
    });
    document.querySelectorAll('#terrainDetail [data-heat]').forEach(function (el) {
      el.addEventListener('change', function () {
        var h = item.heat_transitions[Number(el.getAttribute('data-heat'))], key = el.getAttribute('data-key');
        var temp = h.above != null ? h.above : h.below;
        if (key === 'dir') { delete h.above; delete h.below; h[el.value] = temp; }
        else if (key === 'temp') { h[h.above != null ? 'above' : 'below'] = Number(el.value) || 0; }
        else if (key === 'outcome') { delete h.becomes; delete h.gains; if (el.value) h.becomes = el.value; else h.gains = 'burning'; }
        else if (key === 'gains') { h.gains = el.value.trim(); }
        save();
      });
    });
    document.querySelectorAll('#terrainDetail [data-heat-remove]').forEach(function (el) {
      el.addEventListener('click', function () { item.heat_transitions.splice(Number(el.getAttribute('data-heat-remove')), 1); save(); });
    });
    document.getElementById('ht_add').addEventListener('click', function () {
      item.heat_transitions = item.heat_transitions || [];
      item.heat_transitions.push({ above: 3, gains: 'burning' }); save();
    });
  }

  // The ground section of the terrain inspector (Decisions 53, 54, 62): elevation, bearing,
  // foundations, dig depth, the strata bands from the surface down, and liquid bodies.
  function groundFields(t) {
    var bodies = (t.liquids || []).map(function (b, i) {
      var opts = flowingMaterials().map(function (l) { return '<option value="' + esc(l.id) + '"' + (l.id === b.material ? ' selected' : '') + '>' + esc(l.label) + '</option>'; }).join('');
      var pct = function (key, label) { return '<input type="number" min="0" max="100" style="width:56px;" data-body="' + i + '" data-key="' + key + '" value="' + Math.round((b[key] || 0) * 100) + '" aria-label="' + label + '" /><span>%</span>'; };
      return '<div class="band-row"><select data-body="' + i + '" data-key="material" aria-label="Liquid">' + opts + '</select>' +
        '<input type="number" min="0" style="width:56px;" data-body="' + i + '" data-key="min" value="' + esc(b.min) + '" aria-label="Least depth" /><span>to</span>' +
        '<input type="number" min="0" style="width:56px;" data-body="' + i + '" data-key="max" value="' + esc(b.max) + '" aria-label="Most depth" /><span>down,</span>' +
        pct('chance', 'Chance') + '<span>surface</span>' + pct('surface_chance', 'Surface chance') +
        '<button type="button" class="btn small" data-body-remove="' + i + '" aria-label="Remove liquid body">×</button></div>';
    }).join('');
    var options = function (selected) {
      return terrainLib.materials.map(function (m) { return '<option value="' + esc(m.id) + '"' + (m.id === selected ? ' selected' : '') + '>' + esc(m.label) + '</option>'; }).join('');
    };
    var rows = (t.strata || []).map(function (b, i) {
      return '<div class="band-row">' +
        '<span class="mono" style="width:18px;color:var(--ink-dim);">' + (i + 1) + '</span>' +
        '<select data-band="' + i + '" data-key="material" aria-label="Band ' + (i + 1) + ' material">' + options(b.material) + '</select>' +
        '<input type="number" min="0" style="width:64px;" data-band="' + i + '" data-key="min" value="' + esc(b.min) + '" aria-label="Band ' + (i + 1) + ' least cells" />' +
        '<span>to</span><input type="number" min="0" style="width:64px;" data-band="' + i + '" data-key="max" value="' + esc(b.max) + '" aria-label="Band ' + (i + 1) + ' most cells" />' +
        '<span>cells</span><button type="button" class="btn small" data-band-remove="' + i + '" aria-label="Remove band ' + (i + 1) + '">×</button></div>';
    }).join('');
    return '<div class="panel-sub">Ground</div><div class="form-grid">' +
      '<div class="insp-row"><label for="gd_elev">Default elevation (cells)</label><input type="number" min="0" id="gd_elev" value="' + esc(t.default_elevation || 0) + '" /></div>' +
      '<div class="insp-row"><label for="gd_ramp">Relief (± cells)</label><input type="number" min="0" id="gd_ramp" value="' + esc((t.relief || {}).amplitude || 0) + '" /></div>' +
      '<div class="insp-row"><label for="gd_rscale">Relief scale (cells across a bump)</label><input type="number" min="0" id="gd_rscale" value="' + esc((t.relief || {}).scale || 0) + '" /></div>' +
      '<div class="insp-row"><label for="gd_bearing">Bearing (load a column carries)</label><input type="number" min="0" id="gd_bearing" value="' + esc(t.bearing) + '" /></div>' +
      '<div class="insp-row"><label for="gd_found">Foundations raise it to</label><input type="number" min="0" id="gd_found" value="' + esc(t.foundation_max) + '" /></div>' +
      '<div class="insp-row"><label for="gd_dig">Dig depth (cells)</label><input type="number" min="0" id="gd_dig" value="' + esc(t.dig_depth) + '" /></div>' +
      '</div>' +
      '<p class="hint" style="margin-bottom:4px;">Strata, from the surface down. Each map picks a thickness per band from its seed; the last band continues to the dig depth.</p>' +
      rows + '<button type="button" class="btn small" id="gd_addBand">Add band</button>' +
      '<p class="hint" style="margin:12px 0 4px;">Liquid bodies: depth below the surface, the chance a tile has one, and the chance it reaches the surface (a vent or pool).</p>' +
      bodies + '<button type="button" class="btn small" id="gd_addLiquid">Add liquid body</button>' +
      (t.foundation_max < t.bearing ? '<p class="warn-inline">Foundations can\'t lower bearing: raise "Foundations raise it to" to at least ' + esc(t.bearing) + '.</p>' : '');
  }
  function bindGround(t) {
    var save = function () { saveTerrainLib(); renderTerrainView(); };
    var num = function (el) { return Math.max(0, Number(el.value) || 0); };
    [['gd_elev', 'default_elevation'], ['gd_bearing', 'bearing'], ['gd_found', 'foundation_max'], ['gd_dig', 'dig_depth']].forEach(function (pair) {
      document.getElementById(pair[0]).addEventListener('change', function () { t[pair[1]] = num(this); save(); });
    });
    [['gd_ramp', 'amplitude'], ['gd_rscale', 'scale']].forEach(function (pair) {
      document.getElementById(pair[0]).addEventListener('change', function () { t.relief = t.relief || {}; t.relief[pair[1]] = num(this); save(); });
    });
    document.querySelectorAll('#terrainDetail [data-body]').forEach(function (el) {
      el.addEventListener('change', function () {
        var body = t.liquids[Number(el.getAttribute('data-body'))], key = el.getAttribute('data-key');
        if (key === 'material') body.material = el.value;
        else if (key === 'chance' || key === 'surface_chance') body[key] = Math.min(100, num(el)) / 100;
        else { body[key] = num(el); if (body.min > body.max) body[key === 'min' ? 'max' : 'min'] = body[key]; }
        save();
      });
    });
    document.querySelectorAll('#terrainDetail [data-body-remove]').forEach(function (el) {
      el.addEventListener('click', function () { t.liquids.splice(Number(el.getAttribute('data-body-remove')), 1); save(); });
    });
    document.getElementById('gd_addLiquid').addEventListener('click', function () {
      t.liquids = t.liquids || [];
      t.liquids.push(water(12, 24, 1, 0)); save();
    });
    document.querySelectorAll('#terrainDetail [data-band]').forEach(function (el) {
      el.addEventListener('change', function () {
        var band = t.strata[Number(el.getAttribute('data-band'))], key = el.getAttribute('data-key');
        band[key] = key === 'material' ? el.value : num(el);
        if (band.min > band.max) band[key === 'min' ? 'max' : 'min'] = band[key];
        save();
      });
    });
    document.querySelectorAll('#terrainDetail [data-band-remove]').forEach(function (el) {
      el.addEventListener('click', function () { t.strata.splice(Number(el.getAttribute('data-band-remove')), 1); save(); });
    });
    document.getElementById('gd_addBand').addEventListener('click', function () {
      t.strata = t.strata || [];
      t.strata.push({ material: terrainLib.materials.length ? terrainLib.materials[0].id : '', min: 1, max: 2 });
      save();
    });
  }
  document.getElementById('roadMult').addEventListener('change', function () {
    terrainLib.road_move_multiplier = Math.max(0, Number(this.value) || 0); saveTerrainLib();
  });

  // ---------- export ----------
  function pos(k) { var p = parseKey(k); return { row: p[0], col: p[1] }; }
  function tileExport(k) {
    var t = tileAt(k), cap = capacity(k);
    return {
      terrain: tileTerrainId(k), terrain_is_base: !t.terrain,
      feature: t.feature || null,
      bridge: t.bridge || null,
      stability_override: t.stability == null ? null : t.stability,
      max_height_override: t.max_height == null ? null : t.max_height,
      max_width_override: t.max_width == null ? null : t.max_width,
      dig_depth_override: t.max_depth == null ? null : t.max_depth,
      elevation_override: t.elevation == null ? null : t.elevation,
      elevation: elevationAt(k),
      effective_capacity: cap,
      upgrade_slots: t.upgrade_slots || 0, upgrades: t.upgrades || []
    };
  }

  function buildExport() {
    var roster = activeFactions();
    var warnings = [];
    var nodes = Object.keys(state.cells).map(function (k) {
      var d = state.cells[k];
      var s = structureStats(d);
      structureProblems(k, d).forEach(function (p) { warnings.push("node '" + d.id + "': " + p); });
      if (d.owning_faction_id && !roster.some(function (f) { return f.id === d.owning_faction_id; })) {
        warnings.push("node '" + d.id + "': owning_faction_id '" + d.owning_faction_id + "' is not in this map's factions roster");
      }
      (d.hidden_from || []).forEach(function (fid) {
        if (!roster.some(function (f) { return f.id === fid; })) warnings.push("node '" + d.id + "': hidden_from_faction_ids '" + fid + "' is not in this map's factions roster");
      });
      if (window.BreachPlannerSubnodes) BreachPlannerSubnodes.problems(d.subnodes || []).forEach(function (m) { warnings.push("node '" + d.id + "': " + m); });
      d.garrison_units.forEach(function (u) {
        if (u.faction_id && !roster.some(function (f) { return f.id === u.faction_id; })) {
          warnings.push("node '" + d.id + "': garrison unit faction_id '" + u.faction_id + "' is not in this map's factions roster");
        }
      });
      return {
        id: d.id, node_type: d.node_type, is_critical_asset: !!d.is_critical_asset,
        owning_faction_id: d.owning_faction_id || null,
        hidden_from_faction_ids: d.hidden_from || [],
        grid_position: pos(k),
        footprint: footprintOf(k).map(pos),
        structure: structureExport(k, d),
        plan: d.plan || null,
        subnodes: d.subnodes || [],
        fields: d.fields || {},
        garrison_count: garrisonCount(d),
        garrison_units: d.garrison_units
      };
    });
    var usedTerrain = {}, usedFeature = {};
    usedTerrain[state.defaultTerrain] = true;
    var tiles = Object.keys(state.tiles).map(function (k) {
      var t = state.tiles[k];
      if (t.terrain) usedTerrain[t.terrain] = true;
      if (t.feature) usedFeature[t.feature] = true;
      if ((t.upgrades || []).length > (t.upgrade_slots || 0)) warnings.push('tile ' + cellLabel(k) + ': ' + t.upgrades.length + ' upgrades exceed ' + plural(t.upgrade_slots || 0, 'slot'));
      return Object.assign({ grid_position: pos(k) }, tileExport(k));
    });
    var routes = computeRoutes();
    warnings = warnings.concat(roadWarnings(), routeWarnings(routes), lossWarnings());
    return {
      // Envelope read by Godot's DesignerMapImporter (specs/16); bump format_version on breaking changes.
      format: 'breach-designer-map',
      format_version: 1,
      cell_size: 64,
      map_name: state.mapName,
      grid: { cols: state.cols, rows: state.rows },
      default_terrain: state.defaultTerrain,
      ceiling: state.ceiling,
      seed: state.seed,
      channels: state.channels.map(function (ch) {
        return { tiles: ch.tiles.map(pos), width: ch.width, depth: ch.depth, liquid: ch.liquid || null, liquid_depth: ch.liquid_depth };
      }),
      factions: roster,
      faction_relations: state.faction_relations,
      loss_criteria: state.lossCriteria.filter(function (g) { return state.mapFactionIds.indexOf(g.faction_id) !== -1; }),
      nodes: nodes,
      tiles: tiles,
      roads: state.roads.map(function (e) { return { a: pos(e.a), b: pos(e.b) }; }),
      links: state.links.map(function (l, i) {
        var rt = routes[i];
        return { a: l.a, b: l.b, route_found: rt.found, route_cost: rt.cost, route: rt.cells.map(pos) };
      }),
      terrain_library: terrainLib.terrains.filter(function (t) { return usedTerrain[t.id]; }),
      feature_library: terrainLib.features.filter(function (f) { return usedFeature[f.id]; }),
      road_move_multiplier: terrainLib.road_move_multiplier,
      static_defense_library: staticDefenseLib.filter(function (x) { return defenseUsage(x.id) > 0; }),
      structure_feature_library: featureLib.features.filter(function (x) { return featureUsageCount(x.id) > 0; }),
      feature_points_per_segment: featurePointsPerSegment(),
      validation_warnings: warnings,
      note: 'Imported into Godot by DesignerMapImporter (specs/16, specs/19): nodes (incl. hidden_from_faction_ids, Decision 28, and is_critical_asset), factions, relations, links (lanes, edges, off-lane nodes; the route of each link is its initial geometry, Decision 26), the layout (grid, default_terrain, tiles: terrain/feature/bridge/capacity overrides/upgrades; roads) and loss_criteria (groups are ORed; each is ANY/ALL of its critical assets). Terrain and feature definitions live in ONE shared library (content/terrain/terrain_library.json, Decision 32), which the designer terrain library writes; terrain_library/feature_library here are informational copies of the entries this map uses. NOT YET IMPORTED (spec 20): node structure (Decision 27: a 2D side-on plane; segments, boundaries, roofs, approaches), static_defense_library, structure_feature_library. tiles lists only cells with at least one explicit layer; any other cell is plain default_terrain. effective_capacity, garrison_count and requirements are derived. _display_label is prototype-only. factions is the roster of this map only; library default relations are not exported.'
    };
  }

  // ---------- load from export (Decision 30) ----------
  // A saved content/maps_src/<name>.designer.json IS the export, so reopening a map
  // rebuilds the designer state from the export's own fields. There is no second, private
  // copy that could drift from what Godot imports. Derived fields (routes, capacity,
  // requirements, manning, ...) are recomputed. Library entries the map uses but this
  // library lacks are merged in, so a map from another machine still renders.
  function gridKey(p) { return key(p.row, p.col); }
  function mergeById(target, entries) {
    var added = 0;
    (entries || []).forEach(function (e) {
      if (e && e.id && !target.some(function (x) { return x.id === e.id; })) { target.push(clone(e)); added++; }
    });
    return added;
  }
  function mergeLibrariesFrom(data) {
    var added = 0, n;
    n = mergeById(libraryFactions, data.factions); // the export's roster entries are full library entries
    if (n) saveLibrary();
    added += n;
    n = mergeById(terrainLib.terrains, data.terrain_library) + mergeById(terrainLib.features, data.feature_library);
    if (n) saveTerrainLib();
    added += n;
    n = mergeById(staticDefenseLib, data.static_defense_library);
    if (n) saveDefenseLib();
    added += n;
    n = mergeById(featureLib.features, data.structure_feature_library);
    if (n) saveFeatureLib();
    return added + n;
  }
  function boundaryKeyFromExport(b) {
    if (b.kind === 'WALL') return 'W:' + sKey(b.col, b.level);
    if (b.kind === 'FLOOR') return 'F:' + sKey(b.col, b.level);
    return 'R:' + b.col;
  }
  function structureFromExport(sx) {
    var st = { archetype: (sx && sx.archetype) || 'NONE', segments: {}, boundaries: {}, roofs: {} };
    if (!sx) return normalizeStructure(st);
    (sx.segments || []).forEach(function (g) {
      var sg = { features: (g.features || []).slice(), emplacements: (g.emplacements || []).slice() };
      if (g.label) sg.label = g.label;
      st.segments[sKey(g.col, g.level)] = sg;
    });
    (sx.boundaries || []).forEach(function (b) {
      var bk = boundaryKeyFromExport(b), kind = bKind(bk);
      var stored = { preset: b.preset || defaultPreset(kind), fortification: b.fortification || 'NONE' };
      if (kind === 'WALL') {
        stored.material = b.material || 'TIMBER';
        if (!b.exterior_side && b.outside) stored.outside = b.outside; // internal walls only
      }
      if (stored.preset === 'CUSTOM') {
        stored.move = (b.blocks_movement_from || []).slice();
        stored.proj = (b.blocks_projectiles_from || []).slice();
        stored.sight = (b.blocks_sight_from || []).slice();
      }
      st.boundaries[bk] = stored;
    });
    (sx.roofs || []).forEach(function (r) { st.roofs[r.col] = { type: r.type || 'FLAT', emplacements: (r.emplacements || []).slice() }; });
    return normalizeStructure(st);
  }
  function nodeFromExport(x) {
    var d = {
      type: 'NODE', id: x.id, node_type: x.node_type || 'NEUTRAL', is_critical_asset: !!x.is_critical_asset,
      owning_faction_id: x.owning_faction_id || '', hidden_from: (x.hidden_from_faction_ids || []).slice(),
      fields: clone(x.fields || {}), garrison_units: clone(x.garrison_units || []),
      structure: structureFromExport(x.structure)
    };
    if (x.plan) d.plan = clone(x.plan);
    if ((x.subnodes || []).length) d.subnodes = clone(x.subnodes);
    if ((x.footprint || []).length > 1) d.footprint = x.footprint.map(gridKey);
    migrateNode(d);
    return d;
  }
  function tileFromExport(t) {
    var tile = {};
    if (t.terrain && !t.terrain_is_base) tile.terrain = t.terrain;
    if (t.feature) tile.feature = t.feature;
    if (t.bridge) tile.bridge = clone(t.bridge);
    [['stability_override', 'stability'], ['max_height_override', 'max_height'], ['max_width_override', 'max_width'], ['dig_depth_override', 'max_depth'], ['elevation_override', 'elevation']].forEach(function (pair) {
      if (t[pair[0]] != null) tile[pair[1]] = t[pair[0]];
    });
    if (t.upgrade_slots) tile.upgrade_slots = t.upgrade_slots;
    if ((t.upgrades || []).length) tile.upgrades = t.upgrades.slice();
    return tile;
  }
  // Approaches default from grid geometry; only a side that differs from that default is
  // an authored override (so moving a node later still re-derives the rest).
  function restoreApproaches(exportNodes) {
    exportNodes.forEach(function (x) {
      var k = findNodeKeyById(x.id), d = k && state.cells[k];
      if (!d) return;
      ((x.structure && x.structure.approaches) || []).forEach(function (a) {
        if (a.side && a.side !== approachSide(d, a.from_node)) {
          d.approaches = d.approaches || {};
          d.approaches[a.from_node] = a.side;
        }
      });
    });
  }
  function nextNodeNumber() {
    var max = 0;
    Object.keys(state.cells).forEach(function (k) { var m = /^n(\d+)$/.exec(state.cells[k].id); if (m) max = Math.max(max, Number(m[1])); });
    return max + 1;
  }
  function loadFromExport(data) {
    if (!data || data.format !== 'breach-designer-map') throw new Error('not a designer map (format must be "breach-designer-map")');
    if (data.format_version !== 1) throw new Error('unsupported format_version ' + data.format_version + ' (this designer reads 1)');
    var librariesAdded = mergeLibrariesFrom(data);
    state.mapName = data.map_name || 'Untitled map';
    state.cols = (data.grid && data.grid.cols) || state.cols;
    state.rows = (data.grid && data.grid.rows) || state.rows;
    state.defaultTerrain = data.default_terrain || 'FIELDS';
    state.ceiling = data.ceiling || 64;
    state.seed = data.seed || 1;
    state.channels = (data.channels || []).map(function (ch) {
      return { tiles: (ch.tiles || []).map(function (p) { return key(p.row, p.col); }), width: ch.width, depth: ch.depth, liquid: ch.liquid || '', liquid_depth: ch.liquid_depth || 0 };
    });
    state.mapFactionIds = (data.factions || []).map(function (f) { return f.id; });
    if (state.mapFactionIds.indexOf('player') === -1) state.mapFactionIds.unshift('player');
    state.faction_relations = clone(data.faction_relations || []);
    state.lossCriteria = clone(data.loss_criteria || []);
    state.cells = {};
    (data.nodes || []).forEach(function (x) { state.cells[gridKey(x.grid_position)] = nodeFromExport(x); });
    state.tiles = {};
    (data.tiles || []).forEach(function (t) { state.tiles[gridKey(t.grid_position)] = tileFromExport(t); });
    state.roads = (data.roads || []).map(function (e) { return { a: gridKey(e.a), b: gridKey(e.b) }; });
    state.links = (data.links || []).map(function (l) { return { a: l.a, b: l.b }; });
    restoreApproaches(data.nodes || []);
    state.nextId = { NODE: nextNodeNumber() };
    if (data.road_move_multiplier != null && terrainLib.road_move_multiplier !== data.road_move_multiplier) {
      terrainLib.road_move_multiplier = data.road_move_multiplier; saveTerrainLib();
    }
    if (data.feature_points_per_segment != null && featureLib.feature_points_per_segment !== data.feature_points_per_segment) {
      featureLib.feature_points_per_segment = data.feature_points_per_segment; saveFeatureLib();
    }
    state.selectedCell = null; state.structureSel = null; state.linkPending = null; state.roadAnchor = null; state.viewAs = '';
    state.bottomTab = 'paint';
    safeSave();
    renderLanesAll();
    return { librariesAdded: librariesAdded };
  }

  document.getElementById('exportBtn').addEventListener('click', function () {
    var data = buildExport();
    document.getElementById('exportJson').textContent = JSON.stringify(data, null, 2);
    document.getElementById('exportWarnings').innerHTML = data.validation_warnings.length
      ? '<div class="warnings">' + data.validation_warnings.map(function (w) { return '&bull; ' + esc(w); }).join('<br>') + '</div>'
      : '';
    document.getElementById('exportModal').hidden = false;
  });
  document.getElementById('closeExport').addEventListener('click', function () { document.getElementById('exportModal').hidden = true; });
  document.getElementById('doneExport').addEventListener('click', function () { document.getElementById('exportModal').hidden = true; });
  document.getElementById('copyExport').addEventListener('click', function () {
    var text = document.getElementById('exportJson').textContent;
    var btn = document.getElementById('copyExport');
    function selectFallback() {
      var range = document.createRange();
      range.selectNodeContents(document.getElementById('exportJson'));
      var sel = window.getSelection(); sel.removeAllRanges(); sel.addRange(range);
      btn.textContent = 'Selected - press Ctrl+C';
    }
    try {
      navigator.clipboard.writeText(text).then(function () {
        btn.textContent = 'Copied';
        setTimeout(function () { btn.textContent = 'Copy JSON'; }, 1200);
      }, selectFallback);
    } catch (e) { selectFallback(); }
  });

  // ---------- toolbar ----------
  function populateTerrainSelect(sel, current) {
    sel.innerHTML = baseTerrains().map(function (t) {
      return '<option value="' + esc(t.id) + '"' + (t.id === current ? ' selected' : '') + '>' + esc(t.label) + '</option>';
    }).join('');
  }
  document.getElementById('mapName').addEventListener('change', function () { state.mapName = this.value; safeSave(); });
  document.getElementById('resizeBtn').addEventListener('click', function () {
    state.cols = Math.max(4, Math.min(24, Number(document.getElementById('cols').value) || state.cols));
    state.rows = Math.max(3, Math.min(16, Number(document.getElementById('rows').value) || state.rows));
    function outside(k) { var p = parseKey(k); return p[0] >= state.rows || p[1] >= state.cols; }
    Object.keys(state.cells).forEach(function (k) { if (outside(k)) { removeNodeRefs(state.cells[k].id); delete state.cells[k]; } });
    Object.keys(state.tiles).forEach(function (k) { if (outside(k)) delete state.tiles[k]; });
    state.roads = state.roads.filter(function (e) { return !outside(e.a) && !outside(e.b); });
    state.selectedCell = null;
    safeSave(); renderLanesAll();
  });
  document.getElementById('seed').addEventListener('change', function () {
    state.seed = Math.max(1, Math.round(Number(this.value)) || 1); this.value = state.seed; safeSave();
  });
  document.getElementById('rerollSeed').addEventListener('click', function () {
    state.seed = 1 + Math.floor(Math.random() * 2147483646); document.getElementById('seed').value = state.seed; safeSave();
  });
  document.getElementById('ceiling').addEventListener('change', function () {
    state.ceiling = Math.max(1, Number(this.value) || 64); this.value = state.ceiling; safeSave(); renderGrid(); renderInspector();
  });
  document.getElementById('defaultTerrain').addEventListener('change', function () {
    state.defaultTerrain = this.value; safeSave(); renderGrid(); renderInspector();
    if (state.bottomTab === 'structure') renderStructureEditor();
  });

  function resetMap(name, cols, rows, factionIds, baseTerrain) {
    state.mapName = name; state.cols = cols; state.rows = rows;
    state.cells = {}; state.tiles = {}; state.roads = []; state.links = []; state.selectedCell = null; state.nextId = { NODE: 1 };
    state.mapFactionIds = factionIds; state.lossCriteria = [];
    state.faction_relations = relationsFromDefaults();
    state.defaultTerrain = baseTerrain; state.ceiling = 64; state.channels = [];
    state.seed = 1 + Math.floor(Math.random() * 2147483646);
    state.bottomTab = 'paint'; state.structureSel = null;
    safeSave(); renderLanesAll();
  }
  document.getElementById('clearBtn').addEventListener('click', function () {
    var base = baseTerrains()[0];
    resetMap(state.mapName, state.cols, state.rows, ['player'], base ? base.id : 'FIELDS');
  });

  function openNewMapModal() {
    document.getElementById('nmName').value = 'New map';
    document.getElementById('nmCols').value = 12;
    document.getElementById('nmRows').value = 7;
    populateTerrainSelect(document.getElementById('nmDefaultTerrain'), state.defaultTerrain);
    var list = document.getElementById('nmFactionList');
    list.innerHTML = '';
    libraryFactions.forEach(function (f) {
      var row = document.createElement('label');
      row.innerHTML = '<input type="checkbox" value="' + esc(f.id) + '"' + (f.id === 'player' ? ' checked disabled' : '') + ' /> ' +
        '<span class="swatch" style="background:' + factionColor(f.id) + '"></span> ' + esc(f.display_name);
      list.appendChild(row);
    });
    document.getElementById('newMapModal').hidden = false;
  }
  document.getElementById('newMapBtn').addEventListener('click', openNewMapModal);
  document.getElementById('closeNewMap').addEventListener('click', function () { document.getElementById('newMapModal').hidden = true; });
  document.getElementById('cancelNewMap').addEventListener('click', function () { document.getElementById('newMapModal').hidden = true; });
  document.getElementById('createNewMap').addEventListener('click', function () {
    var chosen = Array.prototype.slice.call(document.querySelectorAll('#nmFactionList input:checked')).map(function (el) { return el.value; });
    if (chosen.indexOf('player') === -1) chosen.unshift('player');
    document.getElementById('newMapModal').hidden = true;
    resetMap(
      document.getElementById('nmName').value.trim() || 'New map',
      Math.max(4, Math.min(24, Number(document.getElementById('nmCols').value) || 12)),
      Math.max(3, Math.min(16, Number(document.getElementById('nmRows').value) || 7)),
      chosen,
      document.getElementById('nmDefaultTerrain').value || 'FIELDS'
    );
  });

  // ---------- boot ----------
  function renderLanesAll() {
    document.getElementById('mapName').value = state.mapName;
    document.getElementById('cols').value = state.cols;
    document.getElementById('rows').value = state.rows;
    populateTerrainSelect(document.getElementById('defaultTerrain'), state.defaultTerrain);
    document.getElementById('ceiling').value = state.ceiling;
    document.getElementById('seed').value = state.seed;
    renderRoster();
    renderRelations();
    renderRelationSelects();
    renderPlacementFactionSelect();
    renderLoss();
    renderGrid();
    renderInspector();
    renderCounts();
    renderLinks();
    setBottomTab(state.bottomTab);
  }

  renderPresets();
  renderUnits();
  var savedView = storeGet(VIEW_KEY);
  showView(['factions', 'terrain', 'defenses', 'structures'].indexOf(savedView) !== -1 ? savedView : 'lanes');

  // Repo mode (repo.js) drives Open/Save through this.
  window.BreachDesigner = {
    buildExport: buildExport,
    loadFromExport: function (data) { var r = loadFromExport(data); showView('lanes'); return r; },
    mapName: function () { return state.mapName; },
    libraryKeys: {
      factions: LIBRARY_KEY, default_relations: DEFAULT_REL_KEY, terrain: TERRAIN_LIBRARY_KEY,
      emplacements: DEFENSE_LIBRARY_KEY, room_features: FEATURE_LIBRARY_KEY,
      room_prefabs: ROOM_PREFAB_KEY, structure_prefabs: STRUCTURE_PREFAB_KEY
    }
  };
})();
