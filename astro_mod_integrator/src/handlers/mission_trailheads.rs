use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::Path;

use unreal_mod_manager::unreal_asset::reader::archive_trait::ArchiveTrait;
use unreal_mod_manager::unreal_asset::unversioned::ancestry::Ancestry;
use unreal_mod_manager::unreal_asset::{
    cast,
    engine_version::EngineVersion,
    exports::{Export, ExportNormalTrait},
    properties::{object_property::ObjectProperty, Property},
    types::PackageIndex,
    Import,
};
use unreal_mod_manager::unreal_mod_integrator::{
    helpers::{get_asset, write_asset},
    Error,
};
use unreal_mod_manager::unreal_pak::{PakMemory, PakReader};

use super::MAP_PATHS;

#[allow(clippy::ptr_arg)]
pub(crate) fn handle_mission_trailheads(
    _data: &(),
    integrated_pak: &mut PakMemory,
    game_paks: &mut Vec<PakReader<File>>,
    mod_paks: &mut Vec<PakReader<File>>,
    trailhead_arrays: &Vec<serde_json::Value>,
) -> Result<(), Error> {
    for map_path in MAP_PATHS {
        let mut asset = get_asset(
            integrated_pak,
            game_paks,
            mod_paks,
            &String::from(map_path),
            EngineVersion::VER_UE4_23,
        )?;

        let mut trailheads = Vec::new();
        for trailheads_array in trailhead_arrays {
            let trailheads_array = trailheads_array
                .as_array()
                .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid trailheads"))?;
            for trailhead in trailheads_array {
                let trailhead = trailhead
                    .as_str()
                    .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid trailheads"))?;
                trailheads.push(trailhead);
            }
        }

        let mut mission_data_export_index = None;
        let mut mission_data_property_index = None;

        for i in 0..asset.asset_data.exports.len() {
            let export = &asset.asset_data.exports[i];
            if let Some(normal_export) = export.get_normal_export() {
                if normal_export.base_export.class_index.is_import() {
                    let import = asset
                        .get_import(normal_export.base_export.class_index)
                        .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid import"))?;
                    if import.object_name.get_content() == "AstroSettings" {
                        for j in 0..normal_export.properties.len() {
                            let property = &normal_export.properties[j];
                            if let Some(array_property) = cast!(Property, ArrayProperty, property) {
                                if array_property.name.get_content() == "MissionData"
                                    && array_property
                                        .array_type
                                        .as_ref()
                                        .map(|e| e.get_content() == "ObjectProperty")
                                        .unwrap_or(false)
                                {
                                    mission_data_export_index = Some(i);
                                    mission_data_property_index = Some(j);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let (Some(mission_data_export_index), Some(mission_data_property_index)) =
            (mission_data_export_index, mission_data_property_index)
        {
            for trailhead in trailheads {
                let soft_class_name = Path::new(trailhead)
                    .file_stem()
                    .and_then(|e| e.to_str())
                    .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid trailhead"))?;

                let package_link = Import {
                    class_package: asset.add_fname("/Script/CoreUObject"),
                    class_name: asset.add_fname("Package"),
                    outer_index: PackageIndex::new(0),
                    object_name: asset.add_fname(trailhead),
                    optional: false,
                };
                let package_link = asset.add_import(package_link);

                let mission_data_asset_link = Import {
                    class_package: asset.add_fname("/Script/Astro"),
                    class_name: asset.add_fname("AstroMissionDataAsset"),
                    outer_index: package_link,
                    object_name: asset.add_fname(soft_class_name),
                    optional: false,
                };
                let mission_data_asset_link = asset.add_import(mission_data_asset_link);

                let mission_data_export = cast!(
                    Export,
                    NormalExport,
                    &mut asset.asset_data.exports[mission_data_export_index]
                )
                .expect("Corrupted memory");
                let mission_data_property = cast!(
                    Property,
                    ArrayProperty,
                    &mut mission_data_export.properties[mission_data_property_index]
                )
                .expect("Corrupted memory");

                let property = ObjectProperty {
                    name: mission_data_property.name.clone(),
                    ancestry: Ancestry::default(),
                    property_guid: Some([0u8; 16]),
                    duplication_index: 0,
                    value: mission_data_asset_link,
                };
                mission_data_property.value.push(property.into());
            }
        }

        write_asset(integrated_pak, &asset, &String::from(map_path))
            .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;
    }

    Ok(())
}
