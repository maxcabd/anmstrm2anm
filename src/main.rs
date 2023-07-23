use std::fs::File;
use std::{mem, vec};
use std::time::Instant;
use std::collections::BTreeMap;
use binrw::{BinReaderExt, BinWriterExt};

mod structure;
mod utils;

use crate::structure::page::*;
use crate::structure::anm::{NuccAnm, AnmClump, AnmEntry, AnmEntryFormat, AnmCurveFormat, Curve, CurveHeader};
use crate::structure::anmstrm::{NuccAnmStrm, NuccAnmStrmFrame, AnmStrmEntry, Entry};
use crate::utils::util::*;


fn main() {
    // TODO: Average the time it takes to convert anmstrm to 20.8181299s, a bit slow, lower time if possible
    
    let now = Instant::now();
    let mut anmstrm_file = File::open("C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10\\d18_10.anmstrm").unwrap();
    let anmstrm = anmstrm_file.read_be::<NuccAnmStrm>().unwrap();

    let anmstrm_path = "C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10"; // Use args for this later

    make_anm_page("C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10\\_page.json", "C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10\\_page2.json");
    let other_entry_files: Vec<_> = collect_files!(anmstrm_path, "camera", "lightdirc", "lightpoint", "ambient");
    println!("{:?}", other_entry_files);

    // Collect anmstrmframes filepaths into a vector
    let anmstrmframe_files: Vec<_> = collect_files!(anmstrm_path, "anmstrmframe");
    let anmstrmframes: Vec<NuccAnmStrmFrame> = anmstrmframe_files
    .iter()
    .map(|anmstrmframe_file| File::open(anmstrmframe_file).unwrap().read_be::<NuccAnmStrmFrame>().unwrap())
    .collect();

    let anm = make_anm(anmstrm, anmstrmframes);
    


    let mut anm_file = File::create("C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10.anm").unwrap();
    let mut dmg_anm_file = File::create("C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10_dmg\\d18_10_dmg.anm").unwrap();
    let dmg_anm = make_dmg_anm("C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10_dmg\\_page.json", anm.clone());
    dmg_anm_file.write_be(&dmg_anm).unwrap();
    anm_file.write_be(&anm).unwrap();

    println!("anmstrm converted in: {:?}", now.elapsed());

}

fn make_dmg_anm(page_path: &str, anm: NuccAnm) -> NuccAnm { //TODO: Add original anmstrm as a parameter
    let anmstrm_page = Page::from_json_file(page_path);

    // Get index of 1cmnbod1 clump in chunk references
    let clump_ref_index = anmstrm_page.chunk_references.iter().position(|chunk_ref| {
        chunk_ref.chunk.path.contains("c\\1cmn\\max\\1cmnbod1.max") && chunk_ref.chunk.types.contains("nuccChunkClump")
    }).unwrap_or(0) as u32;

    // Filter entries and coord_parents by clump_index
    let clump_index = anm.clumps.iter().position(|x| x.clump_index == clump_ref_index).unwrap_or(0);
    let mut dmg_anm = NuccAnm {
        anm_length: anm.anm_length,
        frame_size: anm.frame_size,
        entry_count: 0,
        looped: anm.looped,
        clumps: vec![anm.clumps[clump_index].clone()],
        clump_count: 1,
        other_entry_count: 0,
        other_entries: anm.other_entries,
        coord_count: anm.coord_count,
        coord_parents: anm.coord_parents.iter().filter(|coord_parent| coord_parent.parent.clump_index == clump_index as i16).cloned().collect(),
        entries: anm.entries.iter().filter(|entry| entry.coord.clump_index == clump_index as i16).cloned().collect(),

    };


    // TODO: Get the original coord parents, clumps, from the original anmstrm file. 
    dmg_anm.entry_count = dmg_anm.entries.len() as u16;
    dmg_anm.coord_count = dmg_anm.coord_parents.len() as u32;

  

    dmg_anm

}

