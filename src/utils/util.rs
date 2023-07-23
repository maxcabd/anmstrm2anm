use binrw::{binrw};



#[binrw]
#[derive(Debug, Clone)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct Vector3Short {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct KeyframeVector3 {
    pub frame: i32,
    pub value: Vector3,
}


#[binrw]
#[derive(Debug, Clone)]
pub struct KeyframeVector4 {
    pub frame: i32,
    pub value: Vector4,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct KeyframeFloat {
    pub frame: i32,
    pub value: f32,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct QuaternionShort {
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub w: i16,
}

#[binrw]
#[derive(Debug, Clone)]
pub struct RGB {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}