use serde::{Deserialize, Serialize};



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
    pub fn from_json_file(file_path: &str) -> Page {
        let json_str = std::fs::read_to_string(file_path).unwrap();
        serde_json::from_str(&json_str).unwrap()
      
    }

    pub fn to_json_file(&self, file_path: &str) {
        let json_str = serde_json::to_string_pretty(&self).unwrap();
        std::fs::write(file_path, json_str).unwrap();
    }

}