fn make_anm_page(page_path: &str, output_path: &str) {
    let mut anm_page = Page::from_json_file(page_path);

    // Remove unnecessary chunk types and files
    anm_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkAnmStrmFrame"));
    anm_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkLayerSet"));
    anm_page.files.retain(|file| !file.file_name.contains(".anmstrmframe"));
    anm_page.files.retain(|file| !file.file_name.contains(".layerset"));

    // Change the chunk type for the anmstrm file to anm
    for chunk in &mut anm_page.chunk_maps {
        if chunk.types.contains("nuccChunkAnmStrm") {
            chunk.types = "nuccChunkAnm".to_string();
        }
    }

    // Change the chunk type for the anmstrmframe files to anm
    for file in &mut anm_page.files {
        if file.chunk.types.contains("nuccChunkAnmStrm") {
            file.chunk.types = "nuccChunkAnm".to_string();
        }
        if file.file_name.contains("anmstrm") {
            file.file_name = file.file_name.replace("anmstrm", "anm");
        }
    }

    anm_page.to_json_file(output_path);

    // Create a new page for dmg with necessary chunks and references
    let mut dmg_page = Page {
        chunk_maps: anm_page.chunk_maps.clone(),
        chunk_references: anm_page.chunk_references.clone(),
        files: anm_page.files.clone(),
    };

    dmg_page.chunk_maps.retain(|chunk| chunk.path.contains("c\\1cmn\\max\\1cmnbod1.max"));
    dmg_page.chunk_references.retain(|chunk_ref| chunk_ref.chunk.path.contains("c\\1cmn\\max\\1cmnbod1.max"));

    dmg_page.chunk_maps.insert(0, Chunk {
        name: "".to_string(),
        types: "nuccChunkNull".to_string(),
        path: "".to_string(),
    });

    // Remove unnecessary files from dmg page
    dmg_page.files.retain(|file| !file.file_name.contains(".camera"));
    dmg_page.files.retain(|file| !file.file_name.contains(".lightdirc"));
    dmg_page.files.retain(|file| !file.file_name.contains(".lightpoint"));
    dmg_page.files.retain(|file| !file.file_name.contains(".ambient"));

    // Insert the necessary chunk into dmg page
    dmg_page.chunk_maps.insert(1, dmg_page.files[0].chunk.clone());


    dmg_page.to_json_file("C:\\Users\\User\\Desktop\\Projects\\Rust\\anmstrm2anm\\src\\files\\d18_10_dmg\\_page.json");
}



pub fn get_anmstrm_entries(anmstrmframes: &[NuccAnmStrmFrame], entry_count: u16) -> BTreeMap<u16, Vec<AnmStrmEntry>> {
    let mut bone_entries: Vec<Vec<AnmStrmEntry>> = Vec::new();
    let mut material_entries: Vec<Vec<AnmStrmEntry>> = Vec::new();
    let mut camera_entries: Vec<Vec<AnmStrmEntry>> = Vec::new();
    let mut lightdirc_entries: Vec<Vec<AnmStrmEntry>> = Vec::new();

    for _ in 0..entry_count as usize {
        bone_entries.push(Vec::new());
        material_entries.push(Vec::new());
        camera_entries.push(Vec::new());
        lightdirc_entries.push(Vec::new());
    }

    for anmstrmframe in anmstrmframes {
        for (entry_index, entry) in anmstrmframe.entries.iter().enumerate() {
            match &entry.entry_data {
                Entry::Bone(_) => bone_entries[entry_index].push(entry.clone()),
                Entry::Material(_) => material_entries[entry_index].push(entry.clone()),
                Entry::Camera(_) => camera_entries[entry_index].push(entry.clone()),
                Entry::LightDirc(_) => lightdirc_entries[entry_index].push(entry.clone()),
                _ => {}
            }
        }
    }

    let mut anmstrm_entries: BTreeMap<u16, Vec<AnmStrmEntry>> = BTreeMap::new();

    for (entry_index, entries) in bone_entries.iter().enumerate() {
        if !entries.is_empty() {
            anmstrm_entries.insert(entry_index.try_into().unwrap(), entries.clone());
        }
    }

    for (entry_index, entries) in material_entries.iter().enumerate() {
        if !entries.is_empty() {
            anmstrm_entries.insert(entry_index.try_into().unwrap(), entries.clone());
        }
    }

    for (entry_index, entries) in camera_entries.iter().enumerate() {
        if !entries.is_empty() {
            anmstrm_entries.insert(entry_index.try_into().unwrap(), entries.clone());
        }
    }

    for (entry_index, entries) in lightdirc_entries.iter().enumerate() {
        if !entries.is_empty() {
            anmstrm_entries.insert(entry_index.try_into().unwrap(), entries.clone());
        }
    }

    anmstrm_entries
}



