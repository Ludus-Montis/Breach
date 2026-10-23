class_name TerrainDef
extends Resource
## One terrain type in the shared terrain library, per
## specs/19-map-layout-and-objectives.md (Decision 32). Filled from
## content/terrain/terrain_library.json (CrateContent); maps reference terrains by id.

const StratumDef = preload("res://content/definitions/stratum_def.gd")
const LiquidBodyDef = preload("res://content/definitions/liquid_body_def.gd")

@export var id: String = ""
@export var display_name: String = ""
## Short symbol the designer draws on the cell.
@export var glyph: String = ""
@export var color: Color = Color.WHITE
## Can be a map's base (default) terrain.
@export var can_be_base: bool = true
## Impassable without a bridge (water, ravine).
@export var needs_bridge: bool = false
## Movement cost to enter a cell of this terrain (a road multiplies it).
@export var move_cost: float = 1.0
## Unit classes that cannot enter this terrain (no unit-class schema yet; ids only).
@export var blocks_unit_classes: Array[String] = []
## The ground (Decisions 53, 54): the load a cell column bears, the most foundations can
## raise it to, and how deep the strata go, in cells.
@export var bearing: int = 0
## Ground height in cells for a tile of this terrain that sets none (Decision 56).
@export var default_elevation: int = 0
## Seeded unevenness within a tile (Decision 59): up to this many cells above or below the
## tile's height, varying over about relief_scale cells; 0 = smooth.
@export var relief_amplitude: int = 0
@export var relief_scale: int = 0
@export var foundation_max: int = 0
@export var dig_depth: int = 0
## Bodies of water or lava (materials that flow) it may hold (Decisions 62, 64).
@export var liquids: Array[LiquidBodyDef] = []
## Bands from the surface down; the last continues to dig_depth.
@export var strata: Array[StratumDef] = []
## Legacy: the side-on structure editor's capacity (Decision 27), until the plan editor of
## Decision 52 replaces it. TileDef can override them.
@export var default_stability: int = 0
@export var default_max_height: int = 0
@export var default_max_width: int = 0
@export var default_max_length: int = 0
@export var default_max_depth: int = 0
