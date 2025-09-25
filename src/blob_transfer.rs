use serde::{Deserialize, Serialize};

// blob transfer subprotocol

// <request>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Request {
    // param: batch_id
    GetInfo(u128),

    // params: batch_id, chunk index
    GetChunk(u128, usize)
}

// <response>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Response {
    Info(BlobInfo),

    Chunk(BlobChunk)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobInfo {
    // entire blob's hash
    pub hash: u128,
    
    pub num_chunks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobChunk {
    pub blob_hash: u128,

    pub index: usize,

    pub data: Vec<u8>,

    pub chunk_hash: u128,
}