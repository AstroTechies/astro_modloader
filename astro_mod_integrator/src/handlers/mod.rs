use lazy_static::lazy_static;
use regex::Regex;

pub(crate) mod biome_placement_modifiers;
pub(crate) mod item_list_entries;
pub(crate) mod linked_actor_components;
pub(crate) mod mission_trailheads;

lazy_static! {
    static ref GAME_REGEX: Regex = Regex::new(r"^/Game/").unwrap();
}

pub(crate) static MAP_PATHS: [&str; 3] = [
    "Astro/Content/Maps/Staging_T2.umap",
    "Astro/Content/Maps/Staging_T2_PackedPlanets_Switch.umap",
    "Astro/Content/U32_Expansion/U32_Expansion.umap"
];
