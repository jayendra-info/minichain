//! Merkle tree implementation for transaction roots.

use crate::hash::{hash_concat, Hash};

/// Compute the merkle root of a list of hashes.
///
/// Returns the zero hash if the list is empty.
/// Uses a binary merkle tree with pair-wise hashing.
pub fn merkle_root(hashes: &[Hash]) -> Hash {
    if hashes.is_empty() {
        return Hash::ZERO;
    }

    if hashes.len() == 1 {
        return hashes[0];
    }

    // Build the tree bottom-up
    let mut current_level: Vec<Hash> = hashes.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));

        for chunk in current_level.chunks(2) {
            let combined = if chunk.len() == 2 {
                hash_concat(&[chunk[0].as_ref(), chunk[1].as_ref()])
            } else {
                // Odd number of elements: hash the last one with itself
                hash_concat(&[chunk[0].as_ref(), chunk[0].as_ref()])
            };
            next_level.push(combined);
        }

        current_level = next_level;
    }

    current_level[0]
}

/// A merkle tree for efficient proofs.
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// All nodes in the tree, level by level (leaves first).
    levels: Vec<Vec<Hash>>,
}

/// A merkle proof for a single leaf.
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// The leaf being proven.
    pub leaf: Hash,
    /// Sibling hashes from leaf to root.
    pub siblings: Vec<Hash>,
    /// Direction for each sibling (true = right, false = left).
    pub directions: Vec<bool>,
}

impl MerkleTree {
    /// Build a merkle tree from a list of leaf hashes.
    pub fn new(leaves: &[Hash]) -> Self {
        if leaves.is_empty() {
            return Self {
                levels: vec![vec![Hash::ZERO]],
            };
        }

        let mut levels = vec![leaves.to_vec()];

        while levels.last().unwrap().len() > 1 {
            let current = levels.last().unwrap();
            let mut next = Vec::with_capacity(current.len().div_ceil(2));

            for chunk in current.chunks(2) {
                let combined = if chunk.len() == 2 {
                    hash_concat(&[chunk[0].as_ref(), chunk[1].as_ref()])
                } else {
                    hash_concat(&[chunk[0].as_ref(), chunk[0].as_ref()])
                };
                next.push(combined);
            }

            levels.push(next);
        }

        Self { levels }
    }

    /// Get the root of the merkle tree.
    pub fn root(&self) -> Hash {
        *self.levels.last().unwrap().first().unwrap()
    }

    /// Get the number of leaves in the tree.
    pub fn leaf_count(&self) -> usize {
        self.levels.first().map(|l| l.len()).unwrap_or(0)
    }

    /// Generate a proof for the leaf at the given index.
    pub fn proof(&self, index: usize) -> Option<MerkleProof> {
        if index >= self.leaf_count() {
            return None;
        }

        let leaf = self.levels[0][index];
        let mut siblings = Vec::new();
        let mut directions = Vec::new();
        let mut idx = index;

        for level in &self.levels[..self.levels.len() - 1] {
            let sibling_idx = if idx.is_multiple_of(2) {
                idx + 1
            } else {
                idx - 1
            };
            let is_right = idx.is_multiple_of(2);

            let sibling = if sibling_idx < level.len() {
                level[sibling_idx]
            } else {
                level[idx] // Odd leaf hashes with itself
            };

            siblings.push(sibling);
            directions.push(is_right);
            idx /= 2;
        }

        Some(MerkleProof {
            leaf,
            siblings,
            directions,
        })
    }

    /// Verify a merkle proof against this tree's root.
    pub fn verify_proof(&self, proof: &MerkleProof) -> bool {
        verify_proof(&self.root(), proof)
    }
}

/// Verify a merkle proof against a given root.
pub fn verify_proof(root: &Hash, proof: &MerkleProof) -> bool {
    let mut current = proof.leaf;

    for (sibling, is_right) in proof.siblings.iter().zip(proof.directions.iter()) {
        current = if *is_right {
            hash_concat(&[current.as_ref(), sibling.as_ref()])
        } else {
            hash_concat(&[sibling.as_ref(), current.as_ref()])
        };
    }

    current == *root
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::hash;

    fn make_hashes(n: usize) -> Vec<Hash> {
        (0..n).map(|i| hash(&[i as u8])).collect()
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = merkle_root(&[]);
        assert_eq!(root, Hash::ZERO);
    }

    #[test]
    fn test_merkle_root_single() {
        let hashes = make_hashes(1);
        let root = merkle_root(&hashes);
        assert_eq!(root, hashes[0]);
    }

    #[test]
    fn test_merkle_root_two() {
        let hashes = make_hashes(2);
        let root = merkle_root(&hashes);
        let expected = hash_concat(&[hashes[0].as_ref(), hashes[1].as_ref()]);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let hashes = make_hashes(10);
        let r1 = merkle_root(&hashes);
        let r2 = merkle_root(&hashes);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_merkle_root_order_matters() {
        let hashes = make_hashes(4);
        let mut reversed = hashes.clone();
        reversed.reverse();

        let r1 = merkle_root(&hashes);
        let r2 = merkle_root(&reversed);
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_merkle_tree_root_matches() {
        let hashes = make_hashes(8);
        let tree = MerkleTree::new(&hashes);
        assert_eq!(tree.root(), merkle_root(&hashes));
    }

    #[test]
    fn test_merkle_tree_odd_leaves() {
        let hashes = make_hashes(7);
        let tree = MerkleTree::new(&hashes);
        assert_eq!(tree.root(), merkle_root(&hashes));
    }

    #[test]
    fn test_merkle_proof_valid() {
        let hashes = make_hashes(8);
        let tree = MerkleTree::new(&hashes);

        for i in 0..hashes.len() {
            let proof = tree.proof(i).unwrap();
            assert!(tree.verify_proof(&proof));
            assert!(verify_proof(&tree.root(), &proof));
        }
    }

    #[test]
    fn test_merkle_proof_odd_leaves() {
        let hashes = make_hashes(5);
        let tree = MerkleTree::new(&hashes);

        for i in 0..hashes.len() {
            let proof = tree.proof(i).unwrap();
            assert!(tree.verify_proof(&proof));
        }
    }

    #[test]
    fn test_merkle_proof_invalid_index() {
        let hashes = make_hashes(4);
        let tree = MerkleTree::new(&hashes);
        assert!(tree.proof(10).is_none());
    }

    #[test]
    fn test_merkle_proof_wrong_root() {
        let hashes = make_hashes(4);
        let tree = MerkleTree::new(&hashes);
        let proof = tree.proof(0).unwrap();

        let wrong_root = hash(b"wrong");
        assert!(!verify_proof(&wrong_root, &proof));
    }
}
