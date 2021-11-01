//! Functionality for chunking file data and calculating and verifying root Merkle tree root ids.

use crate::{
    crypto::{Methods, Provider},
    error::ArweaveError,
};
use borsh::BorshDeserialize;
type Error = ArweaveError;

/// Single struct used for original data chunks (Leaves) and branch nodes (hashes of pairs of child nodes).
#[derive(Debug, PartialEq, Clone)]
pub struct Node {
    pub id: [u8; HASH_SIZE],
    pub data_hash: Option<[u8; HASH_SIZE]>,
    pub min_byte_range: usize,
    pub max_byte_range: usize,
    pub left_child: Option<Box<Node>>,
    pub right_child: Option<Box<Node>>,
}

/// Concatenated ids and offsets for full set of nodes for an original data chunk, starting with the root.
#[derive(Debug, PartialEq, Clone)]
pub struct Proof {
    pub offset: usize,
    pub proof: Vec<u8>,
}

/// Populated with data from deserialized [`Proof`] for original data chunk (Leaf [`Node`]).
#[repr(C)]
#[derive(BorshDeserialize, Debug, PartialEq, Clone)]
pub struct LeafProof {
    data_hash: [u8; HASH_SIZE],
    notepad: [u8; NOTE_SIZE - 8],
    offset: [u8; 8],
}

/// Populated with data from deserialized [`Proof`] for branch [`Node`] (hash of pair of child nodes).
#[derive(BorshDeserialize, Debug, PartialEq, Clone)]
pub struct BranchProof {
    left_id: [u8; HASH_SIZE],
    right_id: [u8; HASH_SIZE],
    notepad: [u8; NOTE_SIZE - 8],
    offset: [u8; 8],
}

/// Includes methods to deserialize [`Proof`]s.
pub trait ProofDeserialize<T> {
    fn try_from_proof_slice(slice: &[u8]) -> Result<T, Error>;
    fn offset(&self) -> usize;
}

impl ProofDeserialize<LeafProof> for LeafProof {
    fn try_from_proof_slice(slice: &[u8]) -> Result<Self, Error> {
        let proof = LeafProof::try_from_slice(slice)?;
        Ok(proof)
    }
    fn offset(&self) -> usize {
        usize::from_be_bytes(self.offset)
    }
}

impl ProofDeserialize<BranchProof> for BranchProof {
    fn try_from_proof_slice(slice: &[u8]) -> Result<Self, Error> {
        let proof = BranchProof::try_from_slice(slice)?;
        Ok(proof)
    }
    fn offset(&self) -> usize {
        usize::from_be_bytes(self.offset)
    }
}

pub const MAX_CHUNK_SIZE: usize = 256 * 1024;
pub const MIN_CHUNK_SIZE: usize = 32 * 1024;
pub const HASH_SIZE: usize = 32;
const NOTE_SIZE: usize = 32;

/// Includes a function to convert a number to a Vec of 32 bytes per the Arweave spec.
pub trait Helpers<T> {
    fn to_note_vec(&self) -> Vec<u8>;
}

impl Helpers<usize> for usize {
    fn to_note_vec(&self) -> Vec<u8> {
        let mut note = vec![0; NOTE_SIZE - 8];
        note.extend((*self as u64).to_be_bytes());
        note
    }
}

/// Ensures there are always an even number of chunks.
fn get_chunk_size(data_len: usize) -> usize {
    if data_len <= MAX_CHUNK_SIZE {
        data_len
    } else {
        get_chunk_size(data_len / 2)
    }
}

/// Generates data chunks from which the calculation of root id starts.
pub fn generate_leaves(data: Vec<u8>, crypto: &Provider) -> Result<Vec<Node>, Error> {
    let chunk_size = get_chunk_size(data.len());
    let data_chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();

    let mut leaves = Vec::<Node>::new();
    let mut min_byte_range = 0;
    for chunk in data_chunks.iter() {
        let data_hash = crypto.hash_SHA256(chunk)?;
        let max_byte_range = min_byte_range + &chunk.len();
        let offset = max_byte_range.to_note_vec();
        let id = crypto.hash_all_SHA256(vec![&data_hash, &offset])?;

        leaves.push(Node {
            id,
            data_hash: Some(data_hash),
            min_byte_range,
            max_byte_range,
            left_child: None,
            right_child: None,
        });
        min_byte_range = min_byte_range + &chunk.len();
    }
    Ok(leaves)
}

