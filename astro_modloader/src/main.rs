// Don't show console window in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

use autoupdater::{
    apis::{
        github::{GithubApi, GithubRelease},
        DownloadApiTrait,
    },
    cargo_crate_version,
};
use lazy_static::lazy_static;
use log::info;

use unreal_modloader::{
    config::{GameConfig, IconData, InstallManager},
    error::ModLoaderError,
    game_platform_managers::GetGameBuildTrait,
    unreal_cpp_bootstrapper::config::{FunctionInfo, FunctionInfoPatterns},
    update_info::UpdateInfo,
    version::GameBuild,
};
use unreal_modloader::{
    unreal_cpp_bootstrapper::config::GameSettings, unreal_modintegrator::IntegratorConfig,
};

use astro_modintegrator::AstroIntegratorConfig;

mod logging;

#[cfg(windows)]
#[derive(Debug, Default)]
struct SteamGetGameBuild {
    game_build: RefCell<Option<GameBuild>>,
}

#[cfg(windows)]
use unreal_modloader::game_platform_managers::{MsStoreInstallManager, SteamInstallManager};
#[cfg(windows)]
impl GetGameBuildTrait<SteamInstallManager> for SteamGetGameBuild {
    fn get_game_build(&self, manager: &SteamInstallManager) -> Option<GameBuild> {
        if self.game_build.borrow().is_none() && manager.get_game_install_path().is_some() {
            let version_file_path = manager
                .game_path
                .borrow()
                .as_ref()
                .unwrap()
                .join("build.version");

            if !version_file_path.is_file() {
                info!("{:?} not found", version_file_path);
                return None;
            }

            let version_file = std::fs::read_to_string(&version_file_path).unwrap();
            let game_build_string = version_file.split(' ').next().unwrap().to_owned();

            *self.game_build.borrow_mut() = GameBuild::try_from(&game_build_string).ok();
        }
        *self.game_build.borrow()
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Default)]
struct ProtonGetGameBuild {
    game_build: RefCell<Option<GameBuild>>,
}

#[cfg(target_os = "linux")]
use unreal_modloader::game_platform_managers::ProtonInstallManager;
#[cfg(target_os = "linux")]
impl GetGameBuildTrait<ProtonInstallManager> for ProtonGetGameBuild {
    fn get_game_build(&self, manager: &ProtonInstallManager) -> Option<GameBuild> {
        if self.game_build.borrow().is_none() && manager.get_game_install_path().is_some() {
            let version_file_path = manager
                .game_path
                .borrow()
                .as_ref()
                .unwrap()
                .join("build.version");

            if !version_file_path.is_file() {
                info!("{:?} not found", version_file_path);
                return None;
            }

            let version_file = std::fs::read_to_string(&version_file_path).unwrap();
            let game_build_string = version_file.split(' ').next().unwrap().to_owned();

            *self.game_build.borrow_mut() = GameBuild::try_from(&game_build_string).ok();
        }
        *self.game_build.borrow()
    }
}

struct AstroGameConfig;

fn load_icon() -> IconData {
    let data = include_bytes!("../assets/icon.ico");
    let image = image::load_from_memory(data).unwrap().to_rgba8();

    IconData {
        data: image.to_vec(),
        width: image.width(),
        height: image.height(),
    }
}

lazy_static! {
    static ref RGB_DATA: IconData = load_icon();
}

impl AstroGameConfig {
    fn get_api(&self) -> GithubApi {
        let mut api = GithubApi::new("AstroTechies", "astro_modloader");
        api.current_version(cargo_crate_version!());
        api.prerelease(true);
        api
    }

    fn get_newer_release(&self, api: &GithubApi) -> Result<Option<GithubRelease>, ModLoaderError> {
        api.get_newer(&None)
            .map_err(|e| ModLoaderError::other(e.to_string()))
    }
}

impl<D, E: std::error::Error + 'static> GameConfig<'static, AstroIntegratorConfig, D, E>
    for AstroGameConfig
