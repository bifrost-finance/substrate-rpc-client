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
use codec::{Compact, Decode, Encode};
use crate::extrinsic::address::Address;
use primitive_types::H256;
use sp_core::{blake2_256, crypto::Pair};
use sp_std::prelude::*;
use sp_runtime::{AnySignature, traits::Verify, generic::Era};
#[cfg(feature = "std")]
use std::fmt;

pub type GenericAddress = Address<[u8; 32], u32>;
pub type AccountId = <AnySignature as Verify>::Signer;

/// Simple generic extra mirroring the SignedExtra currently used in extrinsics. Does not implement
/// the SignedExtension trait. It simply encodes to the same bytes as the real SignedExtra. The
/// Order is (CheckVersion, CheckGenesis, Check::Era, CheckNonce, CheckWeight, TakeFees). This can
/// be locked up in the System module. Fields that are merely PhantomData are not encoded and are
/// therefore omitted here.
#[cfg_attr(feature = "std",derive(Debug))]
#[derive(Decode, Encode, Clone, Eq, PartialEq)]
pub struct GenericExtra(Era, Compact<u32>, Compact<u128>);

impl GenericExtra {
    pub fn new(nonce: u32) -> GenericExtra {
        GenericExtra(
            Era::Immortal,
            Compact(nonce),
            Compact(0 as u128), //weight
        )
    }
}

/// additionalSigned fields of the respective SignedExtra fields.
/// Order is the same as declared in the extra.
pub type AdditionalSigned = (u32, H256, H256, (), (), ());

#[derive(Encode)]
pub struct SignedPayload<Call>((Call, GenericExtra, AdditionalSigned));


impl<Call> SignedPayload<Call> where
    Call: Encode ,
{
    pub fn from_raw(call: Call, extra: GenericExtra, additional_signed: AdditionalSigned) -> Self {
        Self((call, extra, additional_signed))
    }

    /// Get an encoded version of this payload.
    ///
    /// Payloads longer than 256 bytes are going to be `blake2_256`-hashed.
    pub fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.0.using_encoded(|payload| {
            if payload.len() > 256 {
                f(&blake2_256(payload)[..])
            } else {
                f(payload)
            }
        })
    }
}

/// Mirrors the currently used Extrinsic format (V3) from substrate. Has less traits and methods though.
/// The SingedExtra used does not need to implement SingedExtension here.
pub struct UncheckedExtrinsicV3<Call, P>
    where
        Call: Encode ,
        P: Pair,
{
    pub signature: Option<(GenericAddress, P::Signature, GenericExtra)>,
    pub function: Call,
}

impl<Call, P> UncheckedExtrinsicV3<Call, P>
    where
        Call: Encode ,
        P: Pair,
        P::Signature: Encode,
{
    pub fn new_signed(
        function: Call,
        signed: GenericAddress,
        signature: P::Signature,
        extra: GenericExtra,
    ) -> Self {
        UncheckedExtrinsicV3 {
            signature: Some((signed, signature, extra)),
            function,
        }
    }
    
    #[cfg(feature = "std")]
    pub fn hex_encode(&self) -> String {
        let mut hex_str = hex::encode(self.encode());
        hex_str.insert_str(0, "0x");
        hex_str
    }
}

#[cfg(feature = "std")]
impl<Call, P> fmt::Debug for UncheckedExtrinsicV3<Call, P>
where
    Call: fmt::Debug + Encode,
    P: Pair,
    P::Signature: Encode,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "UncheckedExtrinsic({:?}, {:?})",
            self.signature.as_ref().map(|x| (&x.0, &x.2)),
            self.function
        )
    }
}

impl<Call, P> Encode for UncheckedExtrinsicV3<Call, P>
    where
        Call: Encode,
        P: Pair,
        P::Signature: Encode,
{
    fn encode(&self) -> Vec<u8> {
        encode_with_vec_prefix::<Self, _>(|v| {
            match self.signature.as_ref() {
                Some(s) => {
                    v.push(3 as u8 | 0b1000_0000);
                    s.encode_to(v);
                }
                None => {
                    v.push(3 as u8 & 0b0111_1111);
                }
            }
            self.function.encode_to(v);
        })
    }
}

/// Same function as in sp_core::generic. Needed to be copied as it is private there.
fn encode_with_vec_prefix<T: Encode, F: Fn(&mut Vec<u8>)>(encoder: F) -> Vec<u8> {
    let size = sp_std::mem::size_of::<T>();
    let reserve = match size {
        0..=0b0011_1111 => 1,
        0..=0b0011_1111_1111_1111 => 2,
        _ => 4,
    };
    let mut v = Vec::with_capacity(reserve + size);
    v.resize(reserve, 0);
    encoder(&mut v);

    // need to prefix with the total length to ensure it's binary compatible with
    // Vec<u8>.
    let mut length: Vec<()> = Vec::new();
    length.resize(v.len() - reserve, ());
    length.using_encoded(|s| {
        v.splice(0..reserve, s.iter().cloned());
    });

    v
}
