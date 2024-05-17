// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{HashValue};
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};
use aptos_types::proof::{SparseMerkleLeafNode as InternLeafNode, SparseMerkleProof as InternProof};

/// A SparseMerkleProof
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct SparseMerkleProof {
    pub leaf: Option<SparseMerkleLeafNode>,
    pub siblings: Vec<HashValue>,
}

impl From<InternProof> for SparseMerkleProof {
    fn from(proof: InternProof) -> Self {
        Self {
            leaf: proof.leaf().map(|leaf| leaf.into()),
            siblings: proof.siblings().iter().map(|sibling| HashValue::from(*sibling)).collect::<Vec<HashValue>>(),
        }
    }
}

/// A SparseMerkleLeafNode
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct SparseMerkleLeafNode {
    pub key: HashValue,
    pub value: HashValue,
}

impl From<InternLeafNode> for  SparseMerkleLeafNode {
    fn from(value: InternLeafNode) -> Self {
        Self {
            key: value.key().into(),
            value: value.value_hash().into(),
        }
    }
}