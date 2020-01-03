// Copyright 2019 Liebi Technologies.
// This file is part of Bifrost.

// Bifrost is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Bifrost is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Bifrost.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
#[macro_use] extern crate log;

#[cfg(feature = "std")]
use std::sync::mpsc::channel;
#[cfg(feature = "std")]
use std::sync::mpsc::Sender as ThreadOut;

pub use codec::{Decode, Encode};
use metadata::RuntimeMetadataPrefixed;
#[cfg(feature = "std")]
use node_metadata::NodeMetadata;
use sp_core::{crypto::Pair, H256 as Hash};
use primitive_types::U256;
use sp_std::prelude::*;
use sp_version::RuntimeVersion;
#[cfg(feature = "std")]
use websocket::ClientBuilder;
#[cfg(feature = "std")]
use ws::Result as WsResult;
#[cfg(feature = "std")]
use rpc::json_req;

#[cfg(feature = "std")]
use utils::*;

#[macro_use]
pub mod extrinsic;
#[cfg(feature = "std")]
pub mod node_metadata;
#[cfg(feature = "std")]
pub mod rpc;
#[cfg(feature = "std")]
pub mod utils;

pub use sp_core;
pub use keyring;
use sp_runtime::{AccountId32, MultiSignature};

#[cfg(feature = "std")]
#[derive(Clone)]
pub struct Api<P>
    where
        P: Pair,
        MultiSignature: From<P::Signature>,
{
    url: String,
    pub signer: Option<P>,
    pub genesis_hash: Hash,
    pub metadata: NodeMetadata,
    pub sp_version: RuntimeVersion,
}

