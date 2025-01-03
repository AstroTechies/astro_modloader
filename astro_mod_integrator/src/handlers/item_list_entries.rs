use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, ErrorKind};
use std::path::Path;

use unreal_mod_manager::unreal_asset::properties::object_property::TopLevelAssetPath;
use unreal_mod_manager::unreal_asset::types::PackageIndexTrait;
use unreal_mod_manager::unreal_asset::unversioned::ancestry::Ancestry;
use unreal_mod_manager::unreal_asset::{
    cast,
    engine_version::EngineVersion,
    exports::{Export, ExportNormalTrait},
    properties::{
        object_property::{ObjectProperty, SoftObjectPath, SoftObjectProperty},
        Property,
    },
    types::PackageIndex,
    Import,
};
use unreal_mod_manager::unreal_helpers::game_to_absolute;
use unreal_mod_manager::unreal_mod_integrator::{
    helpers::{get_asset, write_asset},
    Error, IntegratorConfig,
};
use unreal_mod_manager::unreal_pak::{PakMemory, PakReader};

use crate::AstroIntegratorConfig;

#[allow(clippy::assigning_clones, clippy::ptr_arg)]
pub(crate) fn handle_item_list_entries(
    _data: &(),
    integrated_pak: &mut PakMemory,
    game_paks: &mut Vec<PakReader<BufReader<File>>>,
    mod_paks: &mut Vec<PakReader<BufReader<File>>>,
    item_list_entires_maps: &Vec<serde_json::Value>,
) -> Result<(), Error> {
    let mut new_items = HashMap::new();

    for item_list_entries_map in item_list_entires_maps {
        let item_list_entries_map = item_list_entries_map
            .as_object()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid item_list_entries"))?;

        // we duplicate /Game/Items/ItemTypes/MasterItemList entries into /Game/Items/ItemTypes/BaseGameInitialKnownItemList, if the latter list is not specified
        // this provides backwards compatibility for older mods
        // this can just be suppressed by specifying an entry for /Game/Items/ItemTypes/BaseGameInitialKnownItemList in metadata
        let exists_bgikil = item_list_entries_map.contains_key("/Game/Items/ItemTypes/BaseGameInitialKnownItemList");

        for (name, item_list_entries) in item_list_entries_map {
            let item_list_entries = item_list_entries
                .as_object()
                .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid item_list_entries"))?;

            {
                let new_items_entry = new_items.entry(name.clone()).or_insert_with(HashMap::new);

                for (item_name, entries) in item_list_entries {
                    let entries = entries
                        .as_array()
                        .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid item_list_entries"))?;
    
                    let new_items_entry_map = new_items_entry
                        .entry(item_name.clone())
                        .or_insert_with(Vec::new);
                    for entry in entries {
                        let entry = entry.as_str().ok_or_else(|| {
                            io::Error::new(ErrorKind::Other, "Invalid item_list_entries")
                        })?;
                        new_items_entry_map.push(String::from(entry));
                    }
                }
            }

            // duplicate to BaseGameInitialKnownItemList appropriately, if applicable
            if name == "/Game/Items/ItemTypes/MasterItemList" && !exists_bgikil {
                let orig_entry = new_items.entry(name.clone()).or_insert_with(HashMap::new).clone();

                new_items
                    .entry(String::from("/Game/Items/ItemTypes/BaseGameInitialKnownItemList"))
                    .or_insert_with(HashMap::new)
                    .extend(orig_entry);
            }
        }
    }

    for (asset_name, entries) in &new_items {
        let asset_name = game_to_absolute(AstroIntegratorConfig::GAME_NAME, asset_name)
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid asset name"))?;
        let mut asset = get_asset(
            integrated_pak,
            game_paks,
            mod_paks,
            &asset_name,
            EngineVersion::VER_UE4_23,
        )?;

        let mut item_types_property: HashMap<String, Vec<(usize, usize, String)>> = HashMap::new();
        for i in 0..asset.asset_data.exports.len() {
            if let Some(normal_export) = asset.asset_data.exports[i].get_normal_export() {
                for j in 0..normal_export.properties.len() {
                    let property = &normal_export.properties[j];
                    for entry_name in entries.keys() {
                        let mut arr_name = entry_name.clone();
                        if arr_name.contains('.') {
                            let split: Vec<&str> = arr_name.split('.').collect();
                            let export_name = split[0].to_owned();
                            arr_name = split[1].to_owned();

                            if normal_export.base_export.class_index.is_import() {
                                if asset
                                    .get_import(normal_export.base_export.class_index)
                                    .map(|e| e.object_name.get_content(|e| e != export_name))
                                    .unwrap_or(true)
                                {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }
                        if let Some(array_property) = cast!(Property, ArrayProperty, property) {
                            if array_property.name.get_content(|e| e == arr_name) {
                                item_types_property
                                    .entry(entry_name.clone())
                                    .or_default()
                                    .push((
                                        i,
                                        j,
                                        array_property
                                            .array_type
                                            .as_ref()
                                            .ok_or_else(|| {
                                                io::Error::new(
                                                    ErrorKind::Other,
                                                    "Invalid array_property",
                                                )
                                            })?
                                            .get_owned_content(),
                                    ));
                            }
                        }
                    }
                }
            }
        }
        for (name, item_paths) in entries {
            if !item_types_property.contains_key(name) {
                continue;
            }
            for item_path in item_paths {
                let (real_name, class_name, soft_class_name) = match item_path.contains('.') {
                    true => {
                        let split: Vec<&str> = item_path.split('.').collect();
                        (
                            split[0].to_string(),
                            split[1].to_string(),
                            split[1].to_string(),
                        )
                    }
                    false => (
                        item_path.clone(),
                        Path::new(item_path)
                            .file_stem()
                            .and_then(|e| e.to_str())
                            .map(|e| String::from(e) + "_C")
                            .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid item_path"))?,
                        Path::new(item_path)
                            .file_stem()
                            .and_then(|e| e.to_str())
                            .map(|e| e.to_string())
                            .ok_or_else(|| io::Error::new(ErrorKind::Other, "Invalid item_path"))?,
                    ),
                };

                let mut new_import = PackageIndex::new(0);

                for (export_index, property_index, array_type) in
                    item_types_property.get(name).unwrap()
                {
                    match array_type.as_str() {
                        "ObjectProperty" => {
                            if new_import.index == 0 {
                                let inner_import = Import {
                                    class_package: asset.add_fname("/Script/CoreUObject"),
                                    class_name: asset.add_fname("Package"),
                                    outer_index: PackageIndex::new(0),
                                    object_name: asset.add_fname(&real_name),
                                    optional: false,
                                };
                                let inner_import = asset.add_import(inner_import);

                                let import = Import {
                                    class_package: asset.add_fname("/Script/Engine"),
                                    class_name: asset.add_fname("BlueprintGeneratedClass"),
                                    outer_index: inner_import,
                                    object_name: asset.add_fname(&class_name),
                                    optional: false,
                                };
                                new_import = asset.add_import(import);
                            }

                            let export = cast!(
                                Export,
                                NormalExport,
                                &mut asset.asset_data.exports[*export_index]
                            )
                            .expect("Corrupted memory");
                            let property = cast!(
                                Property,
                                ArrayProperty,
                                &mut export.properties[*property_index]
                            )
                            .expect("Corrupted memory");
                            property.value.push(
                                ObjectProperty {
                                    name: property.name.clone(),
                                    ancestry: Ancestry::default(),
                                    property_guid: None,
                                    duplication_index: 0,
                                    value: new_import,
                                }
                                .into(),
                            );
                        }
                        "SoftObjectProperty" => {
                            asset.add_name_reference(real_name.clone(), false);

                            let asset_path_name = asset.add_fname(&real_name);

                            let export = cast!(
                                Export,
                                NormalExport,
                                &mut asset.asset_data.exports[*export_index]
                            )
                            .expect("Corrupted memory");
                            let property = cast!(
                                Property,
                                ArrayProperty,
                                &mut export.properties[*property_index]
                            )
                            .expect("Corrupted memory");
                            property.value.push(
                                SoftObjectProperty {
                                    name: property.name.clone(),
                                    ancestry: Ancestry::default(),
                                    property_guid: None,
                                    duplication_index: 0,
                                    value: SoftObjectPath {
                                        asset_path: TopLevelAssetPath::new(None, asset_path_name),
                                        sub_path_string: Some(soft_class_name.clone()),
                                    },
                                }
                                .into(),
                            );
                        }
                        _ => {}
                    }
                }
            }
        }

        write_asset(integrated_pak, &asset, &asset_name)
            .map_err(|e| io::Error::new(ErrorKind::Other, e.to_string()))?;
    }

    Ok(())
}
