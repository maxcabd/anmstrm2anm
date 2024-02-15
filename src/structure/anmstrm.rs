use binrw::binrw;

use crate::structure::anm::{CoordParent, AnmCoord};
use crate::structure::anm_utils::*;

#[binrw]
#[derive(Debug)]
pub struct NuccAnmStrm {
    pub anm_length: u32,
    pub frame_size: u32,
    pub frame_count: u16,
    pub is_looped: u16,
    pub clump_count: u16,
    pub other_entry_count: u16,
    pub other_index_count: u16,
    pub coord_count: u16,

    #[br(count = clump_count)]
    pub clumps: Vec<AnmStrmClump>,

    #[br(count = other_entry_count + other_index_count)]
    pub other_entries_indices: Vec<u32>, 

    #[br(count = coord_count)]
    pub coord_parents: Vec<CoordParent>,

    #[br(count = frame_count)]
    pub frames: Vec<AnmStrmFrameInfo>
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmStrmClump {
    pub clump_index: u32,
    pub bone_material_count: u16,
    pub model_count: u16,

    #[br(count = bone_material_count)]
    pub bone_material_indices: Vec<u32>,
    
    #[br(count = model_count)]
    pub model_indices: Vec<u32>,

    #[br(count = model_count)]
    pub unknown: Vec<u32>
}

#[binrw]
#[derive(Debug)]
pub struct AnmStrmFrameInfo {
    pub frame_offset: u32,
    pub frame_number: u16
}


#[binrw]
#[derive(Debug, Clone)]
pub struct NuccAnmStrmFrame {
    pub frame_number: u32,
    pub entry_count: u16,
    pub unknown: u16,

    #[br(count = entry_count)]
    pub entries: Vec<AnmStrmEntry>
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmStrmEntry {
    pub coord: AnmCoord,
    pub entry_format: u16,
    pub entry_size: u16,

    #[br(args(entry_format))]
    pub entry_data: Entry
}

#[binrw]
#[derive(Debug, Clone)]
#[br(import(entry_format: u16))]
pub enum Entry {
    #[br(pre_assert(entry_format == 1))]
    Bone(AnmEntryBone),

    #[br(pre_assert(entry_format == 2))]
    Camera(AnmEntryCamera),

    #[br(pre_assert(entry_format == 4))]
    Material(AnmEntryMaterial),

    #[br(pre_assert(entry_format == 5))]
    LightDirc(AnmEntryLightDirc),

    #[br(pre_assert(entry_format == 6))]
    LightPoint(AnmEntryLightPoint),

    #[br(pre_assert(entry_format == 8))]
    Ambient(AnmEntryAmbient),

    #[br(pre_assert(entry_format == 12))]
    MorphModel(AnmEntryMorphModel),


    #[br(pre_assert(false))]
    Unknown
}



#[binrw]
#[derive(Debug, Clone)]
pub struct AnmEntryBone {
    pub frame_count: i32,
    pub location: Vector3,
    pub rotation: Vector4,
    pub scale: Vector3,
    pub toggled: f32
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmEntryCamera {
    pub frame_count: i32,
    pub location: Vector3,
    pub rotation: Vector4,
    pub fov: f32,
    pub scale: Vector3
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmEntryMaterial {
    pub frame_count: i32,
    pub ambient_color: [f32; 16]
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmEntryLightDirc {
    pub frame_count: i32,
    pub color: Vector3,
    pub intensity: f32,
    pub direction: Vector4,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmEntryLightPoint {
    pub frame_count: i32,
    pub color: Vector3,
    pub position: Vector3,
    pub intensity: f32,
    pub radius: f32,
    pub falloff: f32
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmEntryAmbient {
    pub frame_count: i32,
    pub color: Vector3,
    pub intensity: f32
}

#[binrw]
#[derive(Debug, Clone)]
pub struct AnmEntryMorphModel {
    pub frame_count: i32,
    #[br(count = frame_count)]
    pub morph_weight: Vec<f32>,
}