#[cfg(feature = "std")]
impl<P> Api<P>
    where
        P: Pair,
        MultiSignature: From<P::Signature>,
{
    pub fn new(url: String) -> Self {
        let genesis_hash = Self::_get_genesis_hash(url.clone());
        info!("Got genesis hash: {:?}", genesis_hash);

        let meta = Self::_get_metadata(url.clone());
        let metadata = node_metadata::parse_metadata(&meta);
        info!("Metadata: {:?}", metadata);

        let sp_version = Self::_get_runtime_version(url.clone());
	    info!("Runtime Version: {:?}", sp_version);

        Self {
            url,
            signer: None,
            genesis_hash,
            metadata,
            sp_version,
        }
    }

    pub fn set_signer(mut self, signer: P) -> Self {
        self.signer = Some(signer);
        self
    }

    fn _get_genesis_hash(url: String) -> Hash {
        let jsonreq = json_req::chain_get_block_hash();
        let genesis_hash_str = Self::_get_request(url, jsonreq.to_string())
            .expect("Fetching genesis hash from node failed");
        hexstr_to_hash(genesis_hash_str).unwrap()
    }

    fn _get_runtime_version(url: String) -> RuntimeVersion {
        let jsonreq = json_req::state_get_runtime_version();
        let version_str = Self::_get_request(url, jsonreq.to_string()).unwrap(); //expect("Fetching runtime version from node failed");
        debug!("got the following runtime version (raw): {}", version_str);
        serde_json::from_str(&version_str).unwrap()
    }

    fn _get_metadata(url: String) -> RuntimeMetadataPrefixed {
        let jsonreq = json_req::state_get_metadata();
        let metadata_str = Self::_get_request(url, jsonreq.to_string()).unwrap();

        let _unhex = hexstr_to_vec(metadata_str).unwrap();
        let mut _om = _unhex.as_slice();
        RuntimeMetadataPrefixed::decode(&mut _om).unwrap()
    }

    fn _get_nonce(url: String, signer: [u8; 32]) -> u32 {
        let result_str = Self::_get_storage(
            url,
            "System",
            "AccountNonce",
            Some(signer.encode()),
        ).unwrap();
        let nonce = hexstr_to_u256(result_str).unwrap_or(U256::from_little_endian(&[0, 0, 0, 0]));
        nonce.low_u32()
    }

    fn _get_storage(
        url: String,
        module: &str,
        storage_key_name: &str,
        param: Option<Vec<u8>>,
    ) -> WsResult<String> {
        let keyhash = storage_key_hash(module, storage_key_name, param);
        debug!("with storage key: {}", keyhash);
        let jsonreq = json_req::state_get_storage(&keyhash);
        Self::_get_request(url, jsonreq.to_string())
    }

    fn _get_storage_double_map(
        url: String,
        module: &str,
        storage_key_name: &str,
        first: Vec<u8>,
        second: Vec<u8>
    ) -> WsResult<String> {
        let keyhash = storage_key_hash_double_map(module, storage_key_name, first, second);
        debug!("with storage key: {}", keyhash);
        let jsonreq = json_req::state_get_storage(&keyhash);
        Self::_get_request(url, jsonreq.to_string())
    }

    // low level access
    fn _get_request(url: String, jsonreq: String) -> WsResult<String> {
        let (result_in, result_out) = channel();
        rpc::get(url, jsonreq.clone(), result_in.clone());

        Ok(result_out.recv().unwrap())
    }

    pub fn get_metadata(&self) -> RuntimeMetadataPrefixed {
        Self::_get_metadata(self.url.clone())
    }

    pub fn get_spec_version(&self) -> u32 {
        Self::_get_runtime_version(self.url.clone()).spec_version
    }

    pub fn get_genesis_hash(&self) -> Hash {
        Self::_get_genesis_hash(self.url.clone())
    }

    pub fn get_nonce(&self) -> Result<u32, &str> {
        match &self.signer {
            Some(key) => {
                let mut arr: [u8; 32] = Default::default();
                arr.clone_from_slice(key.to_owned().public().as_ref());
                Ok(Self::_get_nonce(self.url.clone(), arr))
            },
            None => Err("Can't get nonce when no signer is set"),
        }
    }

    pub fn get_free_balance(&self, address: &AccountId32) -> U256 {
        let id: &[u8; 32] = address.as_ref();
        let result_str = self
            .get_storage("Balances", "FreeBalance", Some(id.to_owned().encode()))
            .unwrap();
        hexstr_to_u256(result_str).unwrap()
    }

    pub fn get_request(&self, jsonreq: String) -> WsResult<String> {
        Self::_get_request(self.url.clone(), jsonreq)
    }

    pub fn get_storage(
        &self,
        storage_prefix: &str,
        storage_key_name: &str,
        param: Option<Vec<u8>>,
    ) -> WsResult<String> {
        Self::_get_storage(self.url.clone(), storage_prefix, storage_key_name, param)
    }

    pub fn get_storage_double_map(
        &self,
        storage_prefix: &str,
        storage_key_name: &str,
        first: Vec<u8>,
        second: Vec<u8>,
    ) -> WsResult<String> {
        Self::_get_storage_double_map(self.url.clone(), storage_prefix, storage_key_name,
                                      first, second)
    }

    pub fn send_extrinsic(&self, xthex_prefixed: String) -> WsResult<Hash> {
        debug!("sending extrinsic: {:?}", xthex_prefixed);

        let jsonreq = json_req::author_submit_and_watch_extrinsic(&xthex_prefixed).to_string();

        let (result_in, result_out) = channel();
        rpc::send_extrinsic_and_wait_until_finalized(
            self.url.clone(),
            jsonreq.clone(),
            result_in.clone(),
        );

        Ok(hexstr_to_hash(result_out.recv().unwrap()).unwrap())
    }

    pub fn subscribe_events(&self, sender: ThreadOut<String>) {
        debug!("subscribing to events");
        let key = storage_key_hash("System", "Events", None);
        let jsonreq = json_req::state_subscribe_storage(&key).to_string();

        rpc::start_event_subscriber(self.url.clone(), jsonreq.clone(), sender.clone());
    }
}


#[cfg(feature = "std")]
pub fn is_online(ws_addr: &str) -> websocket::WebSocketResult<bool> {
    let mut client = ClientBuilder::new(&ws_addr).unwrap();
    let client = client.connect(None)?;
    let _ = client.shutdown()?;
    Ok(true)
}
