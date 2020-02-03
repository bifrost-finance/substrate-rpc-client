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
//! Offers macros that build extrinsics for custom runtime modules based on the metadata.
//! Additionally, some predefined extrinsics for common runtime modules are implemented.

/// Generates the extrinsic's call field for a given module and call passed as &str
/// # Arguments
///
/// * 'node_metadata' - This crate's parsed node metadata as field of the API.
/// * 'module' - Module name as &str for which the call is composed.
/// * 'call' - Call name as &str
/// * 'args' - Optional sequence of arguments of the call. They are not checked against the metadata.
/// As of now the user needs to check himself that the correct arguments are supplied.
#[macro_export]
macro_rules! compose_call {
($node_metadata: expr, $module: expr, $call_name: expr $(, $args: expr) *) => {
        {
            let mut meta = $node_metadata;
            meta.retain(|m| !m.calls.is_empty());

            let module_index = meta
            .iter().position(|m| m.name == $module).expect("Module not found in Metadata");

            let call_index = meta[module_index].calls
            .iter().position(|c| c.name == $call_name).expect("Call not found in Module");

            ([module_index as u8, call_index as u8] $(, ($args)) *)
        }
    };
}

/// Generates an Unchecked extrinsic for a given call
/// # Arguments
///
/// * 'signer' - AccountKey that is used to sign the extrinsic.
/// * 'call' - call as returned by the compose_call! macro or via substrate's call enums.
/// * 'nonce' - signer's account nonce: u32
/// * 'genesis_hash' - sp_core::Hash256/[u8; 32].
/// * 'runtime_spec_version' - RuntimeVersion.spec_version/u32
#[macro_export]
macro_rules! compose_extrinsic_offline {
    ($signer: expr,
    $call: expr,
    $nonce: expr,
    $genesis_hash: expr,
    $runtime_spec_version: expr) => {{
        use $crate::extrinsic::xt_primitives::*;
        use $crate::sp_core::crypto::Pair;;
        use $crate::extrinsic::node_primitives::AccountId;

        let extra = GenericExtra::new($nonce);
        let raw_payload = SignedPayload::from_raw(
            $call.clone(),
            extra.clone(),
            (
                $runtime_spec_version,
                $genesis_hash,
                $genesis_hash,
                (),
                (),
                (),
                (),
            ),
        );

        let signature = raw_payload.using_encoded(|payload| $signer.sign(payload));

        let mut arr: [u8; 32] = Default::default();
        arr.clone_from_slice($signer.public().as_ref());

        UncheckedExtrinsicV4::new_signed(
            $call,
            GenericAddress::from(AccountId::from(arr)),
            signature.into(),
            extra
        )
    }};
}

/// Generates an Unchecked extrinsic for a given module and call passed as a &str.
/// # Arguments
///
/// * 'api' - This instance of API. If the *signer* field is not set, an unsigned extrinsic will be generated.
/// * 'module' - Module name as &str for which the call is composed.
/// * 'call' - Call name as &str
/// * 'args' - Optional sequence of arguments of the call. They are not checked against the metadata.
/// As of now the user needs to check himself that the correct arguments are supplied.

