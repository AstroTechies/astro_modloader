use std::{collections::HashMap, io};

use crate::unreal_modintegrator::bake_instructions;
use handlers::MAP_PATHS;
use unreal_modloader::unreal_asset::ue4version::VER_UE4_23;
use unreal_modloader::unreal_modintegrator::helpers::game_to_absolute;
use unreal_modloader::unreal_modintegrator::BakedInstructions;
use unreal_modloader::unreal_modintegrator::IntegratorConfig;

use lazy_static::lazy_static;

pub mod assets;
pub(crate) mod handlers;

use crate::handlers::{
    biome_placement_modifiers, item_list_entries, linked_actor_components, mission_trailheads,
};

pub use unreal_modloader;
pub use unreal_modloader::unreal_asset;
pub use unreal_modloader::unreal_modintegrator;
pub use unreal_modloader::unreal_modmetadata;
pub use unreal_modloader::unreal_pak;
pub struct AstroIntegratorConfig;

lazy_static! {
    static ref FILE_REFS: HashMap<String, &'static [u8]> = HashMap::from([
        (
            game_to_absolute(
                AstroIntegratorConfig::GAME_NAME,
                "/Game/Integrator/NotificationActor.uasset"
            )
            .unwrap(),
            assets::ALERT_MOD_NOTIFICATION_ACTOR_ASSET
        ),
        (
            game_to_absolute(
                AstroIntegratorConfig::GAME_NAME,
                "/Game/Integrator/NotificationActor.uexp"
            )
            .unwrap(),
            assets::ALERT_MOD_NOTIFICATION_ACTOR_EXPORT
        ),
    ]);
}

impl<'data> IntegratorConfig<'data, (), io::Error> for AstroIntegratorConfig {
    fn get_data(&self) -> &'data () {
        &()
    }

    fn get_handlers(
        &self,
    ) -> std::collections::HashMap<
        String,
        Box<
            dyn FnMut(
                &(),
                &mut unreal_pak::PakFile,
                &mut Vec<unreal_pak::PakFile>,
                &mut Vec<unreal_pak::PakFile>,
                &Vec<serde_json::Value>,
            ) -> Result<(), io::Error>,
        >,
    > {
        type HandlerFn = dyn FnMut(
            &(),
            &mut unreal_pak::PakFile,
            &mut Vec<unreal_pak::PakFile>,
            &mut Vec<unreal_pak::PakFile>,
            &Vec<serde_json::Value>,
        ) -> Result<(), io::Error>;
        let mut handlers: std::collections::HashMap<String, Box<HandlerFn>> = HashMap::new();

        handlers.insert(
            String::from("mission_trailheads"),
            Box::new(mission_trailheads::handle_mission_trailheads),
        );

        handlers.insert(
            String::from("linked_actor_components"),
            Box::new(linked_actor_components::handle_linked_actor_components),
        );

        handlers.insert(
            String::from("item_list_entries"),
            Box::new(item_list_entries::handle_item_list_entries),
        );

        handlers.insert(
            String::from("biome_placement_modifiers"),
            Box::new(biome_placement_modifiers::handle_biome_placement_modifiers),
        );

        handlers
    }

    fn get_instructions(&self) -> Option<BakedInstructions> {
        let instructions = bake_instructions!(
            "persistent_actors": ["/Game/Integrator/NotificationActor"],
            "persistent_actor_maps": MAP_PATHS
        );

        Some(BakedInstructions::new(FILE_REFS.clone(), instructions))
    }

    const GAME_NAME: &'static str = "Astro";
    const INTEGRATOR_VERSION: &'static str = "0.1.0";
    const ENGINE_VERSION: i32 = VER_UE4_23;
}
