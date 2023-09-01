use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, ErrorKind};
use std::path::Path;

use log::warn;
use serde::{Deserialize, Serialize};

use unreal_mod_manager::unreal_asset::types::PackageIndexTrait;
use unreal_mod_manager::unreal_asset::unversioned::ancestry::Ancestry;
use unreal_mod_manager::unreal_asset::{
    cast,
    engine_version::EngineVersion,
    exports::ExportNormalTrait,
    properties::{object_property::ObjectProperty, Property, PropertyDataTrait},
    types::PackageIndex,
    Import,
};
use unreal_mod_manager::unreal_helpers::Guid;
use unreal_mod_manager::unreal_mod_integrator::{
    helpers::{get_asset, write_asset},
    Error,
};
use unreal_mod_manager::unreal_pak::{PakMemory, PakReader};

use super::MAP_PATHS;

#[derive(Deserialize, Serialize, Debug)]
enum BiomeType {
    Surface,
    Crust,
}

#[derive(Deserialize, Serialize, Debug)]
struct PlacementModifier {
    pub planet_type: String,
    pub biome_type: BiomeType,
    pub biome_name: String,
    pub layer_name: String,
    pub placements: Vec<String>,
}

#[allow(clippy::ptr_arg)]
pub(crate) fn handle_biome_placement_modifiers(
    _data: &(),
    integrated_pak: &mut PakMemory,
    game_paks: &mut Vec<PakReader<BufReader<File>>>,
    mod_paks: &mut Vec<PakReader<BufReader<File>>>,
    placement_modifiers: &Vec<serde_json::Value>,
) -> Result<(), Error> {
    let mut biome_placement_modifiers = Vec::new();

    for modifiers in placement_modifiers {
        let modifiers: Vec<PlacementModifier> = serde_json::from_value(modifiers.clone())
            .map_err(|e| io::Error::new(ErrorKind::Other, e))?;

        biome_placement_modifiers.extend(modifiers);
    }

    for map_path in MAP_PATHS {
        if map_path == "Astro/Content/Maps/test/BasicSphereT2.umap" {
            continue;
        }
        let mut asset = get_asset(
            integrated_pak,
            game_paks,
            mod_paks,
            &map_path.to_string(),
            EngineVersion::VER_UE4_23,
        )?;

        let mut voxel_exports = HashMap::new();

        for i in 0..asset.asset_data.exports.len() {
            let export = &asset.asset_data.exports[i];
            if let Some(normal_export) = export.get_normal_export() {
                let class_index = normal_export.base_export.class_index;
                if class_index.is_import() {
                    let import = asset.get_import(class_index).ok_or_else(|| {
                        io::Error::new(ErrorKind::Other, "Corrupted game installation")
                    })?;

                    if import.object_name.get_owned_content() == "VoxelVolumeComponent"
                        && normal_export.base_export.object_name.get_owned_content()
                            != "Default Voxel Volume"
                    {
                        voxel_exports
                            .insert(normal_export.base_export.object_name.get_owned_content(), i);
                    }
                }
            }
        }

        for modifier in &biome_placement_modifiers {
            let mut modifier_imports = Vec::new();
            for placement_path in &modifier.placements {
                let placement_name = Path::new(placement_path)
                    .file_stem()
                    .and_then(|e| e.to_str())
                    .ok_or_else(|| {
                        io::Error::new(
                            ErrorKind::Other,
                            format!("Invalid placement {placement_path}"),
                        )
                    })?;

                let package_import = Import {
                    class_package: asset.add_fname("/Script/CoreUObject"),
                    class_name: asset.add_fname("Package"),
                    outer_index: PackageIndex::new(0),
                    object_name: asset.add_fname(placement_path),
                    optional: false,
                };
                let package_import = asset.add_import(package_import);

                let modifier_import = Import {
                    class_package: asset.add_fname("/Script/Terrain2"),
                    class_name: asset.add_fname("ProceduralModifier"),
                    outer_index: package_import,
                    object_name: asset.add_fname(placement_name),
                    optional: false,
                };
                let modifier_import = asset.add_import(modifier_import);
                modifier_imports.push(modifier_import);
            }

            let voxels_name = modifier.planet_type.clone() + "Voxels";
            let export_index = voxel_exports.get(&voxels_name);
            if export_index.is_none() {
                warn!(
                    "Failed to find voxel export {} for {}",
                    voxels_name, map_path
                );
                continue;
            }

            let export_index = export_index.unwrap();
            let mut name_map = asset.get_name_map();
            let export = (asset.asset_data.exports[*export_index])
                .get_normal_export_mut()
                .unwrap();

            let biome_property_name = match modifier.biome_type {
                BiomeType::Surface => "SurfaceBiomes",
                BiomeType::Crust => "CrustBiome",
            };

            let mut biome_property_index = None;
            for i in 0..export.properties.len() {
                let property = &export.properties[i];
                if property
                    .get_name()
                    .get_content(|e| e == biome_property_name)
                {
                    biome_property_index = Some(i);
                    break;
                }
            }

            let biome_property_index = biome_property_index.ok_or_else(|| {
                io::Error::new(
                    ErrorKind::Other,
                    format!("Failed to find biome type {biome_property_name}"),
                )
            })?;
            let biome_property = cast!(
                Property,
                ArrayProperty,
                &mut export.properties[biome_property_index]
            )
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "Corrupted game installation"))?;

            let biome = biome_property
                .value
                .iter_mut()
                .filter_map(|e| cast!(Property, StructProperty, e))
                .find(|e| {
                    e.value
                        .iter()
                        .filter_map(|e| cast!(Property, NameProperty, e))
                        .any(|e| e.value.get_content(|e| e == modifier.biome_name))
                })
                .ok_or_else(|| {
                    io::Error::new(
                        ErrorKind::Other,
                        format!("Failed to find biome {}", modifier.biome_name),
                    )
                })?;

            let layers = biome
                .value
                .iter_mut()
                .find(|e| e.get_name().get_content(|e| e == "Layers"))
                .and_then(|e| cast!(Property, ArrayProperty, e))
                .ok_or_else(|| io::Error::new(ErrorKind::Other, "Corrupted game installation"))?;

            let layer = layers
                .value
                .iter_mut()
                .filter_map(|e| cast!(Property, StructProperty, e))
                .find(|e| {
                    e.value
                        .iter()
                        .filter_map(|e| cast!(Property, NameProperty, e))
                        .any(|e| e.value.get_content(|e| e == modifier.layer_name))
                })
                .ok_or_else(|| {
                    io::Error::new(
                        ErrorKind::Other,
                        format!(
                            "Failed to find layer {} for biome {}",
                            modifier.layer_name, modifier.biome_name
                        ),
                    )
                })?;

            let object_placement_modifiers = layer
                .value
                .iter_mut()
                .find(|e| {
                    e.get_name()
                        .get_content(|e| e == "ObjectPlacementModifiers")
                })
                .and_then(|e| cast!(Property, ArrayProperty, e))
                .ok_or_else(|| {
                    io::Error::new(ErrorKind::Other, "Corrupted game installation".to_string())
                })?;

            for import_index in &modifier_imports {
                let placement_modifier = ObjectProperty {
                    name: name_map
                        .get_mut()
                        .add_fname(&object_placement_modifiers.value.len().to_string()),
                    ancestry: Ancestry::default(),
                    property_guid: Some(Guid::default()),
                    duplication_index: 0,
                    value: *import_index,
                };
                object_placement_modifiers
                    .value
                    .push(placement_modifier.into());
            }
        }

        write_asset(integrated_pak, &asset, &map_path.to_string())
            .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;
    }

    Ok(())
}
