// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    accept_type::AcceptType,
    context::api_spawn_blocking,
    failpoint::fail_point_poem,
    response::{
        api_forbidden, build_not_found, module_not_found, resource_not_found, table_item_not_found,
        BadRequestError, BasicErrorWith404, BasicResponse, BasicResponseStatus, BasicResultWith404,
        InternalError,
    },
    ApiTags, Context,
};
use anyhow::Context as AnyhowContext;
use aptos_api_types::{
    verify_module_identifier, Address, AptosErrorCode, AsConverter, IdentifierWrapper, LedgerInfo,
    MoveModuleBytecode, MoveResource, MoveStructTag, MoveValue, RawStateValueRequest,
    RawTableItemRequest, TableItemRequest, VerifyInput, VerifyInputWithRecursion, U64,
};
use aptos_crypto::hash::CryptoHash;
use aptos_crypto::HashValue;
use aptos_storage_interface::DbReader;
use aptos_types::account_config::AccountResource;
use aptos_types::epoch_change::EpochChangeProof;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use aptos_types::proof::{SparseMerkleProof, TransactionAccumulatorProof};
use aptos_types::state_store::{state_key::StateKey, table::TableHandle, TStateView};
use aptos_types::transaction::TransactionInfo;
use aptos_types::trusted_state::TrustedState;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_types::waypoint::Waypoint;
use aptos_vm::data_cache::AsMoveResolver;
use move_core_types::move_resource::MoveStructType;
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use poem_openapi::{
    param::{Path, Query},
    payload::Json,
    OpenApi,
};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, sync::Arc};

