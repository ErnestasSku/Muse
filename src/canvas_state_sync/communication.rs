// use eframe::emath::TSTransform;
use serde::{Deserialize, Serialize};

use crate::canvas_app::App;

// use crate::canvas_image::CanvasImageData;


#[derive(Serialize, Deserialize)]
pub enum MessageType {
    NewImage { bytes: Vec<u8> },
    CanvasState { state: SyncableState },
    // 
}

#[derive(Serialize, Deserialize)]
pub struct SyncableState {
    // transform : Option<TSTransform>,
    // images: Vec<CanvasImageData>,
    pub dropped_bytes: Vec<Vec<u8>>,
    
}

impl From<&App> for SyncableState {
    fn from(value: &App) -> Self {
        Self { dropped_bytes: value.dropped_bytes.clone() }
    }
}