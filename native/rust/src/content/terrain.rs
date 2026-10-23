//! The one shared terrain library (Decision 32, `TerrainLibraryDef`): every terrain a map's
//! tiles can be, the features a tile can carry and the materials strata, liquids and walls are
//! made of. One file, `content/terrain/terrain_library.json`, which the designer's library page
//! reads and writes. The fields and their defaults are GDScript's, name for name: until their
//! last reader ports, GDScript's definitions are filled from these (Decision 153).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A colour as the designer writes it, `#rrggbb` or `#rrggbbaa`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Colour(String);

impl JsonSchema for Colour {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Colour".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^#([0-9a-fA-F]{6}|[0-9a-fA-F]{8})$",
        })
    }
}

impl Colour {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Colour {
    /// `Color.WHITE`.
    fn default() -> Self {
        Colour("#ffffff".into())
    }
}

impl TryFrom<String> for Colour {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let digits = text.strip_prefix('#').unwrap_or("");
        let hex = digits.chars().all(|c| c.is_ascii_hexdigit());
        if hex && (digits.len() == 6 || digits.len() == 8) {
            Ok(Colour(text))
        } else {
            Err(format!("{text:?} is not a colour (#rrggbb or #rrggbbaa)"))
        }
    }
}

impl From<Colour> for String {
    fn from(colour: Colour) -> String {
        colour.0
    }
}

/// The shared library (`TerrainLibraryDef`), each list in the designer's order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct TerrainLibrary {
    pub terrains: Vec<Terrain>,
    pub features: Vec<TerrainFeature>,
    /// What strata (and later walls) are made of (Decisions 54, 57).
    pub materials: Vec<Material>,
    /// Multiplies a terrain's move_cost on a cell with a road.
    pub road_move_multiplier: f64,
}

impl Default for TerrainLibrary {
    fn default() -> Self {
        TerrainLibrary {
            terrains: Vec::new(),
            features: Vec::new(),
            materials: Vec::new(),
            road_move_multiplier: 0.5,
        }
    }
}

impl TerrainLibrary {
    pub fn has_material(&self, id: &str) -> bool {
        self.materials.iter().any(|material| material.id == id)
    }

    /// The field (`terrains[0].strata[1].material_id`) and id of the first material named
    /// that isn't in the library: a stratum's, a liquid body's or what a heat transition
    /// becomes.
    pub fn missing_material(&self) -> Option<(String, String)> {
        for (at, terrain) in self.terrains.iter().enumerate() {
            for (index, stratum) in terrain.strata.iter().enumerate() {
                if !self.has_material(&stratum.material_id) {
                    let field = format!("terrains[{at}].strata[{index}].material_id");
                    return Some((field, stratum.material_id.clone()));
                }
            }
            for (index, body) in terrain.liquids.iter().enumerate() {
                if !self.has_material(&body.material_id) {
                    let field = format!("terrains[{at}].liquids[{index}].material_id");
                    return Some((field, body.material_id.clone()));
                }
            }
        }
        for (at, material) in self.materials.iter().enumerate() {
            for (index, transition) in material.heat_transitions.iter().enumerate() {
                if !transition.becomes.is_empty() && !self.has_material(&transition.becomes) {
                    let field = format!("materials[{at}].heat_transitions[{index}].becomes");
                    return Some((field, transition.becomes.clone()));
                }
            }
        }
        None
    }
}

