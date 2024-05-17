// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::HashValue;
use crate::U64;
use aptos_types::chain_id::ChainId;
use poem_openapi::{Enum, Object as PoemObject};
use serde::{Deserialize, Serialize};

/// The Ledger information representing the current state of the chain
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct LedgerInfo {
    /// Chain ID of the current chain
    pub chain_id: u8,
    pub epoch: U64,
    pub ledger_version: U64,
    pub oldest_ledger_version: U64,
    pub block_height: U64,
    pub oldest_block_height: U64,
    pub ledger_timestamp: U64,
}

impl LedgerInfo {
    pub fn new(
        chain_id: &ChainId,
        info: &aptos_types::ledger_info::LedgerInfoWithSignatures,
        oldest_ledger_version: u64,
        oldest_block_height: u64,
        block_height: u64,
    ) -> Self {
        let ledger_info = info.ledger_info();
        Self {
            chain_id: chain_id.id(),
            epoch: U64::from(ledger_info.epoch()),
            ledger_version: ledger_info.version().into(),
            oldest_ledger_version: oldest_ledger_version.into(),
            block_height: block_height.into(),
            oldest_block_height: oldest_block_height.into(),
            ledger_timestamp: ledger_info.timestamp_usecs().into(),
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn version(&self) -> u64 {
        self.ledger_version.into()
    }

    pub fn oldest_version(&self) -> u64 {
        self.oldest_ledger_version.into()
    }

    pub fn timestamp(&self) -> u64 {
        self.ledger_timestamp.into()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PoemObject)]
pub struct LedgerInfoWithSignatures {
    pub variant: LedgerInfoVariant,
    pub data: LedgerInfoWithV0,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Enum)]
pub enum LedgerInfoVariant {
    V0,
}

impl From<aptos_types::ledger_info::LedgerInfoWithSignatures> for LedgerInfoWithSignatures {
    fn from(value: aptos_types::ledger_info::LedgerInfoWithSignatures) -> Self {
        match value {
            aptos_types::ledger_info::LedgerInfoWithSignatures::V0(v0) => {
                LedgerInfoWithSignatures {
                    variant: LedgerInfoVariant::V0,
                    data: v0.into(),
                }
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct LedgerInfoWithV0 {
    ledger_info: CompleteLedgerInfo,
    /// Aggregated BLS signature of all the validators that signed the message. The bitmask in the
    /// aggregated signature can be used to find out the individual validators signing the message
    signatures: AggregateSignature,
}

impl From<aptos_types::ledger_info::LedgerInfoWithV0> for LedgerInfoWithV0 {
    fn from(value: aptos_types::ledger_info::LedgerInfoWithV0) -> Self {
        Self {
            ledger_info: value.ledger_info().clone().into(),
            signatures: value.signatures().clone().into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct CompleteLedgerInfo {
    commit_info: BlockInfo,

    /// Hash of consensus specific data that is opaque to all parts of the system other than
    /// consensus.
    consensus_data_hash: HashValue,
}

impl From<aptos_types::ledger_info::LedgerInfo> for CompleteLedgerInfo {
    fn from(value: aptos_types::ledger_info::LedgerInfo) -> Self {
        Self {
            commit_info: value.commit_info().clone().into(),
            consensus_data_hash: value.consensus_data_hash().into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct BlockInfo {
    /// The epoch to which the block belongs.
    epoch: U64,
    /// The consensus protocol is executed in rounds, which monotonically increase per epoch.
    round: U64,
    /// The identifier (hash) of the block.
    id: HashValue,
    /// The accumulator root hash after executing this block.
    executed_state_id: HashValue,
    /// The version of the latest transaction after executing this block.
    version: U64,
    /// The timestamp this block was proposed by a proposer.
    timestamp_usecs: U64,
    /// An optional field containing the next epoch info
    next_epoch_state: Option<EpochState>,
}

impl From<aptos_types::block_info::BlockInfo> for BlockInfo {
    fn from(value: aptos_types::block_info::BlockInfo) -> Self {
        Self {
            epoch: value.epoch().into(),
            round: value.round().into(),
            id: value.id().into(),
            executed_state_id: value.executed_state_id().into(),
            version: value.version().into(),
            timestamp_usecs: value.timestamp_usecs().into(),
            next_epoch_state: value.next_epoch_state().map(|state| state.clone().into()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct EpochState {
    pub epoch: U64,
    pub verifier: ValidatorVerifier,
}

impl EpochState {
    pub fn new(epoch: U64, verifier: ValidatorVerifier) -> Self {
        Self { epoch, verifier }
    }
}

impl From<aptos_types::epoch_state::EpochState> for EpochState {
    fn from(value: aptos_types::epoch_state::EpochState) -> Self {
        Self {
            epoch: value.epoch.into(),
            verifier: value.verifier.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct ValidatorVerifier {
    /// A vector of each validator's on-chain account address to its pubkeys and voting power.
    validator_infos: Vec<ValidatorConsensusInfo>,
}

impl From<aptos_types::validator_verifier::ValidatorVerifier> for ValidatorVerifier {
    fn from(value: aptos_types::validator_verifier::ValidatorVerifier) -> Self {
        Self {
            validator_infos: value
                .validator_infos()
                .iter()
                .map(|info| info.clone().into())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct ValidatorConsensusInfo {
    pub address: Vec<u8>,
    pub public_key: Vec<u8>,
    pub voting_power: U64,
}

impl From<aptos_types::validator_verifier::ValidatorConsensusInfo> for ValidatorConsensusInfo {
    fn from(value: aptos_types::validator_verifier::ValidatorConsensusInfo) -> Self {
        Self {
            address: value.address.to_vec(),
            public_key: value.public_key().to_bytes().to_vec(),
            voting_power: value.voting_power.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PoemObject)]
pub struct AggregateSignature {
    validator_bitmask: Vec<u8>,
    sig: Option<Vec<u8>>,
}

impl From<aptos_types::aggregate_signature::AggregateSignature> for AggregateSignature {
    fn from(sig: aptos_types::aggregate_signature::AggregateSignature) -> Self {
        dbg!(&sig);
        Self {
            validator_bitmask: sig.get_signers_bitvec().clone().into(),
            sig: sig.sig().clone().map(|sig| sig.to_bytes().to_vec()),
        }
    }
}
