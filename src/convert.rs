use std::{
    path::Path,
    fs::File,
    io::{BufReader, Error},
    mem, vec,
};

use hashbrown::HashMap;
use rayon::prelude::*; // Parallel iterator
use binrw::BinReaderExt;
use indicatif::{ProgressBar, ProgressStyle};


use crate::structure::anm::{NuccAnm, AnmEntry, AnmEntryFormat, AnmCurveFormat, Curve, CurveHeader, AnmClump};
use crate::structure::anmstrm::{NuccAnmStrm, NuccAnmStrmFrame, AnmStrmEntry, Entry};
use crate::structure::anm_utils::*;


const SCALE_COMPRESS: f32 = 4096.0;
const QUAT_COMPRESS: f32 = 32767.0;
const RGB_CONVERT: f32 = 255.0;


/// Converts ANMSTRM data into a vector of ANM data (ANM and DMG ANM)
pub fn convert_anmstrm(anmstrm: &NuccAnmStrm, mut anmstrm_frame_filepaths: Vec<String>) -> Result<Vec<NuccAnm>, Error> {
    anmstrm_frame_filepaths.sort_by(|a, b| {
        let a = Path::new(a).file_name().unwrap().to_str().unwrap();
        let b = Path::new(b).file_name().unwrap().to_str().unwrap();

        let a = a.split("_").collect::<Vec<&str>>()[1]
            .split(".")
            .collect::<Vec<&str>>()[0]
            .parse::<u32>()
            .unwrap();

        let b = b.split("_").collect::<Vec<&str>>()[1]
            .split(".")
            .collect::<Vec<&str>>()[0]
            .parse::<u32>()
            .unwrap();

        a.cmp(&b)
    });

    let anmstrmframes = parse_anmstrm_frames(anmstrm_frame_filepaths)?;
    let anmstrm_entries = build_anmstrm_entries_map(&anmstrmframes)?;

    let anm_entries = convert_entries(anmstrm_entries);


    println!("building anm files...");
    let mut anm = build_anm(anmstrm, anm_entries)?;
    let dmg_anm = build_dmg_anm(&mut anm, anmstrm); // Consumes the original anm to create the anm from the dmg clump and mutates the original anm

    let mut anms: Vec<NuccAnm> = Vec::with_capacity(2);
    
    anms.push(anm);
    anms.push(dmg_anm);
 

    Ok(anms)
}