/// One terrain type (`TerrainDef`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Terrain {
    pub id: String,
    pub display_name: String,
    /// Short symbol the designer draws on the cell.
    pub glyph: String,
    pub color: Colour,
    /// Can be a map's base (default) terrain.
    pub can_be_base: bool,
    /// Impassable without a bridge (water, ravine).
    pub needs_bridge: bool,
    /// Movement cost to enter a cell of this terrain (a road multiplies it).
    pub move_cost: f64,
    /// Unit classes that cannot enter this terrain (ids only).
    pub blocks_unit_classes: Vec<String>,
    /// The ground (Decisions 53, 54): the load a cell column bears, the most foundations can
    /// raise it to, and how deep the strata go, in cells.
    pub bearing: i64,
    /// Ground height in cells for a tile of this terrain that sets none (Decision 56).
    pub default_elevation: i64,
    /// Seeded unevenness within a tile (Decision 59): up to this many cells above or below
    /// the tile's height, varying over about relief_scale cells; 0 = smooth.
    pub relief_amplitude: i64,
    pub relief_scale: i64,
    pub foundation_max: i64,
    pub dig_depth: i64,
    /// Bodies of water or lava (materials that flow) it may hold (Decisions 62, 64).
    pub liquids: Vec<LiquidBody>,
    /// Bands from the surface down; the last continues to dig_depth.
    pub strata: Vec<Stratum>,
    /// The side-on structure editor's capacity (Decision 27); a tile can override them.
    pub default_stability: i64,
    pub default_max_height: i64,
    pub default_max_width: i64,
    pub default_max_length: i64,
    pub default_max_depth: i64,
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain {
            id: String::new(),
            display_name: String::new(),
            glyph: String::new(),
            color: Colour::default(),
            can_be_base: true,
            needs_bridge: false,
            move_cost: 1.0,
            blocks_unit_classes: Vec::new(),
            bearing: 0,
            default_elevation: 0,
            relief_amplitude: 0,
            relief_scale: 0,
            foundation_max: 0,
            dig_depth: 0,
            liquids: Vec::new(),
            strata: Vec::new(),
            default_stability: 0,
            default_max_height: 0,
            default_max_width: 0,
            default_max_length: 0,
            default_max_depth: 0,
        }
    }
}

/// A body of a material that flows a terrain may hold (`LiquidBodyDef`, Decision 62).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct LiquidBody {
    pub material_id: String,
    pub min_cells: i64,
    pub max_cells: i64,
    pub chance: f64,
    pub surface_chance: f64,
}

impl Default for LiquidBody {
    fn default() -> Self {
        LiquidBody {
            material_id: String::new(),
            min_cells: 0,
            max_cells: 0,
            chance: 1.0,
            surface_chance: 0.0,
        }
    }
}

/// One band of a terrain's strata (`StratumDef`, Decision 54), its thickness in cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Stratum {
    pub material_id: String,
    pub min_cells: i64,
    pub max_cells: i64,
}

impl Default for Stratum {
    fn default() -> Self {
        Stratum { material_id: String::new(), min_cells: 1, max_cells: 1 }
    }
}

/// A natural feature a tile can carry (`TerrainFeatureDef`): its effect isn't designed yet.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct TerrainFeature {
    pub id: String,
    pub display_name: String,
    pub glyph: String,
    pub effect_notes: String,
}

/// What strata, walls and liquids are made of (`MaterialDef`, Decisions 54, 57, 63, 64).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Material {
    pub id: String,
    pub display_name: String,
    pub color: Colour,
    /// Load one cell of it puts on what is beneath (Decision 53).
    pub weight: i64,
    /// Cells it can bridge unsupported before it falls (Decision 57).
    pub span: i64,
    /// Load one eighth of a cell of it carries (Decision 65).
    pub strength: i64,
    /// The heat it gives off, a level 0-10 (Decision 66).
    pub heat: i64,
    /// Trait id -> level (Decision 64): dig_difficulty, climb_difficulty, flows, loose, glows.
    pub traits: BTreeMap<String, i64>,
    /// How it reacts to heat.
    pub heat_transitions: Vec<HeatTransition>,
}

impl Default for Material {
    fn default() -> Self {
        Material {
            id: String::new(),
            display_name: String::new(),
            color: Colour::default(),
            weight: 1,
            span: 1,
            strength: 0,
            heat: 1,
            traits: BTreeMap::new(),
            heat_transitions: Vec::new(),
        }
    }
}

/// What a material does past a heat level (`HeatTransitionDef`, Decisions 63, 66): becomes
/// another material, or gains a trait.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct HeatTransition {
    pub threshold: i64,
    /// Above the threshold (true) or below it.
    pub rising: bool,
    pub becomes: String,
    pub gains_trait: String,
}

impl Default for HeatTransition {
    fn default() -> Self {
        HeatTransition {
            threshold: 0,
            rising: true,
            becomes: String::new(),
            gains_trait: String::new(),
        }
    }
}
