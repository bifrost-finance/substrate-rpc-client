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
use std::sync::mpsc::Sender as ThreadOut;
use ws::{CloseCode, Handler, Handshake, Message, Result, Sender};

pub type OnMessageFn = fn(msg: Message, out: Sender, result: ThreadOut<String>) -> Result<()>;

pub struct RpcClient {
    pub out: Sender,
    pub request: String,
    pub result: ThreadOut<String>,
    pub on_message_fn: OnMessageFn,
}

impl Handler for RpcClient {
    fn on_open(&mut self, _: Handshake) -> Result<()> {
        self.out.send(self.request.clone()).unwrap();
        Ok(())
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        (self.on_message_fn)(msg, self.out.clone(), self.result.clone())
    }
}

pub fn on_get_request_msg(msg: Message, out: Sender, result: ThreadOut<String>) -> Result<()> {
    let retstr = msg.as_text().unwrap();
    let value: serde_json::Value = serde_json::from_str(retstr).unwrap();

    result.send(value["result"].to_string()).unwrap();
    out.close(CloseCode::Normal).unwrap();
    Ok(())
}

pub fn on_subscription_msg(msg: Message, _out: Sender, result: ThreadOut<String>) -> Result<()> {
    let retstr = msg.as_text().unwrap();
    let value: serde_json::Value = serde_json::from_str(retstr).unwrap();
    match value["id"].as_str() {
        Some(_idstr) => {}
        _ => {
            // subscriptions
            debug!("no id field found in response. must be subscription");
            debug!("method: {:?}", value["method"].as_str());
            match value["method"].as_str() {
                Some("state_storage") => {
                    let _changes = &value["params"]["result"]["changes"];
                    let _res_str = _changes[0][1].as_str().unwrap().to_string();
                    result.send(_res_str).unwrap();
                }
                _ => error!("unsupported method"),
            }
        }
    };
    Ok(())
}

pub fn on_extrinsic_msg(msg: Message, out: Sender, result: ThreadOut<String>) -> Result<()> {
    let retstr = msg.as_text().unwrap();
    let value: serde_json::Value = serde_json::from_str(retstr).unwrap();
    match value["id"].as_str() {
        Some(idstr) => match idstr.parse::<u32>() {
            Ok(req_id) => match req_id {
                REQUEST_TRANSFER => match value.get("error") {
                    Some(err) => error!("ERROR: {:?}", err),
                    _ => debug!("no error"),
                },
                _ => debug!("Unknown request id"),
            },
            Err(_) => error!("error assigning request id"),
        },
        _ => {
            // subscriptions
            debug!("no id field found in response. must be subscription");
            debug!("method: {:?}", value["method"].as_str());
            match value["method"].as_str() {
                Some("author_extrinsicUpdate") => {
                    match value["params"]["result"].as_str() {
                        Some(res) => debug!("author_extrinsicUpdate: {}", res),
                        _ => {
                            debug!(
                                "author_extrinsicUpdate: finalized: {}",
                                value["params"]["result"]["finalized"].as_str().unwrap()
                            );
                            // return result to calling thread
                            result
                                .send(
                                    value["params"]["result"]["finalized"]
                                        .as_str()
                                        .unwrap()
                                        .to_string(),
                                )
                                .unwrap();
                            // we've reached the end of the flow. return
                            out.close(CloseCode::Normal).unwrap();
                        }
                    }
                }
                _ => error!("unsupported method"),
            }
        }
    };
    Ok(())
}