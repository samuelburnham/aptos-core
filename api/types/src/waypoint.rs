use crate::{HashValue, U64};
use poem_openapi::Object as PoemObject;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, PoemObject)]
pub struct Waypoint {
    version: U64,
    value: HashValue,
}

impl From<aptos_types::waypoint::Waypoint> for Waypoint {
    fn from(value: aptos_types::waypoint::Waypoint) -> Self {
        Self {
            version: value.version().into(),
            value: value.value().into(),
        }
    }
}