#[macro_export]
#[cfg(feature = "std")]
macro_rules! compose_extrinsic {
    ($api: expr,
    $module: expr,
    $call: expr
    $(, $args: expr) *) => {
        {
            use $crate::extrinsic::xt_primitives::*;

            info!("Composing generic extrinsic for module {:?} and call {:?}", $module, $call);
            let call = $crate::compose_call!($api.metadata.clone(), $module, $call $(, ($args)) *);

            if let Some(signer) = $api.signer.clone() {
                $crate::compose_extrinsic_offline!(
                    signer,
                    call.clone(),
                    $api.get_nonce().unwrap(),
                    $api.genesis_hash,
                    $api.sp_version.spec_version
                )
            } else {
                UncheckedExtrinsicV4 {
                    signature: None,
                    function: call.clone(),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use codec::Compact;
    use keyring::AccountKeyring;
    use sp_core::crypto::Pair;
    use node_primitives::AccountId;
    use crate::extrinsic::xt_primitives::*;
    use crate::Api;
    use eos_chain::{
        Action, ActionReceipt, Checksum256, get_proof,
        IncrementalMerkle, ProducerSchedule, SignedBlockHeader
    };

    use std::{
        error::Error,
        fs::File,
        io::Read,
        path::Path,
        str::FromStr,
    };
//    use substrate_primitives::crypto::UncheckedInto;

    #[test]
    fn test_init_schedule() {
        env_logger::init();
        let url = "127.0.0.1:9944";
        let from = AccountKeyring::Alice.pair();
        let api = Api::new(format!("ws://{}", url)).set_signer(from.clone());

        let schedule = r#"
            {
  "version": 1,
  "producers": [
    {
      "producer_name": "batinthedark",
      "block_signing_key": "EOS6dwoM8XGMQn49LokUcLiony7JDkbHrsFDvh5svLvPDkXtvM7oR"
    },
    {
      "producer_name": "bighornsheep",
      "block_signing_key": "EOS5xfwWr4UumKm4PqUGnyCrFWYo6j5cLioNGg5yf4GgcTp2WcYxf"
    },
    {
      "producer_name": "bigpolarbear",
      "block_signing_key": "EOS6oZi9WjXUcLionUtSiKRa4iwCW5cT6oTzoWZdENXq1p2pq53Nv"
    },
    {
      "producer_name": "clevermonkey",
      "block_signing_key": "EOS5mp5wmRyL5RH2JUeEh3eoZxkJ2ZZJ9PVd1BcLioNuq4PRCZYxQ"
    },
    {
      "producer_name": "funnyhamster",
      "block_signing_key": "EOS7A9BoRetjpKtE3sqA6HRykRJ955MjQ5XdRmCLionVte2uERL8h"
    },
    {
      "producer_name": "gorillapower",
      "block_signing_key": "EOS8X5NCx1Xqa1xgQgBa9s6EK7M1SjGaDreAcLion4kDVLsjhQr9n"
    },
    {
      "producer_name": "hippopotamus",
      "block_signing_key": "EOS7qDcxm8YtAZUA3t9kxNGuzpCLioNnzpTRigi5Dwsfnszckobwc"
    },
    {
      "producer_name": "hungryolddog",
      "block_signing_key": "EOS6tw3AqqVUsCbchYRmxkPLqGct3vC63cEzKgVzLFcLionoY8YLQ"
    },
    {
      "producer_name": "iliketurtles",
      "block_signing_key": "EOS6itYvNZwhqS7cLion3xp3rLJNJAvKKegxeS7guvbBxG1XX5uwz"
    },
    {
      "producer_name": "jumpingfrogs",
      "block_signing_key": "EOS7oVWG413cLioNG7RU5Kv7NrPZovAdRSP6GZEG4LFUDWkgwNXHW"
    },
    {
      "producer_name": "lioninjungle",
      "block_signing_key": "EOS5BcLionmbgEtcmu7qY6XKWaE1q31qCQSsd89zXij7FDXQnKjwk"
    },
    {
      "producer_name": "littlerabbit",
      "block_signing_key": "EOS65orCLioNFkVT5uDF7J63bNUk97oF8T83iWfuvbSKWYUUq9EWd"
    },
    {
      "producer_name": "proudrooster",
      "block_signing_key": "EOS5qBd3T6nmLRsuACLion346Ue8UkCwvsoS5f3EDC1jwbrEiBDMX"
    },
    {
      "producer_name": "pythoncolors",
      "block_signing_key": "EOS8R7GB5CLionUEy8FgGksGAGtc2cbcQWgty3MTAgzJvGTmtqPLz"
    },
    {
      "producer_name": "soaringeagle",
      "block_signing_key": "EOS6iuBqJKqSK82QYCGuM96gduQpQG8xJsPDU1CLionPMGn2bT4Yn"
    },
    {
      "producer_name": "spideronaweb",
      "block_signing_key": "EOS6M4CYEDt3JDKS6nsxMnUcdCLioNcbyEzeAwZsQmDcoJCgaNHT8"
    },
    {
      "producer_name": "ssssssssnake",
      "block_signing_key": "EOS8SDhZ5CLioNLie9mb7kDu1gHfDXLwTvYBSxR1ccYSJERvutLqG"
    },
    {
      "producer_name": "thebluewhale",
      "block_signing_key": "EOS6Wfo1wwTPzzBVT8fe3jpz8vxCnf77YscLionBnw39iGzFWokZm"
    },
    {
      "producer_name": "thesilentowl",
      "block_signing_key": "EOS7y4hU89NJ658H1KmAdZ6A585bEVmSV8xBGJ3SbQM4Pt3pcLion"
    },
    {
      "producer_name": "wealthyhorse",
      "block_signing_key": "EOS5i1HrfxfHLRJqbExgRodhrZwp4dcLioNn4xZWCyhoBK6DNZgZt"
    }
  ]
}
        "#;

        let v2_producers: Result<eos_chain::ProducerSchedule, _> = serde_json::from_str(schedule);
        assert!(v2_producers.is_ok());
        let v2_producers = v2_producers.unwrap();
//        let s = hex::encode(schedule).into_bytes();

        let json = "change_schedule_9313.json";
        let signed_blocks_str = read_json_from_file(json);
        let signed_blocks_headers: Vec<SignedBlockHeader> = serde_json::from_str(&signed_blocks_str.unwrap()).unwrap();

        let ids_json = "block_ids_list.json";
        let ids_str = read_json_from_file(ids_json).unwrap();
        let block_ids_list: Vec<Vec<String>> = serde_json::from_str(&ids_str).unwrap();

        let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.iter().map(|ids| {
            ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
        }).collect::<Vec<_>>();

        let proposal = compose_call!(
            api.metadata.clone(),
            "BridgeEos",
            "init_schedule",
            v2_producers,
            signed_blocks_headers,
            block_ids_list
        );

        let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
            api.clone(),
            "Sudo",
            "sudo",
            proposal
        );

        println!("[+] Composed extrinsic: {:?}\n", xt);
        // send and watch extrinsic until finalized
        let tx_hash = api.send_extrinsic(xt.hex_encode()).unwrap();
        println!("[+] Transaction got finalized. Hash: {:?}\n", tx_hash);
    }

    #[test]
    fn prove_action_should_be_ok() {
        let shedule_json = "schedule_v2.json";
        let v2_producers_str = read_json_from_file(shedule_json);
        assert!(v2_producers_str.is_ok());
        let v2_producers: Result<ProducerSchedule, _> = serde_json::from_str(&v2_producers_str.unwrap());
        assert!(v2_producers.is_ok());
        let v2_producers = v2_producers.unwrap();

        let v2_schedule_hash = v2_producers.schedule_hash();
        assert!(v2_schedule_hash.is_ok());

        // get block headers
        let block_headers_json = "actions_verification_10776.json";
        let signed_blocks_str = read_json_from_file(block_headers_json);
        let signed_blocks: Result<Vec<SignedBlockHeader>, _> = serde_json::from_str(&signed_blocks_str.unwrap());
        assert!(signed_blocks.is_ok());
        let signed_blocks_headers = signed_blocks.unwrap();

        let node_count = 10774;
        let active_nodes: Vec<Checksum256> = vec![
            "45c2c1cbc4b049d72a627124b05f5c476ae1cc87955fbea70bc8dbe549cf395a".into(),
            "d96747605aaed959630b23a28e0004f42a87eae93f51d5fe241735644a0c3921".into(),
            "937a489eea576d74a3d091cc4dcf1cb867f01e314ac7f1334f6cec00dfcee476".into(),
            "36cbf5d9c35b2538181bf7f8af4ee57c55c17e516eedd992a73bace9ca14a5c3".into(),
            "40e8bb864481e7bb01674ec3517c84e557869fea8160c4b2762d3e83d71d6034".into(),
            "afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
            "f1329d3ee84040279460cbc87b6769b7363e477a832f73d639e0692a4042f093".into()
        ];
        let merkle = IncrementalMerkle::new(node_count, active_nodes);

        // block ids list
        let ids_json = "block_ids_list_10776.json";
        let ids_str = read_json_from_file(ids_json).unwrap();
        let block_ids_list: Result<Vec<Vec<String>>, _> = serde_json::from_str(&ids_str);
        assert!(block_ids_list.is_ok());

        let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.as_ref().unwrap().iter().map(|ids| {
            ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
        }).collect::<Vec<_>>();

        // read action merkle paths
        let action_merkle_paths_json = "action_merkle_paths.json";
        let action_merkle_paths_str = read_json_from_file(action_merkle_paths_json);
        assert!(action_merkle_paths_str.is_ok());
        let action_merkle_paths: Result<Vec<String>, _> = serde_json::from_str(&action_merkle_paths_str.unwrap());
        assert!(action_merkle_paths.is_ok());
        let action_merkle_paths = action_merkle_paths.unwrap();
        let action_merkle_paths = {
            let mut path: Vec<Checksum256> = Vec::with_capacity(action_merkle_paths.len());
            for path_str in action_merkle_paths {
                path.push(Checksum256::from_str(&path_str).unwrap());
            }
            path
        };

        let proof = get_proof(15, action_merkle_paths);
        assert!(proof.is_ok());
        let actual_merkle_paths = proof.unwrap();

        // get action
        let actions_json = "actions_from_10776.json";
        let actions_str = read_json_from_file(actions_json);
        assert!(actions_str.is_ok());
        let actions: Result<Vec<Action>, _> = serde_json::from_str(actions_str.as_ref().unwrap());
        assert!(actions.is_ok());
        let actions = actions.unwrap();

        let action = actions[3].clone();

        let action_receipt = r#"{
			"receiver": "megasuper333",
			"act_digest": "eaa3b4bf845a1b41668ab7ca49fb5644fc91a6c0156dfd33911b4ec69d2e41d6",
			"global_sequence": 3040972,
			"recv_sequence": 1,
			"auth_sequence": [
			  [
				"junglefaucet",
				21
			  ]
			],
			"code_sequence": 2,
			"abi_sequence": 2
		}"#;
        let action_receipt: Result<ActionReceipt, _> = serde_json::from_str(action_receipt);
        assert!(action_receipt.is_ok());
        let action_receipt = action_receipt.unwrap();

        env_logger::init();
        let url = "127.0.0.1:9944";
        let from = AccountKeyring::Alice.pair();
        let api = Api::new(format!("ws://{}", url)).set_signer(from.clone());

        let proposal = compose_call!(
            api.metadata.clone(),
            "BridgeEos",
            "prove_action",
            action,
            action_receipt,
            actual_merkle_paths,
            merkle,
            signed_blocks_headers,
            block_ids_list
        );

        let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
            api.clone(),
            "Sudo",
            "sudo",
            proposal
        );

        println!("[+] Composed extrinsic: {:?}\n", xt);
        // send and watch extrinsic until finalized
        let tx_hash = api.send_extrinsic(xt.hex_encode()).unwrap();
        println!("[+] Transaction got finalized. Hash: {:?}\n", tx_hash);
    }

    #[test]
    fn change_schedule_should_be_ok() {
        let json = "change_schedule_9313.json";
        let signed_blocks_str = read_json_from_file(json);
        let signed_blocks_headers: Vec<SignedBlockHeader> = serde_json::from_str(&signed_blocks_str.unwrap()).unwrap();

        let ids_json = "block_ids_list.json";
        let ids_str = read_json_from_file(ids_json).unwrap();
        let block_ids_list: Vec<Vec<String>> = serde_json::from_str(&ids_str).unwrap();

        let block_ids_list: Vec<Vec<Checksum256>> = block_ids_list.iter().map(|ids| {
            ids.iter().map(|id| Checksum256::from_str(id).unwrap()).collect::<Vec<_>>()
        }).collect::<Vec<_>>();

        let node_count = 9311;
        let active_nodes: Vec<Checksum256> = vec![
            "0000245f60aa338bd246cb7598a14796ee0210f669f9c9b37f6ddad0b5765649".into(),
            "9d41d4581cab233fe68c4510cacd05d0cc979c53ae317ce9364040578037de6a".into(),
            "a397d1a6dc90389dc592ea144b1801c4b323c12b0b2f066aa55faa5892803317".into(),
            "0cf502411e185ea7e3cc790e0b757807987e767a81c463c3e4ee5970b7fd1c67".into(),
            "9f774a35e86ddb2d293da1bfe2e25b7b447fd3d9372ee580fce230a87fefa586".into(),
            "4d018eda9a22334ac0492489fdf79118d696eea52af3871a7e4bf0e2d5ab5945".into(),
            "acba7c7ee5c1d8ba97ea1a841707fbb2147e883b56544ba821814aebe086383e".into(),
            "afa502d408f5bdf1660fa9fe3a1fcb432462467e7eb403a8499392ee5297d8d1".into(),
            "4d723385cad26cf80c2db366f9666a3ef77679c098e07d1af48d523b64b1d460".into()
        ];
        let merkle = IncrementalMerkle::new(node_count, active_nodes);

        env_logger::init();
        let url = "127.0.0.1:9944";
        let from = AccountKeyring::Alice.pair();
        let api = Api::new(format!("ws://{}", url)).set_signer(from.clone());

        let proposal = compose_call!(
            api.metadata.clone(),
            "BridgeEos",
            "change_schedule",
            merkle,
            signed_blocks_headers,
            block_ids_list
        );

        let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
            api.clone(),
            "Sudo",
            "sudo",
            proposal
        );

        // Unable to decode Vec on index 2 createType(ExtrinsicV4):: Source is too large
        println!("[+] Composed extrinsic: {:?}\n", xt);
        // send and watch extrinsic until finalized
        let tx_hash = api.send_extrinsic(xt.hex_encode()).unwrap();
        println!("[+] Transaction got finalized. Hash: {:?}\n", tx_hash);
    }

    fn read_json_from_file(json_name: impl AsRef<str>) -> Result<String, Box<dyn Error>> {
        let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/test_data/")).join(json_name.as_ref());
        let mut file = File::open(path)?;
        let mut json_str = String::new();
        file.read_to_string(&mut json_str)?;
        Ok(json_str)
    }

    #[test]
    fn test_balances_transfer() {
        env_logger::init();
        let url = "127.0.0.1:9944";
        let from = AccountKeyring::Alice.pair();
        let api = Api::new(format!("ws://{}", url)).set_signer(from.clone());

        let to = AccountId::from(AccountKeyring::Bob);
        let result = api.get_free_balance(&to);
        println!("[+] Bob's Free Balance is is {}\n", result);

        let acc_id = GenericAddress::from(to.clone());
        // generate extrinsic
        let xt: UncheckedExtrinsicV4<_> = compose_extrinsic!(
            api.clone(),
            "Balances",
            "transfer",
            acc_id,
            Compact(1230u128)
        );

        println!(
            "Sending an extrinsic from Alice (Key = {:?}),\n\nto Bob (Key = {:?})\n",
            from.public(),
            to
        );

        println!("[+] Composed extrinsic: {:?}\n", xt);
        // send and watch extrinsic until finalized
        let tx_hash = api.send_extrinsic(xt.hex_encode());
        assert!(tx_hash.is_ok());
        println!("[+] Transaction got finalized. Hash: {:?}\n", tx_hash.unwrap());

        // verify that Bob's free Balance increased
        let result = api.get_free_balance(&to);
        println!("[+] Bob's Free Balance is now {}\n", result);
    }
}
