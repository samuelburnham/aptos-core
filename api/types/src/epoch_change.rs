use crate::ledger_info::LedgerInfoWithSignatures;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PoemObject)]
pub struct EpochChangeProof {
    pub ledger_info_with_sigs: Vec<LedgerInfoWithSignatures>,
    pub more: bool,
}

impl From<aptos_types::epoch_change::EpochChangeProof> for EpochChangeProof {
    fn from(epoch_change_proof: aptos_types::epoch_change::EpochChangeProof) -> Self {
        Self {
            ledger_info_with_sigs: epoch_change_proof
                .ledger_info_with_sigs
                .into_iter()
                .map(LedgerInfoWithSignatures::from)
                .collect(),
            more: epoch_change_proof.more,
        }
    }
}