/// API for retrieving individual state
#[derive(Clone)]
pub struct StateApi {
    pub context: Arc<Context>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AccountProofPayload {
    /// Proof for the account inclusion
    state_proof: SparseMerkleProof,
    /// Account leaf key
    element_key: HashValue,
    /// Account state value
    element_hash: HashValue,
    /// Proof for the transaction inclusion
    transaction_proof: TransactionAccumulatorProof,
    /// Hashed representation of the transaction
    transaction: TransactionInfo,
    /// Transaction version.
    transaction_index: u64,
    /// Signed Ledger info with the transaction
    ledger_info_v0: LedgerInfoWithSignatures,
    /// ValidatorVerifier valid for the proof
    validator_verifier: ValidatorVerifier,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct EpochChangeProofPayload {
    epoch_change_proof: EpochChangeProof,
    trusted_state: TrustedState,
}

#[OpenApi]
impl StateApi {
    /// Get account resource
    ///
    /// Retrieves an individual resource from a given account and at a specific ledger version. If the
    /// ledger version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/accounts/:address/resource/:resource_type",
        method = "get",
        operation_id = "get_account_resource",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_resource(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Name of struct to retrieve e.g. `0x1::account::Account`
        resource_type: Path<MoveStructTag>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveResource> {
        resource_type
            .0
            .verify(0)
            .context("'resource_type' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_get_account_resource")?;
        self.context
            .check_api_output_enabled("Get account resource", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || {
            api.resource(
                &accept_type,
                address.0,
                resource_type.0,
                ledger_version.0.map(|inner| inner.0),
            )
        })
        .await
    }

    #[oai(
        path = "/accounts/:address/proof",
        method = "get",
        operation_id = "get_account_proof",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_proof(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Block height to get state of account
        ///
        /// If not provided, it will be the latest block
        block_height: Query<Option<U64>>,
    ) -> BasicResultWith404<Vec<u8>> {
        fail_point_poem("endpoint_get_account_proof")?;
        self.context
            .check_api_output_enabled("Get account proof", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || {
            api.proof(&accept_type, address.0, block_height.0.map(|inner| inner.0))
        })
        .await
    }

    #[oai(
        path = "/epoch/proof",
        method = "get",
        operation_id = "get_epoch_change_proof",
        tag = "ApiTags::General"
    )]
    async fn get_epoch_change_proof(
        &self,
        accept_type: AcceptType,
        /// Epoch number for the change proof
        ///
        /// If not provided, it will be the latest epoch change
        epoch_number: Query<Option<U64>>,
    ) -> BasicResultWith404<Vec<u8>> {
        self.context
            .check_api_output_enabled("Get account resource", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || {
            api.epoch_change_proof(&accept_type, epoch_number.0.map(|inner| inner.0))
        })
        .await
    }

    /// Get account module
    ///
    /// Retrieves an individual module from a given account and at a specific ledger version. If the
    /// ledger version is not specified in the request, the latest ledger version is used.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/accounts/:address/module/:module_name",
        method = "get",
        operation_id = "get_account_module",
        tag = "ApiTags::Accounts"
    )]
    async fn get_account_module(
        &self,
        accept_type: AcceptType,
        /// Address of account with or without a `0x` prefix
        address: Path<Address>,
        /// Name of module to retrieve e.g. `coin`
        module_name: Path<IdentifierWrapper>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveModuleBytecode> {
        verify_module_identifier(module_name.0.as_str())
            .context("'module_name' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_get_account_module")?;
        self.context
            .check_api_output_enabled("Get account module", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || {
            api.module(&accept_type, address.0, module_name.0, ledger_version.0)
        })
        .await
    }

    /// Get table item
    ///
    /// Get a table item at a specific ledger version from the table identified by {table_handle}
    /// in the path and the "key" (TableItemRequest) provided in the request body.
    ///
    /// This is a POST endpoint because the "key" for requesting a specific
    /// table item (TableItemRequest) could be quite complex, as each of its
    /// fields could themselves be composed of other structs. This makes it
    /// impractical to express using query params, meaning GET isn't an option.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/tables/:table_handle/item",
        method = "post",
        operation_id = "get_table_item",
        tag = "ApiTags::Tables"
    )]
    async fn get_table_item(
        &self,
        accept_type: AcceptType,
        /// Table handle hex encoded 32-byte string
        table_handle: Path<Address>,
        /// Table request detailing the key type, key, and value type
        table_item_request: Json<TableItemRequest>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveValue> {
        table_item_request
            .0
            .verify()
            .context("'table_item_request' invalid")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        fail_point_poem("endpoint_get_table_item")?;
        self.context
            .check_api_output_enabled("Get table item", &accept_type)?;
        let api = self.clone();
        api_spawn_blocking(move || {
            api.table_item(
                &accept_type,
                table_handle.0,
                table_item_request.0,
                ledger_version.0,
            )
        })
        .await
    }

    /// Get raw table item
    ///
    /// Get a table item at a specific ledger version from the table identified by {table_handle}
    /// in the path and the "key" (RawTableItemRequest) provided in the request body.
    ///
    /// The `get_raw_table_item` requires only a serialized key comparing to the full move type information
    /// comparing to the `get_table_item` api, and can only return the query in the bcs format.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/tables/:table_handle/raw_item",
        method = "post",
        operation_id = "get_raw_table_item",
        tag = "ApiTags::Tables"
    )]
    async fn get_raw_table_item(
        &self,
        accept_type: AcceptType,
        /// Table handle hex encoded 32-byte string
        table_handle: Path<Address>,
        /// Table request detailing the key type, key, and value type
        table_item_request: Json<RawTableItemRequest>,
        /// Ledger version to get state of account
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveValue> {
        fail_point_poem("endpoint_get_table_item")?;

        if AcceptType::Json == accept_type {
            return Err(api_forbidden(
                "Get raw table item",
                "Only BCS is supported as an AcceptType.",
            ));
        }
        self.context
            .check_api_output_enabled("Get raw table item", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || {
            api.raw_table_item(
                &accept_type,
                table_handle.0,
                table_item_request.0,
                ledger_version.0,
            )
        })
        .await
    }

    /// Get raw state value.
    ///
    /// Get a state value at a specific ledger version, identified by the key provided
    /// in the request body.
    ///
    /// The Aptos nodes prune account state history, via a configurable time window.
    /// If the requested ledger version has been pruned, the server responds with a 410.
    #[oai(
        path = "/experimental/state_values/raw",
        method = "post",
        operation_id = "get_raw_state_value",
        tag = "ApiTags::Experimental",
        hidden
    )]
    async fn get_raw_state_value(
        &self,
        accept_type: AcceptType,
        /// Request that carries the state key.
        request: Json<RawStateValueRequest>,
        /// Ledger version at which the value is got.
        ///
        /// If not provided, it will be the latest version
        ledger_version: Query<Option<U64>>,
    ) -> BasicResultWith404<MoveValue> {
        fail_point_poem("endpoint_get_raw_state_value")?;

        if AcceptType::Json == accept_type {
            return Err(api_forbidden(
                "Get raw state value",
                "Only BCS is supported as an AcceptType.",
            ));
        }
        self.context
            .check_api_output_enabled("Get raw state value", &accept_type)?;

        let api = self.clone();
        api_spawn_blocking(move || api.raw_value(&accept_type, request.0, ledger_version.0)).await
    }
}

