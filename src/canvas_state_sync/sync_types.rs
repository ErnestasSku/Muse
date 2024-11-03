use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::canvas_app::App;


#[derive(Serialize, Deserialize)]
pub struct ChunkedMessage {
    pub id: u64,
    pub chunk_index: u32,
    pub total_chunks: u32,
    pub data: Vec<u8>,
}

pub struct ChunkCollector {
    chunks: HashMap<u64, Vec<Option<Vec<u8>>>>,       // Mapping message ID -> list of chunks
    chunk_sizes: HashMap<u64, u32>,                   // Mapping message ID -> total number of chunks
    pub chunk_times: HashMap<u64, std::time::SystemTime>  // Mapping message ID -> TimeStamp of the message
}

impl ChunkCollector {
    pub fn new() -> Self {
        ChunkCollector {
            chunks: HashMap::new(),
            chunk_sizes: HashMap::new(),
            chunk_times: HashMap::new(),
        }
    }

    pub fn remove_chunks(&mut self, id: &u64) {
        self.chunks.remove(id);
        self.chunk_sizes.remove(id);
        self.chunk_times.remove(id);
    }

    pub fn add_chunk(&mut self, id: u64, chunk_index: u32, total_chunks: u32, data: Vec<u8>) {
        // If it's a new message, initialize storage
        self.chunk_sizes.entry(id).or_insert(total_chunks);
        self.chunk_times.entry(id).or_insert(std::time::SystemTime::now());
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

    pub fn reassemble(&mut self, id: u64) -> Option<Vec<u8>> {
        if self.is_complete(id) {
            let chunk_list = self.chunks.get(&id).unwrap();
            let mut message_data = Vec::new();
            for chunk in chunk_list {
                if let Some(data) = chunk {
                    message_data.extend_from_slice(&data);
                }
            }

            self.remove_chunks(&id);
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
