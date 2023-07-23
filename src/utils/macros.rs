

#[macro_export]
macro_rules! collect_files {
    ($dir:expr, $($ext:expr),+) => {{
        use std::fs;
        use std::path::{Path, PathBuf};

        let dir_path = Path::new($dir);

        let files: Vec<PathBuf> = fs::read_dir(dir_path)
            .expect("Failed to read directory!")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().ok().map(|ft| ft.is_file()).unwrap_or(false))
            .filter(|entry| {
                let ext = entry.path().extension().map(|ext| ext.to_string_lossy().to_lowercase());
                $(ext == Some($ext.to_string().to_lowercase()))||+
            })
            .map(|entry| entry.path())
            .collect();

        if files.is_empty() {
            // Don't make an error here, just return an empty vector
            Vec::new()
            //panic!("No {:#?} files found in directory: {}", &[$($ext),+], dir_path.display());
        } else {
            files
        }
    }};
}