pub fn make_anm_entries(anmstrm_entries: BTreeMap<u16, Vec<AnmStrmEntry>>) -> Vec<AnmEntry> {
    let mut anm_entries: Vec<AnmEntry> = Vec::new();

    for (_index, entry) in &anmstrm_entries {
        let mut anm_entry = AnmEntry {
            coord: entry[0].coord.clone(),
            entry_format: 0,
            curve_count: 0,
            curve_headers: Vec::new(),
            curves: Vec::new(),
        };

        let mut curve_index = 0; // Index for the curve headers

        for (frame, anmstrm_entry) in entry.iter().enumerate().map(|(frame, entry)| (frame * 100, entry)) {
            match &anmstrm_entry.entry_data {
                Entry::Bone(anmstrm_entry_bone) => {
                    anm_entry.entry_format = AnmEntryFormat::BONE as u16;

                    if frame == 0 {
                        // Create curves and curve headers for location, rotation, scale, and toggled
                        anm_entry.curves.push(Curve::KeyframeVector3(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeVector4(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeVector3(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
    

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::INT1_FLOAT3 as u16, // Curve format for KeyframeVector3
                            frame_count: 0,
                            curve_size: 0,
                        });
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::INT1_FLOAT4 as u16, // Curve format for KeyframeVector4
                            frame_count: 0,
                            curve_size: 0,
                        });
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 2,
                            curve_format: AnmCurveFormat::INT1_FLOAT3 as u16, // Curve format for KeyframeVector3
                            frame_count: 0,
                            curve_size: 0,
                        });
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 3,
                            curve_format: AnmCurveFormat::FLOAT1 as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                       curve_index += 4;
                    }
                    // Push keyframes for location, rotation, scale, and toggled
                    if let Curve::KeyframeVector3(location_keyframes) = &mut anm_entry.curves[0] {
                        location_keyframes.push(KeyframeVector3 {
                            frame: frame as i32,
                            value: anmstrm_entry_bone.location.clone(),
                        });
                    }
                    if let Curve::KeyframeVector4(rotation_keyframes) = &mut anm_entry.curves[1] {
                        rotation_keyframes.push(KeyframeVector4 {
                            frame: frame as i32,
                            value: anmstrm_entry_bone.rotation.clone(),
                        });
                    }
                    if let Curve::KeyframeVector3(scale_keyframes) = &mut anm_entry.curves[2] {
                        scale_keyframes.push(KeyframeVector3 {
                            frame: frame as i32,
                            value: anmstrm_entry_bone.scale.clone(),
                        });
                    }
                    // Set toggled value for toggled curve, we only need 1 toggle curve
                    if frame == 0 {
                        if let Curve::Float(toggled_value) = &mut anm_entry.curves[3] {
                            *toggled_value = vec![anmstrm_entry_bone.toggled];
                        }
                    }
                }
                Entry::Material(anmstrm_entry_material) => {
                    anm_entry.entry_format = AnmEntryFormat::MATERIAL as u16;

                    if frame == 0 {
                        // Create a curve for each ambient color value
                        for &ambient_color in &anmstrm_entry_material.ambient_color {
                            anm_entry.curves.push(Curve::Float(vec![ambient_color]));
                        }

                        // Create curve headers for each curve
                        for (curve_index, curve) in anm_entry.curves.iter().enumerate() {
                            anm_entry.curve_headers.push(CurveHeader {
                                curve_index: curve_index as u16,
                                curve_format: AnmCurveFormat::FLOAT1 as u16, // Curve format for Float
                                frame_count: curve.get_frame_count() as u16,
                                curve_size: 0,
                            });
                        }
                    }
                }
                Entry::Camera(anmstrm_entry_camera) => {
                    anm_entry.entry_format = AnmEntryFormat::CAMERA as u16;

                    if frame == 0 {
                        // Create curves and curve headers for location, rotation, fov
                        anm_entry.curves.push(Curve::KeyframeVector3(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeVector4(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeFloat(Vec::new()));

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::INT1_FLOAT3 as u16, // Curve format for KeyframeVector3
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::INT1_FLOAT4 as u16, // Curve format for KeyframeVector4
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 2,
                            curve_format: AnmCurveFormat::INT1_FLOAT1 as u16, // Curve format for KeyframeFloat
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                        curve_index += 3;
                    }
                    // Push keyframes for location, rotation, fov
                    if let Curve::KeyframeVector3(location_keyframes) = &mut anm_entry.curves[0] {
                        location_keyframes.push(KeyframeVector3 {
                            frame: frame as i32,
                            value: anmstrm_entry_camera.location.clone(),
                        });
                    }
                    if let Curve::KeyframeVector4(rotation_keyframes) = &mut anm_entry.curves[1] {
                        rotation_keyframes.push(KeyframeVector4 {
                            frame: frame as i32,
                            value: anmstrm_entry_camera.rotation.clone(),
                        });
                    }
                    if let Curve::KeyframeFloat(fov_keyframes) = &mut anm_entry.curves[2] {
                        fov_keyframes.push(KeyframeFloat {
                            frame: frame as i32,
                            value: anmstrm_entry_camera.fov,
                        });
                    }
                }
                Entry::LightDirc(anmstrm_entry_lightdir) => {
                    anm_entry.entry_format = AnmEntryFormat::LIGHTDIRC as u16;

                    if frame == 0 {
                        // Create curves and curve headers for color, light strength, rotations
                        anm_entry.curves.push(Curve::RGB(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeVector4(Vec::new()));
                        
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::BYTE3 as u16, // Curve format for RGB
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 2,
                            curve_format: AnmCurveFormat::INT1_FLOAT4 as u16, // Curve format for KeyframeVector4
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                        curve_index += 3;
                    }
                    // Push keyframes for color, light strength, rotations
                    if let Curve::RGB(color_values) = &mut anm_entry.curves[0] {
                        color_values.push(RGB {
                            r: (anmstrm_entry_lightdir.color.x * 255.0) as u8,
                            g: (anmstrm_entry_lightdir.color.y * 255.0) as u8,
                            b: (anmstrm_entry_lightdir.color.z * 255.0) as u8,
                        });     
                    }
                    if let Curve::Float(strength_values) = &mut anm_entry.curves[1] {
                        strength_values.push(anmstrm_entry_lightdir.intensity);
                    }
                    if let Curve::KeyframeVector4(rotation_keyframes) = &mut anm_entry.curves[2] {
                        rotation_keyframes.push(KeyframeVector4 {
                            frame: frame as i32,
                            value: anmstrm_entry_lightdir.direction.clone(),
                        });
                    }
                }
                _ => {
                    // Handle other entry types if necessary
                    // ...
                }     
            }  
        }
        // Update curves and headers for necessary changes
        for (curve, curve_header) in anm_entry.curves.iter_mut().zip(&mut anm_entry.curve_headers) {
            if curve.has_keyframes() {
                curve.append_null_keyframe();
                curve_header.frame_count += 1;
            }
            //If curve is RBG we need to pad the color values to be a multiple of 4
            if curve.get_curve_format() == AnmCurveFormat::BYTE3 as u16 {
                curve.pad_color_values();    
            }
            // Make sure we update the frame count and curve size for the curves
            curve_header.curve_size += mem::size_of_val(&curve) as u16;
            curve_header.frame_count = curve.get_frame_count() as u16;
        }

        anm_entry.curve_count = anm_entry.curves.len() as u16;
        anm_entries.push(anm_entry);
    }
    anm_entries
}


pub fn make_anm(anmstrm: NuccAnmStrm, anmstrmframes: Vec<NuccAnmStrmFrame>) -> NuccAnm {

    let anm_clumps: Vec<AnmClump> = anmstrm.clumps.into_iter().map(|clump| AnmClump {
        clump_index: clump.clump_index,
        bone_material_count: clump.bone_material_count,
        model_count: clump.model_count,
        bone_material_indices: clump.bone_material_indices,
        model_indices: clump.model_indices
    }).collect();

    let anm_entries = make_anm_entries(get_anmstrm_entries(
        &anmstrmframes,
        anmstrmframes.first().map(|frame| frame.entry_count).unwrap_or(0),
    ));

    let anm = NuccAnm {
        anm_length: anmstrm.anm_length,
        frame_size: anmstrm.frame_size,
        entry_count: anm_entries.len() as u16,
        looped: anmstrm.is_looped,
        clump_count: anmstrm.clump_count,
        other_entry_count: anmstrm.other_entry_count,
        coord_count: anmstrm.coord_count,
        clumps: anm_clumps,
        other_entries: anmstrm.other_entries_indices,
        coord_parents: anmstrm.coord_parents,
        entries: anm_entries
    };

    anm
}
