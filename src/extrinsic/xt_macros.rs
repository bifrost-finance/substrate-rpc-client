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
            ),
        );

        let signature = raw_payload.using_encoded(|payload| $signer.sign(payload));

        let mut arr: [u8; 32] = Default::default();
        arr.clone_from_slice($signer.public().as_ref());
        UncheckedExtrinsicV3 {
            signature: Some((GenericAddress::from(arr), signature, extra)),
            function: $call,
        }
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
macro_rules! compose_extrinsic {
	($api: expr,
	$module: expr,
	$call: expr
	$(, $args: expr) *) => {
		{
            use $crate::extrinsic::codec::Compact;
            use $crate::extrinsic::xt_primitives::*;

            let call = compose_call!($api.metadata.clone(), $module, $call $(, ($args)) *);

            if let Some(signer) = $api.signer.clone() {
                compose_extrinsic_offline!(
                    signer,
                    call.clone(),
                    $api.get_nonce().unwrap(),
                    $api.genesis_hash,
                    $api.sp_version.spec_version
                )
            } else {
                UncheckedExtrinsicV3 {
                    signature: None,
                    function: call.clone(),
                }
            }
		}
    };
}

#[cfg(test)]
mod tests {
    use codec::{Compact, Encode};
    use keyring::AccountKeyring;
    use sp_core::{sr25519, crypto::Pair};
    use crate::extrinsic::xt_primitives::*;
    use crate::utils::*;
    use crate::Api;

    #[test]
    fn test_balances_transfer() {
        env_logger::init();
        let url = "127.0.0.1:9944";
        let from = AccountKeyring::Alice.pair();
        let api = Api::new(format!("ws://{}", url)).set_signer(from.clone());

        let to = AccountId::from(AccountKeyring::Bob);
        let result = api.get_free_balance(&to);
        info!("[+] Bob's Free Balance is is {}\n", result);

        let acc_id = GenericAddress::from(to.0.clone());
        // generate extrinsic
        let xt: UncheckedExtrinsicV3<_, sr25519::Pair> = compose_extrinsic!(
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
        let tx_hash = api.send_extrinsic(xt.hex_encode()).unwrap();
        println!("[+] Transaction got finalized. Hash: {:?}\n", tx_hash);

        // verify that Bob's free Balance increased
        let result = api.get_free_balance(&to);
        println!("[+] Bob's Free Balance is now {}\n", result);
    }
}
