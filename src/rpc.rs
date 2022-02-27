//! This library crate defines the remote counting service.
//!
//! The client and server depend on it.

use bluer::Uuid;
use remoc::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fmt};

/// Generic RPC error.
#[derive(Clone, Serialize, Deserialize)]
pub struct GenericRpcError {
    msg: String,
    debug_msg: String,
}

impl fmt::Debug for GenericRpcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", &self.debug_msg)
    }
}

impl fmt::Display for GenericRpcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.debug_msg)
    }
}

impl From<anyhow::Error> for GenericRpcError {
    fn from(err: anyhow::Error) -> Self {
        Self {
            msg: format!("{}", &err),
            debug_msg: format!("{:?}", &err),
        }
    }
}

impl From<remoc::rtc::CallError> for GenericRpcError {
    fn from(err: remoc::rtc::CallError) -> Self {
        Self {
            msg: format!("{}", &err),
            debug_msg: format!("{:?}", &err),
        }
    }
}

impl std::error::Error for GenericRpcError {}

/// Generic RPC result.
pub type GenericRpcResult<T> = Result<T, GenericRpcError>;

/// BlueR remote testing service.
#[rtc::remote]
pub trait BlueRTest {
    /// Get the Bluetooth address
    async fn get_server_address(&self) -> GenericRpcResult<[u8; 6]>;
    async fn get_client_address(&self) -> GenericRpcResult<[u8; 6]>;
    async fn get_client_name(&self) -> GenericRpcResult<String>;

    /// Send Bluetooth LE advertisements.
    ///
    /// The sending is stopped when the returned oneshot channel sender is dropped.
    async fn advertise(
        &self,
        local_name: Option<String>,
        service_uuids: BTreeSet<Uuid>,
    ) -> GenericRpcResult<rch::oneshot::Sender<()>>;
}