impl StateApi {
    /// Read a resource at the ledger version
    ///
    /// JSON: Convert to MoveResource
    /// BCS: Leave it encoded as the resource
    fn resource(
        &self,
        accept_type: &AcceptType,
        address: Address,
        resource_type: MoveStructTag,
        ledger_version: Option<u64>,
    ) -> BasicResultWith404<MoveResource> {
        let tag: StructTag = resource_type
            .try_into()
            .context("Failed to parse given resource type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;

        let (ledger_info, ledger_version, state_view) = self.context.state_view(ledger_version)?;
        let bytes = state_view
            .as_converter(
                self.context.db.clone(),
                self.context.table_info_reader.clone(),
            )
            .find_resource(&state_view, address, &tag)
            .context(format!(
                "Failed to query DB to check for {} at {}",
                tag, address
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| resource_not_found(address, &tag, ledger_version, &ledger_info))?;

        match accept_type {
            AcceptType::Json => {
                let resource = state_view
                    .as_converter(
                        self.context.db.clone(),
                        self.context.table_info_reader.clone(),
                    )
                    .try_into_resource(&tag, &bytes)
                    .context("Failed to deserialize resource data retrieved from DB")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((resource, &ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    fn epoch_change_proof(
        &self,
        accept_type: &AcceptType,
        epoch_number: Option<u64>,
    ) -> BasicResultWith404<Vec<u8>> {
        let (ledger_info, _, _) = self.context.state_view(None)?;

        fn get_epoch_change_proof_payload(
            db: &Arc<dyn DbReader>,
            epoch_number: u64,
            ledger_info: &LedgerInfo,
        ) -> Result<(TrustedState, EpochChangeProof), BasicErrorWith404> {
            let mut epoch_change_proof: EpochChangeProof = db
                .get_epoch_ending_ledger_infos(epoch_number - 2, epoch_number)
                .map_err(|err| {
                    BasicErrorWith404::internal_with_code(
                        err,
                        AptosErrorCode::InternalError,
                        ledger_info,
                    )
                })?;

            assert_eq!(
                epoch_change_proof.ledger_info_with_sigs.len(),
                2,
                "Expected two LedgerInfoWithSignatures in EpochchangeProof"
            );

            let penultimate_li = epoch_change_proof.ledger_info_with_sigs.remove(0);
            let waypoint = Waypoint::new_any(penultimate_li.ledger_info());

            Ok((
                TrustedState::EpochState {
                    waypoint,
                    epoch_state: aptos_types::epoch_state::EpochState::new(
                        epoch_number - 1,
                        penultimate_li
                            .ledger_info()
                            .next_epoch_state()
                            .expect("Latest li for epoch change should contain a next EpochState")
                            .clone()
                            .verifier,
                    ),
                },
                epoch_change_proof,
            ))
        }

        let (trusted_state, epoch_change_proof): (TrustedState, EpochChangeProof) =
            match epoch_number {
                Some(epoch_number) => {
                    get_epoch_change_proof_payload(&self.context.db, epoch_number, &ledger_info)?
                },
                None => {
                    let latest_epoch_state: aptos_types::epoch_state::EpochState =
                        self.context.db.get_latest_epoch_state().map_err(|err| {
                            BasicErrorWith404::internal_with_code(
                                err,
                                AptosErrorCode::InternalError,
                                &ledger_info,
                            )
                        })?;
                    get_epoch_change_proof_payload(
                        &self.context.db,
                        latest_epoch_state.epoch,
                        &ledger_info,
                    )?
                },
            };

        let epoch_change_proof_payload = EpochChangeProofPayload {
            epoch_change_proof,
            trusted_state,
        };

        match accept_type {
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bcs::to_bytes(&epoch_change_proof_payload).unwrap(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
            _ => Err(api_forbidden(
                "Get epoch change proof",
                "Only BCS is supported as an AcceptType.",
            )),
        }
    }

    fn proof(
        &self,
        accept_type: &AcceptType,
        address: Address,
        block_height: Option<u64>,
    ) -> BasicResultWith404<Vec<u8>> {
        // Get latest ledger info
        let (ledger_info, ledger_version, state_view) = self.context.state_view(None)?;

        let tx_version = if let Some(block_height) = block_height {
            self.context
                .get_block_by_height(block_height, &ledger_info, false)?
                .last_version
        } else {
            ledger_version
        };

        let latest_li_w_sig = self
            .context
            .get_latest_ledger_info_with_signatures()
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        // Compute account key
        let account_key = StateKey::resource(address.inner(), &AccountResource::struct_tag())
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        let latest_epoch_state: aptos_types::epoch_state::EpochState =
            state_view.db.get_latest_epoch_state().map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        // Get state value and sparse merkle proof
        let (state_value, state_proof) = state_view
            .db
            .get_state_value_with_proof_by_version(&account_key, tx_version)
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        let sparse_proof: SparseMerkleProof = state_proof;
        let element_key = account_key.hash();
        let element_hash = state_value
            .ok_or_else(|| {
                BasicErrorWith404::internal_with_code(
                    "No state value from get_state_value_with_proof_by_version",
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .hash();

        let txn_w_proof = self
            .context
            .db
            .get_transaction_by_version(tx_version, latest_li_w_sig.ledger_info().version(), false)
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        let ledger_info_to_transaction_info_proof =
            txn_w_proof.proof.ledger_info_to_transaction_info_proof;

        let proof = AccountProofPayload {
            state_proof: sparse_proof,
            element_key,
            element_hash,
            transaction_proof: ledger_info_to_transaction_info_proof,
            transaction: txn_w_proof.proof.transaction_info.clone(),
            transaction_index: tx_version,
            ledger_info_v0: latest_li_w_sig,
            validator_verifier: latest_epoch_state.verifier,
        };

        match accept_type {
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bcs::to_bytes(&proof).unwrap(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
            _ => Err(api_forbidden(
                "Get account proof",
                "Only BCS is supported as an AcceptType.",
            )),
        }
    }

    /// Retrieve the module
    ///
    /// JSON: Parse ABI and bytecode
    /// BCS: Leave bytecode as is BCS encoded
    pub fn module(
        &self,
        accept_type: &AcceptType,
        address: Address,
        name: IdentifierWrapper,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveModuleBytecode> {
        let state_key = StateKey::module(address.inner(), &name);
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .context(format!("Failed to query DB to check for {:?}", state_key))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| module_not_found(address, &name, ledger_version, &ledger_info))?;

        match accept_type {
            AcceptType::Json => {
                let module = MoveModuleBytecode::new(bytes.to_vec())
                    .try_parse_abi()
                    .context("Failed to parse move module ABI from bytes retrieved from storage")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((module, &ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieve table item for a specific ledger version
    pub fn table_item(
        &self,
        accept_type: &AcceptType,
        table_handle: Address,
        table_item_request: TableItemRequest,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveValue> {
        // Parse the key and value types for the table
        let key_type = table_item_request
            .key_type
            .try_into()
            .context("Failed to parse key_type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;
        let key = table_item_request.key;
        let value_type = table_item_request
            .value_type
            .try_into()
            .context("Failed to parse value_type")
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code_no_info(err, AptosErrorCode::InvalidInput)
            })?;

        // Retrieve local state
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;

        let converter = state_view.as_converter(
            self.context.db.clone(),
            self.context.table_info_reader.clone(),
        );

        // Convert key to lookup version for DB
        let vm_key = converter
            .try_into_vm_value(&key_type, key.clone())
            .map_err(|err| {
                BasicErrorWith404::bad_request_with_code(
                    err,
                    AptosErrorCode::InvalidInput,
                    &ledger_info,
                )
            })?;
        let raw_key = vm_key.undecorate().simple_serialize().ok_or_else(|| {
            BasicErrorWith404::bad_request_with_code(
                "Failed to serialize table key",
                AptosErrorCode::InvalidInput,
                &ledger_info,
            )
        })?;

        // Retrieve value from the state key
        let state_key = StateKey::table_item(&TableHandle(table_handle.into()), &raw_key);
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .context(format!(
                "Failed when trying to retrieve table item from the DB with key: {}",
                key
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                table_item_not_found(table_handle, &key, ledger_version, &ledger_info)
            })?;

        match accept_type {
            AcceptType::Json => {
                let move_value = converter
                    .try_into_move_value(&value_type, &bytes)
                    .context("Failed to deserialize table item retrieved from DB")
                    .map_err(|err| {
                        BasicErrorWith404::internal_with_code(
                            err,
                            AptosErrorCode::InternalError,
                            &ledger_info,
                        )
                    })?;

                BasicResponse::try_from_json((move_value, &ledger_info, BasicResponseStatus::Ok))
            },
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieve table item for a specific ledger version
    pub fn raw_table_item(
        &self,
        accept_type: &AcceptType,
        table_handle: Address,
        table_item_request: RawTableItemRequest,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveValue> {
        // Retrieve local state
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;

        let state_key =
            StateKey::table_item(&TableHandle(table_handle.into()), &table_item_request.key.0);
        let bytes = state_view
            .get_state_value_bytes(&state_key)
            .context(format!(
                "Failed when trying to retrieve table item from the DB with key: {}",
                table_item_request.key,
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                build_not_found(
                    "Table Item",
                    format!(
                        "Table handle({}), Table key({}) and Ledger version({})",
                        table_handle, table_item_request.key, ledger_version
                    ),
                    AptosErrorCode::TableItemNotFound,
                    &ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => Err(api_forbidden(
                "Get raw table item",
                "Please use get table item instead.",
            )),
            AcceptType::Bcs => BasicResponse::try_from_encoded((
                bytes.to_vec(),
                &ledger_info,
                BasicResponseStatus::Ok,
            )),
        }
    }

    /// Retrieve state value for a specific ledger version
    pub fn raw_value(
        &self,
        accept_type: &AcceptType,
        request: RawStateValueRequest,
        ledger_version: Option<U64>,
    ) -> BasicResultWith404<MoveValue> {
        // Retrieve local state
        let (ledger_info, ledger_version, state_view) = self
            .context
            .state_view(ledger_version.map(|inner| inner.0))?;

        let state_key = bcs::from_bytes(&request.key.0)
            .context(format!(
                "Failed deserializing state value. key: {}",
                request.key
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;
        let state_value = state_view
            .get_state_value(&state_key)
            .context(format!("Failed fetching state value. key: {}", request.key,))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?
            .ok_or_else(|| {
                build_not_found(
                    "Raw State Value",
                    format!(
                        "StateKey({}) and Ledger version({})",
                        request.key, ledger_version
                    ),
                    AptosErrorCode::StateValueNotFound,
                    &ledger_info,
                )
            })?;
        let bytes = bcs::to_bytes(&state_value)
            .context(format!(
                "Failed serializing state value. key: {}",
                request.key
            ))
            .map_err(|err| {
                BasicErrorWith404::internal_with_code(
                    err,
                    AptosErrorCode::InternalError,
                    &ledger_info,
                )
            })?;

        match accept_type {
            AcceptType::Json => Err(api_forbidden(
                "Get raw state value",
                "This serves only bytes. Use other APIs for Json.",
            )),
            AcceptType::Bcs => {
                BasicResponse::try_from_encoded((bytes, &ledger_info, BasicResponseStatus::Ok))
            },
        }
    }
}
