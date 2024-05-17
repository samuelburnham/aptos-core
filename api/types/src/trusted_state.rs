use crate::ledger_info::EpochState;
use crate::waypoint::Waypoint;
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PoemObject)]
pub struct TrustedState {
    pub variant: u8,
    pub data: TrustedStateData,
}

impl TrustedState {
    pub fn new_epoch_state(waypoint: Waypoint, epoch_state: EpochState) -> Self {
        Self {
            variant: 1,
            data: TrustedStateData {
                waypoint,
                epoch_state,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PoemObject)]
pub struct TrustedStateData {
    waypoint: Waypoint,
    epoch_state: EpochState,
}

impl From<aptos_types::trusted_state::TrustedState> for TrustedState {
    fn from(value: aptos_types::trusted_state::TrustedState) -> Self {
        match value {
            aptos_types::trusted_state::TrustedState::EpochWaypoint(_) => {
                unimplemented!("Cannot handle TrustedState::EpochWaypoint")
            },
            aptos_types::trusted_state::TrustedState::EpochState {
                epoch_state,
                waypoint,
            } => TrustedState {
                variant: 1,
                data: TrustedStateData {
                    epoch_state: epoch_state.into(),
                    waypoint: waypoint.into(),
                },
            },
        }
    }
}