/// Hashes together a single branch node from a pair of child nodes.
pub fn hash_branch(left: Node, right: Node, crypto: &Provider) -> Result<Node, Error> {
    let max_byte_range = left.max_byte_range.to_note_vec();
    let id = crypto.hash_all_SHA256(vec![&left.id, &right.id, &max_byte_range])?;
    Ok(Node {
        id,
        data_hash: None,
        min_byte_range: left.max_byte_range,
        max_byte_range: right.max_byte_range,
        left_child: Some(Box::new(left)),
        right_child: Some(Box::new(right)),
    })
}

/// Builds one layer of branch nodes from a layer of child nodes.
pub fn build_layer<'a>(nodes: Vec<Node>, crypto: &Provider) -> Result<Vec<Node>, Error> {
    let mut layer = Vec::<Node>::with_capacity(&nodes.len() / 2);
    let mut nodes_iter = nodes.into_iter();
    while let (Some(left), Some(right)) = (nodes_iter.next(), nodes_iter.next()) {
        layer.push(hash_branch(left, right, &crypto).unwrap());
    }
    Ok(layer)
}

/// Builds all layers from leaves up to single root node.
pub fn generate_data_root(mut nodes: Vec<Node>, crypto: &Provider) -> Result<Node, Error> {
    while nodes.len() > 1 {
        nodes = build_layer(nodes, &crypto)?;
    }
    let root = nodes.pop().unwrap();
    Ok(root)
}

/// Calculates [`Proof`] for each data chunk contained in root [`Node`].
pub fn resolve_proofs(node: Node, proof: Option<Proof>) -> Result<Vec<Proof>, Error> {
    let mut proof = if let Some(proof) = proof {
        proof
    } else {
        Proof {
            offset: 0,
            proof: Vec::new(),
        }
    };
    match node {
        // Leaf
        Node {
            data_hash: Some(data_hash),
            max_byte_range,
            left_child: None,
            right_child: None,
            ..
        } => {
            proof.offset = max_byte_range - 1;
            proof.proof.extend(data_hash);
            proof.proof.extend(max_byte_range.to_note_vec());
            return Ok(vec![proof]);
        }
        // Branch
        Node {
            data_hash: None,
            min_byte_range,
            left_child: Some(left_child),
            right_child: Some(right_child),
            ..
        } => {
            proof.proof.extend(left_child.id.clone());
            proof.proof.extend(right_child.id.clone());
            proof.proof.extend(min_byte_range.to_note_vec());

            let mut left_proof = resolve_proofs(*left_child, Some(proof.clone()))?;
            let right_proof = resolve_proofs(*right_child, Some(proof))?;
            left_proof.extend(right_proof);
            return Ok(left_proof);
        }
        _ => unreachable!(),
    }
}

