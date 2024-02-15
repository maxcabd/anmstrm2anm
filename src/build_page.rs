use std::fmt::format;

use crate::structure::page::*;

pub fn build_anm_page(filepath: &str) -> Page {
    let mut page = Page::from_json_file(filepath);

    page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkMorphModel"));
    page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkAnmStrmFrame"));
    

    for map in page.chunk_maps.iter_mut() {
        if map.types.contains("nuccChunkAnmStrm") {
            map.types = "nuccChunkAnm".to_string();
        }
    }

    // Remove 1cmn references
    page.chunk_maps.retain(|chunk| !chunk.name.contains("1cmn"));
    page.chunk_references.retain(|reference| !reference.name.contains("1cmn"));

    page.files.retain(|file| !file.file_name.contains(".morphmodel"));
    page.files.retain(|file| !file.file_name.contains(".anmstrmframe"));

    for file in page.files.iter_mut() {
        if file.file_name.contains("anmstrm") {
            file.file_name = file.file_name.replace("anmstrm", "anm");
            file.chunk.types = "nuccChunkAnm".to_string();

        }
    }

    page
}


pub fn build_dmg_page(filepath: &str) -> Page {
    let mut dmg_page = Page::from_json_file(filepath);

    // remove morphmodel chunks
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkMorphModel"));
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkAnmStrmFrame"));
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkCamera"));
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkLightDirc"));
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkLightPoint"));
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkAmbient"));
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkLayerSet"));

    dmg_page.files.retain(|file| !file.file_name.contains(".morphmodel"));
    dmg_page.files.retain(|file| !file.file_name.contains(".anmstrmframe"));
    dmg_page.files.retain(|file| !file.file_name.contains(".camera"));
    dmg_page.files.retain(|file| !file.file_name.contains(".lightdirc"));
    dmg_page.files.retain(|file| !file.file_name.contains(".lightpoint"));
    dmg_page.files.retain(|file| !file.file_name.contains(".ambient"));
    dmg_page.files.retain(|file| !file.file_name.contains(".layerset"));

    for map in dmg_page.chunk_maps.iter_mut() {
        if map.types.contains("nuccChunkAnmStrm") {
            map.types = "nuccChunkAnm".to_string();
            map.name = format!("{}{}", map.name, "_dmg");
            map.path = map.path.replace(".max", "_dmg.max");
        }

        

    }

    for file in dmg_page.files.iter_mut() {
        if file.file_name.contains("anmstrm") {
            file.file_name = file.file_name.replace(".anmstrm", "_dmg.anm");
            file.chunk.types = "nuccChunkAnm".to_string();
            file.chunk.name = format!("{}{}", file.chunk.name, "_dmg");
            file.chunk.path = file.chunk.path.replace(".max", "_dmg.max");

        }
    }

    // remove nuccChunkAnmStrmFrame
    dmg_page.chunk_maps.retain(|chunk| !chunk.types.contains("nuccChunkAnmStrmFrame"));
    dmg_page.chunk_maps.retain(|chunk| chunk.name.contains("1cmn") 
    || chunk.types.contains("nuccChunkNull")
    || chunk.types.contains("nuccChunkAnm")
    || chunk.types.contains("nuccChunkIndex")
    || chunk.types.contains("nuccChunkPage"));



    
    dmg_page.chunk_references.retain(|reference| reference.name.contains("1cmn"));

    
    dmg_page
 
}