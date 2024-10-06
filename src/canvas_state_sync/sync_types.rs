// use eframe::egui::ahash::HashMap;
use std::collections::HashMap;
// use eframe::emath::TSTransform;
use serde::{Deserialize, Serialize};

use crate::canvas_app::App;

// use crate::canvas_image::CanvasImageData;

#[derive(Serialize, Deserialize)]
pub struct ChunkedMessage {
    pub id: u64,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub data: Vec<u8>,
}

pub struct ChunkCollector {
    chunks: HashMap<u64, Vec<Option<Vec<u8>>>>, // Mapping message ID -> list of chunks
    chunk_sizes: HashMap<u64, u32>,             // Mapping message ID -> total number of chunks
}

impl ChunkCollector {
    pub fn new() -> Self {
        ChunkCollector {
            chunks: HashMap::new(),
            chunk_sizes: HashMap::new(),
        }
    }

    pub fn add_chunk(&mut self, id: u64, chunk_index: u32, total_chunks: u32, data: Vec<u8>) {
        // If it's a new message, initialize storage
        self.chunk_sizes.entry(id).or_insert(total_chunks);
        let chunk_list = self
            .chunks
            .entry(id)
            .or_insert_with(|| vec![None; total_chunks as usize]);

        // Store the chunk
        chunk_list[chunk_index as usize] = Some(data);
    }

    pub fn is_complete(&self, id: u64) -> bool {
        if let Some(chunk_list) = self.chunks.get(&id) {
            return chunk_list.iter().all(|chunk| chunk.is_some());
        }
        false
    }

    pub fn reassemble(&self, id: u64) -> Option<Vec<u8>> {
        if self.is_complete(id) {
            let chunk_list = self.chunks.get(&id).unwrap();
            let mut message_data = Vec::new();
            for chunk in chunk_list {
                if let Some(data) = chunk {
                    message_data.extend_from_slice(&data);
                }
            }
            return Some(message_data);
        }
        None
    }
}

#[derive(Serialize, Deserialize)]
pub enum MessageType {
    NewImage { bytes: Vec<u8> },
    CanvasState { state: SyncableState },
}

#[derive(Serialize, Deserialize)]
pub struct SyncableState {
    // TSTransfor does not derive Ser/Deser.
    // transform : Option<TSTransform>,
    // images: Vec<CanvasImageData>,
    pub dropped_bytes: Vec<Vec<u8>>,
}

impl From<&App> for SyncableState {
    fn from(value: &App) -> Self {
        Self {
            dropped_bytes: value.dropped_bytes.clone(),
        }
    }
}
