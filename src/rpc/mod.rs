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

use client::*;
use std::sync::mpsc::Sender as ThreadOut;
use std::thread;
use ws::connect;

mod client;
pub mod json_req;

pub fn get(url: String, json_req: String, result_in: ThreadOut<String>) {
    start_rpc_client_thread(url, json_req, result_in, on_get_request_msg)
}

pub fn send_extrinsic_and_wait_until_finalized(
    url: String,
    json_req: String,
    result_in: ThreadOut<String>,
) {
    start_rpc_client_thread(url, json_req, result_in, on_extrinsic_msg)
}

pub fn start_event_subscriber(url: String, json_req: String, result_in: ThreadOut<String>) {
    start_rpc_client_thread(url, json_req, result_in, on_subscription_msg)
}

fn start_rpc_client_thread(
    url: String,
    jsonreq: String,
    result_in: ThreadOut<String>,
    on_message_fn: OnMessageFn,
) {
    let _client = thread::Builder::new()
        .name("client".to_owned())
        .spawn(move || {
            connect(url, |out| RpcClient {
                out,
                request: jsonreq.clone(),
                result: result_in.clone(),
                on_message_fn,
            })
            .unwrap()
        })
        .unwrap();
}
