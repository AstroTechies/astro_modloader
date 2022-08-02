#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;

use astro_modintegrator::unreal_modintegrator::IntegratorConfig;
use astro_modintegrator::unreal_modloader::config::{GameConfig, InstallManager};
use astro_modintegrator::unreal_modloader::error::ModLoaderError;
use astro_modintegrator::unreal_modloader::game_platform_managers::{
    GetGameBuildTrait, MsStoreInstallManager, SteamInstallManager,
};
use astro_modintegrator::unreal_modloader::update_info::UpdateInfo;
use astro_modintegrator::unreal_modloader::version::GameBuild;
use astro_modintegrator::unreal_modloader::IconData;
use astro_modintegrator::{unreal_modloader, AstroIntegratorConfig};

mod logging;

use autoupdater::apis::github::GithubApi;
use autoupdater::apis::DownloadApiTrait;
use autoupdater::cargo_crate_version;
use log::info;

use lazy_static::lazy_static;

#[derive(Debug, Default)]
struct SteamGetGameBuild {
    game_build: RefCell<Option<GameBuild>>,
}

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

struct AstroGameConfig;

impl<T, E: std::error::Error> GameConfig<'static, AstroIntegratorConfig, T, E> for AstroGameConfig
where
    AstroIntegratorConfig: IntegratorConfig<'static, T, E>,
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

    fn get_install_managers(
        &self,
    ) -> std::collections::BTreeMap<&'static str, Box<dyn InstallManager>> {
        let mut managers: std::collections::BTreeMap<&'static str, Box<dyn InstallManager>> =
            BTreeMap::new();

        managers.insert(
            "Steam",
            Box::new(SteamInstallManager::new(
                361420,
                AstroIntegratorConfig::GAME_NAME,
                Box::new(SteamGetGameBuild::default()),
            )),
        );
        managers.insert(
            "Microsoft Store",
            Box::new(MsStoreInstallManager::new(
                "SystemEraSoftworks",
                "ASTRONEER",
            )),
        );

        managers
    }

    fn get_newer_update(&self) -> Result<Option<UpdateInfo>, ModLoaderError> {
        let mut api = GithubApi::new("AstroTechies", "astro_modloader");
        api.current_version(cargo_crate_version!());

        let download = api
            .get_newer(&None)
            .map_err(|e| ModLoaderError::other(e.to_string()))?;

        if let Some(download) = download {
            return Ok(Some(UpdateInfo::new(download.tag_name, download.body)));
        }

        Ok(None)
    }

    fn update_modloader(&self, callback: Box<dyn Fn(f32)>) -> Result<(), ModLoaderError> {
        let mut api = GithubApi::new("AstroTechies", "astro_modloader");
        api.current_version(cargo_crate_version!());

        let download = api
            .get_newer(&None)
            .map_err(|e| ModLoaderError::other(e.to_string()))?;

        if let Some(download) = download {
            let asset = &download.assets[0];
            api.download(asset, Some(callback))
                .map_err(|e| ModLoaderError::other(e.to_string()))?;
        }
        Ok(())
    }
}

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

fn main() {
    logging::init().unwrap();

    info!("Astroneer Modloader");

    let config = AstroGameConfig;

    unreal_modloader::run(config, Some(RGB_DATA.clone()));
}