/// Parses ANMSTRM frame files and returns a vector of parsed frames.
fn parse_anmstrm_frames(anmstrm_frame_filepaths: Vec<String>) -> Result<Vec<NuccAnmStrmFrame>, Error> {
    let mut anmstrmframes: Vec<NuccAnmStrmFrame> = Vec::with_capacity(anmstrm_frame_filepaths.len());

    let pb = ProgressBar::new(anmstrm_frame_filepaths.len() as u64);
    pb.set_style(ProgressStyle::with_template("parsing anmstrm...    {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
    .unwrap()
    .progress_chars("||-"));

    for (i,file) in anmstrm_frame_filepaths.iter().enumerate() {
        let mut buf = BufReader::new(File::open(file)?);
        anmstrmframes.push(buf.read_be::<NuccAnmStrmFrame>().unwrap());
        pb.set_message(format!("parsed #{}", i + 1));
        pb.inc(1);
    }
    pb.finish_with_message("done");
    
    Ok(anmstrmframes)
}

/// Builds entries from ANMSTRM frames and returns a vector of entries.
fn build_entries_from_frames(anmstrmframes: &Vec<NuccAnmStrmFrame>) -> Vec<Vec<AnmStrmEntry>> {
    let entry_count = anmstrmframes.first().map_or(0, |frame| frame.entry_count);

    let mut anmstrm_entries: Vec<Vec<AnmStrmEntry>> = vec![Vec::new(); entry_count as usize];

    anmstrm_entries.par_iter_mut().for_each(|entry| *entry = Vec::new());

    let pb = ProgressBar::new(anmstrmframes.len() as u64);
    pb.set_style(ProgressStyle::with_template("gathering frames...   {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
    .unwrap()
    .progress_chars("||-"));

    for (i, anmstrmframe) in anmstrmframes.iter().enumerate() {
        for (entry_index, entry) in anmstrmframe.entries.iter().enumerate() {
            match &entry.entry_data {
                Entry::Bone(_) | Entry::Material(_) | 
                Entry::Camera(_) | Entry::LightDirc(_) | 
                Entry::LightPoint(_) | Entry::Ambient(_)
                 => {
                    anmstrm_entries[entry_index].push(entry.clone());
                }
                _ => {}
            }    
        }

        pb.set_message(format!("frame #{}", i + 1));
        pb.inc(1);
    }

    pb.finish_with_message("done");

   
    anmstrm_entries
}

/// Builds a map of ANMSTRM entries, where the key is the entry index.
fn build_anmstrm_entries_map(anmstrmframes: &Vec<NuccAnmStrmFrame>) -> Result<HashMap<u16, Vec<AnmStrmEntry>>, Error> {
    let anmstrm_entries = build_entries_from_frames(anmstrmframes);

    Ok(anmstrm_entries
        .into_iter()
        .enumerate()
        .filter(|(_, entries)| !entries.is_empty())
        .map(|(entry_index, entries)| (entry_index.try_into().unwrap(), entries))
        .collect()) 
}

/// Converts ANMSTRM entries map into a vector of ANM entries.
fn convert_entries(anmstrm_entries: HashMap<u16, Vec<AnmStrmEntry>>) -> Vec<AnmEntry> {
    let mut anm_entries: Vec<AnmEntry> = Vec::with_capacity(anmstrm_entries.len());

    let pb = ProgressBar::new(anmstrm_entries.len() as u64);
    pb.set_style(ProgressStyle::with_template("converting entries... {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
    .unwrap()
    .progress_chars("||-"));

    for (i, entry) in &anmstrm_entries {
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

                // ----------------- BONE -----------------
                Entry::Bone(anmstrm_entry_bone) => {
                    anm_entry.entry_format = AnmEntryFormat::BONE as u16;

                    if frame == 0 {
                        // Create curves and curve headers for location, rotation, scale, and toggled
                        anm_entry.curves.push(Curve::KeyframeVector3(Vec::new()));
                        anm_entry.curves.push(Curve::QuaternionShort(Vec::new()));
                        anm_entry.curves.push(Curve::Vector3Short(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
    
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::INT1_FLOAT3 as u16, // Curve format for keyframe loc
                            frame_count: 0,
                            curve_size: 0,
                        });
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::SHORT4 as u16, // Curve format for rot
                            frame_count: 0,
                            curve_size: 0,
                        });
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 2,
                            curve_format: AnmCurveFormat::SHORT3 as u16, // Curve format for scale
                            frame_count: 0,
                            curve_size: 0,
                        });
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 3,
                            curve_format: AnmCurveFormat::FLOAT1 as u16, // Curve format for toggled
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
                    if let Curve::QuaternionShort(rotation_keyframes) = &mut anm_entry.curves[1] {
                        rotation_keyframes.push(QuaternionShort {
                            x: (anmstrm_entry_bone.rotation.x * QUAT_COMPRESS) as i16,
                            y: (anmstrm_entry_bone.rotation.y * QUAT_COMPRESS) as i16,
                            z: (anmstrm_entry_bone.rotation.z * QUAT_COMPRESS) as i16,
                            w: (anmstrm_entry_bone.rotation.w * QUAT_COMPRESS) as i16,
                            
                        });
                    }
                    if let Curve::Vector3Short(scale_keyframes) = &mut anm_entry.curves[2] {
                        scale_keyframes.push(Vector3Short {
                            x: (anmstrm_entry_bone.scale.x * SCALE_COMPRESS) as i16,
                            y: (anmstrm_entry_bone.scale.y * SCALE_COMPRESS) as i16,
                            z: (anmstrm_entry_bone.scale.z * SCALE_COMPRESS) as i16,
                        });
                    }

                    if let Curve::Float(toggled_value) = &mut anm_entry.curves[3] {
                        toggled_value.push(anmstrm_entry_bone.toggled);
                    }
                }
                
                // ----------------- MATERIAL -----------------
                Entry::Material(anmstrm_entry_material) => {
                    anm_entry.entry_format = AnmEntryFormat::MATERIAL as u16;

                    if frame == 0 {
                        anm_entry.curves.push(Curve::KeyframeFloat(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeFloat(Vec::new()));

                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));

                        anm_entry.curves.push(Curve::KeyframeFloat(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeFloat(Vec::new()));

                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new())); 
                        anm_entry.curves.push(Curve::Float(Vec::new()));

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::INT1_FLOAT1 as u16, // Curve format for KeyframeFloat
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::INT1_FLOAT1 as u16, // Curve format for KeyframeFloat
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 2,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 3,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 4,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 5,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 6,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 7,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 8,
                            curve_format: AnmCurveFormat::INT1_FLOAT1 as u16, // Curve format for KeyframeFloat
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 9,
                            curve_format: AnmCurveFormat::INT1_FLOAT1 as u16, // Curve format for KeyframeFloat
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index:  curve_index + 10,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                             curve_index: curve_index + 11,
                             curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                             frame_count: 0,
                             curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                             curve_index: curve_index + 12,
                             curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                             frame_count: 0,
                             curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                             curve_index: curve_index + 13,
                             curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                             frame_count: 0,
                             curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                             curve_index: curve_index + 14,
                             curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                             frame_count: 0,
                             curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                             curve_index: curve_index + 15,
                             curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                             frame_count: 0,
                             curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                             curve_index: curve_index + 16,
                             curve_format: AnmCurveFormat::FLOAT1 as u16, // Curve format for toggle
                             frame_count: 0,
                             curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                             curve_index: curve_index + 17,
                             curve_format: AnmCurveFormat::FLOAT1 as u16, // Curve format for toggle
                             frame_count: 0,
                             curve_size: 0,
                        });

                        curve_index += 18;
                    }

                    // Push keyframes for material color and toggled
                    if let Curve::KeyframeFloat(color_values) = &mut anm_entry.curves[0] {
                        color_values.push(KeyframeFloat {
                            frame: frame as i32,
                            value: anmstrm_entry_material.ambient_color[0],
                        });

                        color_values.push(KeyframeFloat {
                            frame: frame as i32 + 50,
                            value: anmstrm_entry_material.ambient_color[0],
                        });
                    }

                    if let Curve::KeyframeFloat(color_values) = &mut anm_entry.curves[1] {
                        color_values.push(KeyframeFloat {
                            frame: frame as i32,
                            value: anmstrm_entry_material.ambient_color[1],
                        });

                        color_values.push(KeyframeFloat {
                            frame: frame as i32 + 50,
                            value: anmstrm_entry_material.ambient_color[1],
                        });
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[2] {
                        color_values.push(anmstrm_entry_material.ambient_color[2]);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[3] {
                        color_values.push(anmstrm_entry_material.ambient_color[3]);
                    }


                    if let Curve::Float(color_values) = &mut anm_entry.curves[4] {
                        color_values.push(anmstrm_entry_material.ambient_color[4]);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[5] {
                        color_values.push(anmstrm_entry_material.ambient_color[5]);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[6] {
                        color_values.push(anmstrm_entry_material.ambient_color[6]);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[7] {
                        color_values.push(anmstrm_entry_material.ambient_color[7]);
                    }

                    if let Curve::KeyframeFloat(color_values) = &mut anm_entry.curves[8] {
                        color_values.push(KeyframeFloat {
                            frame: frame as i32,
                            value: anmstrm_entry_material.ambient_color[8],
                        });

                        color_values.push(KeyframeFloat {
                            frame: frame as i32 + 50,
                            value: anmstrm_entry_material.ambient_color[8],
                        });
                    }

                    if let Curve::KeyframeFloat(color_values) = &mut anm_entry.curves[9] {
                        color_values.push(KeyframeFloat {
                            frame: frame as i32,
                            value: anmstrm_entry_material.ambient_color[9],
                        });

                        color_values.push(KeyframeFloat {
                            frame: frame as i32 + 50,
                            value: anmstrm_entry_material.ambient_color[9],
                        });
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[10] {
                        color_values.push(anmstrm_entry_material.ambient_color[10]);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[11] {
                        color_values.push(anmstrm_entry_material.ambient_color[11]);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[12] {
                        color_values.push(anmstrm_entry_material.ambient_color[12]);
                    }


                    if let Curve::Float(color_values) = &mut anm_entry.curves[13] {
                        color_values.push(anmstrm_entry_material.ambient_color[13]);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[14] {
                        color_values.push(anmstrm_entry_material.ambient_color[14]);
                    }


                    if let Curve::Float(color_values) = &mut anm_entry.curves[15] {
                        color_values.push(anmstrm_entry_material.ambient_color[15]);
                    }


                    if let Curve::Float(color_values) = &mut anm_entry.curves[16] {
                        color_values.push(0.0);
                    }

                    if let Curve::Float(color_values) = &mut anm_entry.curves[17] {
                        color_values.push(1.0);
                    }                       
                }

                Entry::Camera(anmstrm_entry_camera) => {
                    anm_entry.entry_format = AnmEntryFormat::CAMERA as u16;

                    if frame == 0 {
                        // Create curves and curve headers for location, rotation, fov
                        anm_entry.curves.push(Curve::KeyframeVector3(Vec::new()));
                        anm_entry.curves.push(Curve::QuaternionShort(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeFloat(Vec::new()));

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::INT1_FLOAT3 as u16, // Curve format for KeyframeVector3
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::SHORT4 as u16, // Curve format for Short4
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
                    if let Curve::QuaternionShort(rotation_keyframes) = &mut anm_entry.curves[1] {
                        rotation_keyframes.push(QuaternionShort {
                            x: (anmstrm_entry_camera.rotation.x * QUAT_COMPRESS) as i16,
                            y: (anmstrm_entry_camera.rotation.y * QUAT_COMPRESS) as i16,
                            z: (anmstrm_entry_camera.rotation.z * QUAT_COMPRESS) as i16,
                            w: (anmstrm_entry_camera.rotation.w * QUAT_COMPRESS) as i16,
                        });
                    }
                    if let Curve::KeyframeFloat(fov_keyframes) = &mut anm_entry.curves[2] {
                        fov_keyframes.push(KeyframeFloat {
                            frame: frame as i32,
                            value: anmstrm_entry_camera.fov,
                        });
                    }
                }


                // ----------------- LIGHTDIRC -----------------
                Entry::LightDirc(anmstrm_entry_lightdir) => {
                    anm_entry.entry_format = AnmEntryFormat::LIGHTDIRC as u16;

                    if frame == 0 {
                        anm_entry.curves.push(Curve::RGB(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::QuaternionShort(Vec::new()));
                        
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
                            curve_format: AnmCurveFormat::SHORT4 as u16, // Curve format for KeyframeVector4
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                        curve_index += 3;
                    }
                    // Push keyframes for color, light strength, rotations
                    if let Curve::RGB(color_values) = &mut anm_entry.curves[0] {
                        color_values.push(RGB {
                            r: (anmstrm_entry_lightdir.color.x * RGB_CONVERT) as u8,
                            g: (anmstrm_entry_lightdir.color.y * RGB_CONVERT) as u8,
                            b: (anmstrm_entry_lightdir.color.z * RGB_CONVERT) as u8,
                        });     
                    }

                    if let Curve::Float(strength_values) = &mut anm_entry.curves[1] {
                        strength_values.push(anmstrm_entry_lightdir.intensity);
                    }

                    if let Curve::QuaternionShort(rotation_keyframes) = &mut anm_entry.curves[2] {
                        rotation_keyframes.push(QuaternionShort {
                            x: (anmstrm_entry_lightdir.direction.x * QUAT_COMPRESS) as i16,
                            y: (anmstrm_entry_lightdir.direction.y * QUAT_COMPRESS) as i16,
                            z: (anmstrm_entry_lightdir.direction.z * QUAT_COMPRESS) as i16,
                            w: (anmstrm_entry_lightdir.direction.w * QUAT_COMPRESS) as i16,
                        });
                    }
                }

                // ----------------- LIGHT POINT -----------------
                Entry::LightPoint(anm_entry_lightpoint) => {
                    anm_entry.entry_format = AnmEntryFormat::LIGHTPOINT as u16;

                    if frame == 0 {
                        // Create curves and curve headers for color, light strength, location
                        anm_entry.curves.push(Curve::RGB(Vec::new()));
                        anm_entry.curves.push(Curve::KeyframeVector3(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));

                        
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::BYTE3 as u16, // Curve format for RGB
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::INT1_FLOAT3 as u16, // Curve format for KeyframeVector3
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 2,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 3,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 4,
                            curve_format: AnmCurveFormat::FLOAT1ALT as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });
    
                        curve_index += 5;
                    }

                    // Push keyframes for color, light strength, location
                    if let Curve::RGB(color_values) = &mut anm_entry.curves[0] {
                        color_values.push(RGB {
                            r: (anm_entry_lightpoint.color.x * RGB_CONVERT) as u8,
                            g: (anm_entry_lightpoint.color.y * RGB_CONVERT) as u8,
                            b: (anm_entry_lightpoint.color.z * RGB_CONVERT) as u8,
                        });     
                    }

                    if let Curve::KeyframeVector3(location_keyframes) = &mut anm_entry.curves[1] {
                        location_keyframes.push(KeyframeVector3 {
                            frame: frame as i32,
                            value: anm_entry_lightpoint.position.clone(),
                        });
                    }

                    if let Curve::Float(intensity_values) = &mut anm_entry.curves[2] {
                        intensity_values.push(anm_entry_lightpoint.intensity);
                    }

                    if let Curve::Float(radius_values) = &mut anm_entry.curves[3] {
                        radius_values.push(anm_entry_lightpoint.radius);
                    }

                    if let Curve::Float(falloff_values) = &mut anm_entry.curves[4] {
                        falloff_values.push(anm_entry_lightpoint.falloff);
                    }
                }

                Entry::Ambient(anm_entry_ambient) => {
                    anm_entry.entry_format = AnmEntryFormat::AMBIENT as u16;

                    if frame == 0 {
                        // Create curves and curve headers for color, light strength
                        anm_entry.curves.push(Curve::RGB(Vec::new()));
                        anm_entry.curves.push(Curve::Float(Vec::new()));
                        
                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index,
                            curve_format: AnmCurveFormat::BYTE3 as u16, // Curve format for RGB
                            frame_count: 0,
                            curve_size: 0,
                        });

                        anm_entry.curve_headers.push(CurveHeader {
                            curve_index: curve_index + 1,
                            curve_format: AnmCurveFormat::FLOAT1ALT2 as u16, // Curve format for Float
                            frame_count: 0,
                            curve_size: 0,
                        });


                        curve_index += 2; 
                    }

                    // Push keyframes for color, light strength
                    if let Curve::RGB(color_values) = &mut anm_entry.curves[0] {
                        color_values.push(RGB {
                            r: (anm_entry_ambient.color.x * RGB_CONVERT) as u8,
                            g: (anm_entry_ambient.color.y * RGB_CONVERT) as u8,
                            b: (anm_entry_ambient.color.z * RGB_CONVERT) as u8,
                        });     
                    }

                    if let Curve::Float(strength_values) = &mut anm_entry.curves[1] {
                        strength_values.push(anm_entry_ambient.intensity);
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
        pb.set_message(format!("entry #{}", i + 1));
        pb.inc(1);
        
    }

    
    pb.finish_with_message("done");

    anm_entries
    
}

/// Builds an ANM object from ANMSTRM and converted ANM entries.
pub fn build_anm(anmstrm: &NuccAnmStrm, anm_entries: Vec<AnmEntry>) -> Result<NuccAnm, Error> {

    // sort the entries by coord index
    let mut anm_entries = anm_entries;
    anm_entries.sort_by(|a, b| a.coord.coord_index.cmp(&b.coord.coord_index));


    // We map the clumps to a new vector of clumps to avoid cloning the clumps
    let anm_clumps: Vec<AnmClump> = anmstrm.clumps.clone().into_par_iter().map(|clump| AnmClump {
        clump_index: clump.clump_index,
        bone_material_count: clump.bone_material_count,
        model_count: clump.model_count,
        bone_material_indices: clump.bone_material_indices,
        model_indices: clump.model_indices
    }).collect();

    let anm = NuccAnm {
        anm_length: anmstrm.anm_length,
        frame_size: anmstrm.frame_size,
        entry_count: anm_entries.len() as u16,
        looped: anmstrm.is_looped,
        clump_count: anmstrm.clump_count,
        other_entry_count: anmstrm.other_entry_count,
        other_index_count: anmstrm.other_index_count,
        coord_count: anmstrm.coord_count,
        clumps: anm_clumps,
        other_entries_indices: anmstrm.other_entries_indices.clone(),
        coord_parents: anmstrm.coord_parents.clone(),
        entries: anm_entries,
    };

    Ok(anm)
}

/// Builds a DMG ANM object from the ANM and ANMSTRM.
fn build_dmg_anm(anm: &mut NuccAnm, anmstrm: &NuccAnmStrm) -> NuccAnm {
    // ----------------- Clumps -----------------
    let mut clumps = anm.clumps.clone();

    let dmg_clump_index = clumps.par_iter().position_any(|clump| clump.bone_material_count == 97).unwrap_or(0);

    let dmg_clump = clumps[dmg_clump_index].clone();

    clumps.retain(|clump| clump.clump_index == dmg_clump.clump_index as u32); // Remove unnecessary clumps for the DMG ANM

    let subtractor = dmg_clump.clump_index as u32;

    for clump in &mut clumps {
        clump.clump_index -= subtractor;

        for index in &mut clump.bone_material_indices {
            *index -= subtractor;
        }
        for index in &mut clump.model_indices {
            *index -= subtractor;
        }
        
    }

    // ----------------- Coords ----------------- //
    let mut coord_parents = anm.coord_parents.clone();
    // Remove unnecessary coord parents for the DMG ANM
    coord_parents.retain(|coord_parent| coord_parent.parent.clump_index == dmg_clump_index as i16); // Only keep the coord parents related to the DMG clump

    // update indices for the DMG coord parents
    for coord_parent in &mut coord_parents {
        coord_parent.parent.clump_index -= dmg_clump_index as i16;
        coord_parent.child.clump_index -= dmg_clump_index as i16;
    }


    // ----------------- Entries ----------------- // 
    let mut dmg_entries: Vec<AnmEntry> = anm.entries.par_iter()
        .filter(|entry| entry.coord.clump_index == dmg_clump_index as i16)
        .cloned()
        .collect();

    for entry in &mut dmg_entries {
        entry.coord.clump_index -= dmg_clump_index as i16;
    }

    dmg_entries.sort_by(|a, b| a.coord.coord_index.cmp(&b.coord.coord_index));


    // -----------------DMG anm  ----------------- //
    let dmg_anm = NuccAnm {
        anm_length: anmstrm.anm_length,
        frame_size: anmstrm.frame_size,
        entry_count: dmg_entries.len() as u16,
        looped: anmstrm.is_looped,
        clump_count: clumps.len() as u16,
        other_entry_count: 0,
        other_index_count: anmstrm.other_index_count,
        coord_count: coord_parents.len() as u16,
        clumps: clumps,
        other_entries_indices: vec![],
        coord_parents: coord_parents,
        entries: dmg_entries,
    };


    // Mutate the original ANM
    anm.clumps.retain(|clump| clump.clump_index != dmg_clump.clump_index as u32); // Remove the DMG clump from the original ANM
    anm.clump_count = anm.clumps.len() as u16;

    // we need to edit the clump indices starting from the DMG clump index to the end
    for clump in &mut anm.clumps {
        // note we removed 98 indices + clump index so we need to adjust for that
        if clump.clump_index > dmg_clump.clump_index as u32 {
            clump.clump_index -= 99;

            for index in &mut clump.bone_material_indices {
                *index -= 99;
            }

            for index in &mut clump.model_indices {
                *index -= 99;
            }
        }
    }

    anm.coord_parents.retain(|coord_parent| coord_parent.parent.clump_index != dmg_clump_index as i16); // Remove the DMG coord parents from the original ANM
    anm.coord_count = anm.coord_parents.len() as u16;

    for coord in &mut anm.coord_parents {
            if coord.parent.clump_index > dmg_clump_index as i16 {
                coord.parent.clump_index -= 1;
                coord.child.clump_index -= 1;
            }
    }
     
    anm.entries.retain(|entry| entry.coord.clump_index != dmg_clump_index as i16); // Remove the DMG entries from the original ANM if the clump index matches the DMG clump index
    anm.entry_count = anm.entries.len() as u16;


    // we need to also edit the entry clump indices starting from the DMG clump index to the end
    for entry in &mut anm.entries {
        if entry.coord.clump_index > dmg_clump_index as i16 {
            entry.coord.clump_index -= 1;
        }

    }

    dmg_anm
}