/// Validates chunk of data against provided [`Proof`].
pub fn validate_chunk(
    mut root_id: [u8; HASH_SIZE],
    chunk: Node,
    proof: Proof,
    crypto: &Provider,
) -> Result<(), Error> {
    match chunk {
        Node {
            data_hash: Some(data_hash),
            max_byte_range,
            ..
        } => {
            // Split proof into branches and leaf. Leaf is at the end and branches are ordered
            // from root to leaf.
            let (branches, leaf) = proof
                .proof
                .split_at(proof.proof.len() - HASH_SIZE - NOTE_SIZE);

            // Deserailze proof.
            let branch_proofs: Vec<BranchProof> = branches
                .chunks(HASH_SIZE * 2 + NOTE_SIZE)
                .map(|b| BranchProof::try_from_proof_slice(b).unwrap())
                .collect();
            let leaf_proof = LeafProof::try_from_proof_slice(leaf)?;

            // Validate branches.
            for branch_proof in branch_proofs.iter() {
                // Calculate the id from the proof.
                let id = crypto.hash_all_SHA256(vec![
                    &branch_proof.left_id,
                    &branch_proof.right_id,
                    &branch_proof.offset().to_note_vec(),
                ])?;

                // Ensure calculated id correct.
                if !(id == root_id) {
                    return Err(ArweaveError::InvalidProof.into());
                }

                // If the offset from the proof is greater than the offset in the data chunk,
                // then the next id to validate against is from the left.
                root_id = match max_byte_range > branch_proof.offset() {
                    true => branch_proof.right_id,
                    false => branch_proof.left_id,
                }
            }

            // Validate leaf: both id and data_hash are correct.
            let id = crypto.hash_all_SHA256(vec![&data_hash, &max_byte_range.to_note_vec()])?;
            if !(id == root_id) & !(data_hash == leaf_proof.data_hash) {
                return Err(ArweaveError::InvalidProof.into());
            }
        }
        _ => {
            unreachable!()
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio::fs::File;
    use tokio::io::AsyncReadExt;

    #[test]
    fn test_get_chunk_size() -> () {
        assert_eq!(get_chunk_size(MIN_CHUNK_SIZE), MIN_CHUNK_SIZE);
        assert_eq!(get_chunk_size(MAX_CHUNK_SIZE), MAX_CHUNK_SIZE);
        assert_eq!(get_chunk_size(MAX_CHUNK_SIZE * 4), MAX_CHUNK_SIZE);
        assert_eq!(get_chunk_size(MAX_CHUNK_SIZE + 1), (MAX_CHUNK_SIZE + 1) / 2);
    }

    #[tokio::test]
    async fn test_generate_leaves() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ))
        .await?;
        let mut file = File::open("tests/fixtures/1mb.bin").await?;

        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let leaves: Vec<Node> = generate_leaves(contents, &crypto)?;
        assert_eq!(
            leaves[1],
            Node {
                id: [
                    116, 162, 15, 141, 57, 10, 17, 205, 78, 2, 213, 56, 154, 61, 223, 174, 73, 226,
                    192, 82, 70, 39, 237, 145, 89, 66, 199, 123, 31, 23, 88, 38
                ],
                data_hash: Some([
                    49, 180, 221, 222, 226, 186, 75, 140, 193, 105, 70, 238, 149, 178, 153, 32,
                    144, 208, 63, 136, 223, 103, 186, 4, 109, 24, 64, 127, 20, 38, 98, 56
                ]),
                min_byte_range: 262144,
                max_byte_range: 524288,
                left_child: None,
                right_child: None
            }
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_hash_branch() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ))
        .await?;
        let mut file = File::open("tests/fixtures/1mb.bin").await?;

        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let leaves: Vec<Node> = generate_leaves(contents, &crypto)?;
        let mut nodes_iter = leaves.into_iter();
        let left = nodes_iter.next().unwrap();
        let right = nodes_iter.next().unwrap();
        let left_clone = left.clone();
        let right_clone = right.clone();

        let branch = hash_branch(left, right, &crypto)?;
        assert_eq!(
            branch,
            Node {
                id: [
                    50, 116, 51, 211, 72, 86, 49, 84, 45, 220, 75, 153, 44, 133, 213, 88, 58, 246,
                    8, 202, 100, 249, 227, 0, 10, 177, 116, 187, 113, 95, 41, 10,
                ],
                data_hash: None,
                min_byte_range: 262144,
                max_byte_range: 524288,
                left_child: Some(Box::new(left_clone)),
                right_child: Some(Box::new(right_clone))
            }
        );
        Ok(())
    }
    #[tokio::test]
    async fn test_build_layer() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ))
        .await?;
        let mut file = File::open("tests/fixtures/1mb.bin").await?;

        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let leaves: Vec<Node> = generate_leaves(contents, &crypto)?;
        let layer = build_layer(leaves, &crypto)?;
        assert_eq!(
            layer[0].id,
            [
                50, 116, 51, 211, 72, 86, 49, 84, 45, 220, 75, 153, 44, 133, 213, 88, 58, 246, 8,
                202, 100, 249, 227, 0, 10, 177, 116, 187, 113, 95, 41, 10,
            ]
        );
        assert_eq!(layer[0].min_byte_range, 262144);
        assert_eq!(layer[0].max_byte_range, 524288);
        Ok(())
    }

    #[tokio::test]
    async fn test_generate_data_root() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ))
        .await?;
        let mut file = File::open("tests/fixtures/1mb.bin").await?;

        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let leaves: Vec<Node> = generate_leaves(contents, &crypto)?;
        let root = generate_data_root(leaves, &crypto)?;
        assert_eq!(
            root.id,
            [
                80, 80, 69, 118, 15, 123, 97, 51, 235, 63, 130, 216, 210, 76, 201, 220, 236, 7, 49,
                169, 83, 37, 80, 107, 186, 166, 114, 203, 209, 56, 127, 13,
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_generate_data_root_one_chunk() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ))
        .await?;
        let mut file = File::open("tests/fixtures/0.png").await?;

        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let leaves: Vec<Node> = generate_leaves(contents, &crypto)?;
        let root = generate_data_root(leaves, &crypto)?;
        assert_eq!(
            root.id,
            [
                216, 248, 50, 181, 40, 249, 132, 35, 200, 106, 229, 132, 126, 90, 154, 0, 40, 87,
                155, 74, 122, 236, 32, 170, 181, 74, 80, 15, 126, 87, 83, 253,
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_generate_proofs() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ))
        .await?;
        let mut file = File::open("tests/fixtures/1mb.bin").await?;

        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let leaves: Vec<Node> = generate_leaves(contents, &crypto)?;
        let root = generate_data_root(leaves, &crypto)?;

        let proofs = resolve_proofs(root, None)?;
        assert_eq!(
            proofs,
            vec![
                Proof {
                    offset: 262143,
                    proof: vec![
                        50, 116, 51, 211, 72, 86, 49, 84, 45, 220, 75, 153, 44, 133, 213, 88, 58,
                        246, 8, 202, 100, 249, 227, 0, 10, 177, 116, 187, 113, 95, 41, 10, 119, 13,
                        144, 53, 9, 45, 125, 241, 131, 57, 212, 158, 95, 206, 105, 115, 140, 212,
                        17, 23, 115, 169, 161, 158, 153, 149, 218, 162, 104, 142, 196, 29, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 8, 0, 0, 141, 126, 47, 91, 10, 25, 183, 143, 217, 39, 240, 188, 236, 97,
                        89, 124, 249, 124, 168, 186, 135, 247, 16, 248, 154, 100, 81, 108, 28, 156,
                        152, 156, 116, 162, 15, 141, 57, 10, 17, 205, 78, 2, 213, 56, 154, 61, 223,
                        174, 73, 226, 192, 82, 70, 39, 237, 145, 89, 66, 199, 123, 31, 23, 88, 38,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 4, 0, 0, 230, 109, 212, 174, 212, 28, 88, 210, 62, 6, 202, 239,
                        18, 88, 65, 18, 49, 13, 5, 41, 138, 129, 207, 143, 110, 56, 113, 12, 150,
                        223, 137, 174, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0,
                    ],
                },
                Proof {
                    offset: 524287,
                    proof: vec![
                        50, 116, 51, 211, 72, 86, 49, 84, 45, 220, 75, 153, 44, 133, 213, 88, 58,
                        246, 8, 202, 100, 249, 227, 0, 10, 177, 116, 187, 113, 95, 41, 10, 119, 13,
                        144, 53, 9, 45, 125, 241, 131, 57, 212, 158, 95, 206, 105, 115, 140, 212,
                        17, 23, 115, 169, 161, 158, 153, 149, 218, 162, 104, 142, 196, 29, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 8, 0, 0, 141, 126, 47, 91, 10, 25, 183, 143, 217, 39, 240, 188, 236, 97,
                        89, 124, 249, 124, 168, 186, 135, 247, 16, 248, 154, 100, 81, 108, 28, 156,
                        152, 156, 116, 162, 15, 141, 57, 10, 17, 205, 78, 2, 213, 56, 154, 61, 223,
                        174, 73, 226, 192, 82, 70, 39, 237, 145, 89, 66, 199, 123, 31, 23, 88, 38,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 4, 0, 0, 49, 180, 221, 222, 226, 186, 75, 140, 193, 105, 70,
                        238, 149, 178, 153, 32, 144, 208, 63, 136, 223, 103, 186, 4, 109, 24, 64,
                        127, 20, 38, 98, 56, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0,
                    ],
                },
                Proof {
                    offset: 786431,
                    proof: vec![
                        50, 116, 51, 211, 72, 86, 49, 84, 45, 220, 75, 153, 44, 133, 213, 88, 58,
                        246, 8, 202, 100, 249, 227, 0, 10, 177, 116, 187, 113, 95, 41, 10, 119, 13,
                        144, 53, 9, 45, 125, 241, 131, 57, 212, 158, 95, 206, 105, 115, 140, 212,
                        17, 23, 115, 169, 161, 158, 153, 149, 218, 162, 104, 142, 196, 29, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 8, 0, 0, 184, 200, 34, 207, 230, 129, 160, 91, 89, 41, 88, 49, 206, 83,
                        214, 37, 73, 186, 9, 26, 183, 92, 145, 187, 130, 156, 175, 142, 173, 186,
                        68, 121, 134, 230, 103, 186, 33, 99, 44, 211, 13, 30, 4, 8, 118, 27, 175,
                        31, 253, 126, 114, 237, 95, 159, 103, 107, 39, 34, 85, 210, 115, 233, 28,
                        12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 12, 0, 0, 185, 69, 99, 13, 191, 69, 39, 118, 239, 167, 31,
                        90, 244, 65, 38, 2, 232, 200, 221, 155, 198, 115, 245, 16, 124, 111, 87,
                        90, 140, 66, 53, 152, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0,
                    ],
                },
                Proof {
                    offset: 1048575,
                    proof: vec![
                        50, 116, 51, 211, 72, 86, 49, 84, 45, 220, 75, 153, 44, 133, 213, 88, 58,
                        246, 8, 202, 100, 249, 227, 0, 10, 177, 116, 187, 113, 95, 41, 10, 119, 13,
                        144, 53, 9, 45, 125, 241, 131, 57, 212, 158, 95, 206, 105, 115, 140, 212,
                        17, 23, 115, 169, 161, 158, 153, 149, 218, 162, 104, 142, 196, 29, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 8, 0, 0, 184, 200, 34, 207, 230, 129, 160, 91, 89, 41, 88, 49, 206, 83,
                        214, 37, 73, 186, 9, 26, 183, 92, 145, 187, 130, 156, 175, 142, 173, 186,
                        68, 121, 134, 230, 103, 186, 33, 99, 44, 211, 13, 30, 4, 8, 118, 27, 175,
                        31, 253, 126, 114, 237, 95, 159, 103, 107, 39, 34, 85, 210, 115, 233, 28,
                        12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 12, 0, 0, 50, 32, 92, 251, 142, 102, 91, 172, 106, 78, 179,
                        155, 43, 216, 139, 97, 1, 94, 246, 106, 215, 17, 178, 4, 158, 174, 150,
                        127, 37, 219, 39, 191, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0,
                    ],
                },
            ]
        );
        Ok(())
    }
    #[tokio::test]
    async fn test_validate_chunks() -> Result<(), Error> {
        let crypto = Provider::from_keypair_path(PathBuf::from(
            "tests/fixtures/arweave-key-7eV1qae4qVNqsNChg3Scdi-DpOLJPCogct4ixoq1WNg.json",
        ))
        .await?;
        let mut file = File::open("tests/fixtures/1mb.bin").await?;

        let mut contents = vec![];
        file.read_to_end(&mut contents).await?;
        let leaves: Vec<Node> = generate_leaves(contents, &crypto)?;
        let root = generate_data_root(leaves.clone(), &crypto)?;
        let root_id = root.id.clone();
        let proofs = resolve_proofs(root, None)?;

        for (chunk, proof) in leaves.into_iter().zip(proofs.into_iter()) {
            assert_eq!((), validate_chunk(root_id.clone(), chunk, proof, &crypto)?);
        }

        Ok(())
    }
}