where
    AstroIntegratorConfig: IntegratorConfig<'static, D, E>,
{
    fn get_integrator_config(&self) -> &AstroIntegratorConfig {
        &AstroIntegratorConfig
    }

    fn get_game_build(&self, install_path: &Path) -> Option<GameBuild> {
        let version_file_path = install_path.join("build.version");
        if !version_file_path.is_file() {
            info!("{:?} not found", version_file_path);
            return None;
        }

        let version_file = std::fs::read_to_string(&version_file_path).unwrap();
        let game_build_string = version_file.split(' ').next().unwrap().to_owned();

        GameBuild::try_from(&game_build_string).ok()
    }

    const WINDOW_TITLE: &'static str = "Astroneer Modloader";
    const CONFIG_DIR: &'static str = "AstroModLoader";
    const CRATE_VERSION: &'static str = cargo_crate_version!();

    fn get_install_managers(
        &self,
    ) -> std::collections::BTreeMap<&'static str, Box<dyn InstallManager>> {
        let mut managers: std::collections::BTreeMap<&'static str, Box<dyn InstallManager>> =
            BTreeMap::new();

        #[cfg(windows)]
        managers.insert(
            "Steam",
            Box::new(SteamInstallManager::new(
                361420,
                AstroIntegratorConfig::GAME_NAME,
                Box::<SteamGetGameBuild>::default(),
            )),
        );
        #[cfg(windows)]
        managers.insert(
            "Microsoft Store",
            Box::new(MsStoreInstallManager::new(
                "SystemEraSoftworks",
                "ASTRONEER",
                "Astro",
            )),
        );

        #[cfg(target_os = "linux")]
        managers.insert(
            "Steam (Proton)",
            Box::new(ProtonInstallManager::new(
                361420,
                AstroIntegratorConfig::GAME_NAME,
                Box::new(ProtonGetGameBuild::default()),
            )),
        );

        managers
    }

    fn get_newer_update(&self) -> Result<Option<UpdateInfo>, ModLoaderError> {
        let api = self.get_api();
        let download = self.get_newer_release(&api)?;

        if let Some(download) = download {
            return Ok(Some(UpdateInfo::new(download.tag_name, download.body)));
        }

        Ok(None)
    }

    fn update_modloader(&self, callback: Box<dyn Fn(f32)>) -> Result<(), ModLoaderError> {
        let api = self.get_api();
        let download = self.get_newer_release(&api)?;

        if let Some(download) = download {
            let asset = &download.assets[0];
            api.download(asset, Some(callback))
                .map_err(|e| ModLoaderError::other(e.to_string()))?;
        }
        Ok(())
    }

    fn get_icon(&self) -> Option<IconData> {
        Some(RGB_DATA.clone())
    }

    fn get_cpp_loader_config() -> GameSettings {
        GameSettings {
            is_using_fchunked_fixed_uobject_array: true,
            uses_fname_pool: true,
            function_info_settings: Some(FunctionInfo::Patterns(FunctionInfoPatterns {
                call_function_by_name_with_arguments: Some("41 57 41 56 41 55 41 54 56 57 55 53 48 81 EC ? ? ? ? 44 0F 29 BC 24 ? ? ? ? 44 0F 29 B4 24 ? ? ? ? 44 0F 29 AC 24 ? ? ? ? 44 0F 29 A4 24 ? ? ? ? 44 0F 29 9C 24 ? ? ? ? 44 0F 29 94 24 ? ? ? ? 44 0F 29 8C 24 ? ? ? ? 44 0F 29 84 24 ? ? ? ? 0F 29 BC 24 ? ? ? ? 0F 29 B4 24 ? ? ? ? 48 8B 8C 24 ? ? ? ? 48 8B 94 24 ? ? ? ? 48 8B 84 24 ? ? ? ? 4C 8B 40 18 0F 28 3D".to_string()),
                create_default_object: Some("48 8B C4 55 41 54 41 55 41 56 41 57 48 8D A8 ? ? ? ? 48 81 EC ? ? ? ? 48 C7 45 ? ? ? ? ? 48 89 58 10 48 89 70 18 48 89 78 20 48 8B 05 ? ? ? ? 48 33 C4 48 89 85 ? ? ? ? 48 8B F9 48 83 B9 ? ? ? ? ? 0F 85 ? ? ? ? 48 8B 59 40 45 33 FF 48 85 DB 74 2E B2 01 48 8B CB".to_string()),
                ..Default::default()
            })),
            ..Default::default()
        }
    }
}

fn main() {
    logging::init().unwrap();

    info!("Astroneer Modloader");

    let config = AstroGameConfig;

    unreal_modloader::run(config);
}
