use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum NeedKind {
    // need proving is an umbrella term for segment, join, keccak, and zkr proving
    Prove(u32),

    Groth16(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeccakRequestObject {
    pub claim_digest: [u8; 32],

    pub po2: usize,

    pub control_root: [u8; 32],
    
    // KeccakState in risc0
    pub input: Vec<[u64; 25]>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssumptionDetails {
    // batch id
    pub id: u128,    

    // if the batch is filled with keccak assumptions
    pub blobs_are_keccak: bool,

    pub batch: Vec<InputBlob>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputBlob {
    Blob(Vec<u8>),

    // blob is not available, params: hash, owner
    Token(u128, Vec<u8>)
}

// prove, lift, and join is repreented by this type where input
// is a list of blobs and output is a single proof
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateDetails { 
    // batch id
    pub id: u128,

    // if the batch is filled with segments
    pub blobs_are_segment: bool,

    pub batch: Vec<InputBlob>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Groth16Details {
    pub batch: Vec<(u128, InputBlob)>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobKind {
    Assumption(AssumptionDetails),

    Aggregate(AggregateDetails),

    Groth16(Groth16Details),
}

// used by clients when gossiping about compute needs
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeJob {    
    // network-wide id of the job
    pub id: u128,

    // whether it's prove, join, or groth16
    pub kind: JobKind,
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofKind {
    Assumption(u128),

    // params: batch id
    Aggregate(u128),

    // params: batch id, blob
    Groth16(u128, Vec<u8>)
}

// proofs in custody of prover
// being large in size ~200-300kb, so provers hold them until the client requests their transfer
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofToken {
    pub job_id: u128,

    pub kind: ProofKind,

    // hash of the proof blob
    pub hash: u128,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Request {
    WouldProve,

    ProofIsReady(ProofToken),

    // general blob transfer request, <hash of blob>
    TransferBlob(u128)
}

// clients respond to requests
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Response {
    Job(ComputeJob),

    BlobIsReady(Vec<u8>) 
}
