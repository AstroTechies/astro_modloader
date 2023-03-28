use std::collections::HashMap;

use lazy_static::lazy_static;

use unreal_mod_manager::unreal_asset::engine_version::EngineVersion;
use unreal_mod_manager::unreal_helpers::game_to_absolute;
use unreal_mod_manager::unreal_mod_integrator::{
    BakedMod, Error, HandlerFn, IntegratorConfig, IntegratorMod,
};

pub mod assets;
pub(crate) mod baked;
pub(crate) mod handlers;

use crate::handlers::{
    biome_placement_modifiers, item_list_entries, linked_actor_components, mission_trailheads,
};

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

impl<'data> IntegratorConfig<'data, (), Error> for AstroIntegratorConfig {
    fn get_data(&self) -> &'data () {
        &()
    }

    fn get_handlers(&self) -> std::collections::HashMap<String, Box<HandlerFn<(), Error>>> {
        let mut handlers: std::collections::HashMap<String, Box<HandlerFn<(), Error>>> =
            HashMap::new();

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

    fn get_baked_mods(&self) -> Vec<IntegratorMod<Error>> {
        Vec::from([BakedMod {
            data: baked::CORE_MOD,
            mod_id: "CoreMod".to_string(),
            filename: "800-CoreMod-0.1.0_P.pak",
            is_core: true,
            priority: 800,
        }
        .into()])
    }

    const GAME_NAME: &'static str = "Astro";
    const INTEGRATOR_VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const ENGINE_VERSION: EngineVersion = EngineVersion::VER_UE4_23;
}
