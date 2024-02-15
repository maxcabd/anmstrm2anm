use std::{
    fs::{self, File},
    path::Path,
};
use binrw::{BinReaderExt, BinWriterExt};
use console::Emoji;

mod structure;
mod utils;
mod convert;
mod build_page;

use crate::{
    structure::anmstrm::NuccAnmStrm,
    utils::macros::find_subfolder,
};

use convert::*;
use build_page::*;

static SPARKLE: Emoji<'_, '_> = Emoji("✨ ", ":-)");

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let xfbin_dir = &args[1];
    let working_dir = std::env::current_dir().unwrap();
  

    println!("anmstr2anm v0.1.0");
    println!("by dei");

    let new_xfbin_dir = format!("{}\\{}_converted", working_dir.display(), Path::new(xfbin_dir).file_name().unwrap().to_str().unwrap());
     
    let chunk_name = Path::new(&xfbin_dir)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();

    let anmstrm_dir = find_subfolder(xfbin_dir, "(nuccChunkAnmStrmFrame)")
        .expect("No nuccChunk found in the directory.");

    let anmstrm: NuccAnmStrm = File::open(&collect_files!(anmstrm_dir.as_str(), "anmstrm")[0])
        .unwrap()
        .read_be::<NuccAnmStrm>()
        .unwrap();

    let now = std::time::Instant::now();

    let anmstrm_frame_filepaths = collect_files!(&anmstrm_dir, "anmstrmframe");
    let anms = convert_anmstrm(&anmstrm, anmstrm_frame_filepaths);

    let elapsed = now.elapsed().as_secs_f32();

    for (i, anm) in anms.unwrap().iter().enumerate() {
        let suffix = if i == 1 { "_dmg" } else { "" };

        let anm_path = format!(
            "{}\\[00{}] {}{} (nuccChunkAnm)",
            new_xfbin_dir, i, chunk_name, suffix
        );

        fs::create_dir_all(&anm_path).expect("Failed to create anm directory");

        let anm_file_path = format!("{}\\{}{}.anm", anm_path, chunk_name, suffix);
        let anm_file_path = Path::new(&anm_file_path);

        let mut buf_writer = std::io::BufWriter::new(File::create(anm_file_path).unwrap());

        buf_writer.write_be(anm).expect("Failed to write anm file");
    }

    // Copy other files
    let other_entries = collect_files!(
        anmstrm_dir.as_str(),
        "camera",
        "lightdirc",
        "lightpoint",
        "ambient",
        "layerset",
        "json"
    );

    let main_anm_path = format!("{}\\[000] {} (nuccChunkAnm)", new_xfbin_dir, chunk_name);

    for other_entry_path in &other_entries {
        let dest = Path::new(&main_anm_path).
        join(Path::new(other_entry_path)
        .file_name()
        .unwrap());

        if let Err(err) = fs::copy(&other_entry_path, &dest) {
            eprintln!("Error copying file: {}", err);
        }
    }

    // Build pages
    let anm_page = build_anm_page(&collect_files!(anmstrm_dir.as_str(), "json")[0]);
    anm_page.to_json_file(format!("{}\\_page.json", main_anm_path).as_str());

    let dmg_page = build_dmg_page(&collect_files!(anmstrm_dir.as_str(), "json")[0]);
    let dmg_path = format!("{}\\[001] {}_dmg (nuccChunkAnm)", new_xfbin_dir, chunk_name);
    dmg_page.to_json_file(format!("{}\\_page.json", dmg_path).as_str());

    println!("{} Done converting anmstrm {} to anm in {}s", SPARKLE, chunk_name, elapsed);
    std::thread::sleep(std::time::Duration::from_secs(4));
}
