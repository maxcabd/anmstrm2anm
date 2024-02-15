use std::fs;
use serde::{Deserialize, Serialize};
use regex::Regex;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chunk {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Type")]
    pub types: String,
    #[serde(rename = "Path")]
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChunkMap {
    #[serde(rename = "Chunk Maps")]
    pub chunk_maps: Vec<Chunk>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChunkReference {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Chunk")]
    pub chunk: Chunk,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Files {
    #[serde(rename = "File Name")]
    pub file_name: String,
    #[serde(rename = "Chunk")]
    pub chunk: Chunk,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Page {
    #[serde(rename = "Chunk Maps")]
    pub chunk_maps: Vec<Chunk>,
    #[serde(rename = "Chunk References")]
    pub chunk_references:  Vec<ChunkReference>,
    #[serde(rename = "Chunks")]
    pub files: Vec<Files>
}

impl Page {
    pub fn from_json_file(filepath: &str) -> Page {
        let json_result = fs::read(filepath);
    
        let json = match json_result {
            Ok(data) => String::from_utf8_lossy(&data).to_string(),
            Err(e) => {
                eprintln!("Failed to read JSON file: {}", e);

                String::new()

            }
        };
    
        let modified_json = replace_commas_in_strings(&json);

        serde_json::from_str(&modified_json).expect("Failed to deserialize JSON")
    }
    
    pub fn to_json_file(&self, filepath: &str) {
        fs::File::create(filepath).unwrap();

        let json = serde_json::to_string_pretty(&self).unwrap();
        
        fs::write(filepath, json).unwrap();        
    }

}
fn replace_commas_in_strings(json: &str) -> String {
    let re = Regex::new(r#""[^"]*""#).expect("Failed to create regex");

    let modified_json = re.replace_all(json, |caps: &regex::Captures| {
        caps[0].replace(",", "")

    });
    // Then replace the weird character with a space
    modified_json.replace("�", " ").to_string()
}