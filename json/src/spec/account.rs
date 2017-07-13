// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! Spec account deserialization.

use std::fmt;
use std::collections::BTreeMap;
use uint::Uint;
use bytes::Bytes;
use spec::builtin::Builtin;

use pvss;
use serde::de::{Deserializer, Error};
use serde::Deserialize;
use rustc_serialize::hex::FromHex;

/// Spec account.
#[derive(Debug, PartialEq, Deserialize, Default)]
pub struct Account {
	/// Builtin contract.
	pub builtin: Option<Builtin>,
	/// Balance.
	pub balance: Option<Uint>,
	/// Nonce.
	pub nonce: Option<Uint>,
	/// Code.
	pub code: Option<Bytes>,
	/// Storage.
	pub storage: Option<BTreeMap<Uint, Uint>>,
	/// Constructor.
	pub constructor: Option<Bytes>,

    pub pvss: Option<Pvss>,
}

#[derive(Debug)]
struct HexBytes(Vec<u8>);

impl Deserialize for HexBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer
    {
        let val = String::deserialize(deserializer)?;
        let val = if val.starts_with("0x") { &val[2..] } else { &val };

        FromHex::from_hex(val)
            .map(HexBytes)
            .map_err(|e| {
                D::Error::custom(format!("could not convert from hex: {}. Error: {}", val, e))
            })
    }
}

#[derive(PartialEq)]
pub struct Pvss {
    pub public_key: Option<pvss::crypto::PublicKey>,
    pub private_key: Option<pvss::crypto::PrivateKey>,
}

impl Deserialize for Pvss {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer
    {
        #[derive(Debug, Deserialize)]
        struct Inner {
            public_key: Option<HexBytes>,
            private_key: Option<HexBytes>,
        }

        let val = Inner::deserialize(deserializer)?;

        let public_key = val.public_key.map(|bytes| pvss::crypto::PublicKey::from_bytes(&bytes.0));
        let private_key = val.private_key.map(|bytes| pvss::crypto::PrivateKey::from_bytes(&bytes.0));

        Ok(Pvss { public_key, private_key })
    }
}

impl fmt::Debug for Pvss {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        "Pvss {{ hidden }}".fmt(f)
    }
}

impl Account {
	/// Returns true if account does not have nonce and balance.
	pub fn is_empty(&self) -> bool {
		self.balance.is_none() && self.nonce.is_none() && self.code.is_none() && self.storage.is_none()
	}
}

#[cfg(test)]
mod tests {
	use std::collections::BTreeMap;
	use serde_json;
	use spec::account::Account;
	use util::U256;
	use uint::Uint;
	use bytes::Bytes;
    use pvss;
    use rustc_serialize::hex::FromHex;

	#[test]
	fn account_deserialization() {
		let s = r#"{
			"balance": "1",
			"nonce": "0",
			"builtin": { "name": "ecrecover", "pricing": { "linear": { "base": 3000, "word": 0 } } },
			"code": "1234"
		}"#;
		let deserialized: Account = serde_json::from_str(s).unwrap();
		assert_eq!(deserialized.balance.unwrap(), Uint(U256::from(1)));
		assert_eq!(deserialized.nonce.unwrap(), Uint(U256::from(0)));
		assert_eq!(deserialized.code.unwrap(), Bytes::new(vec![0x12, 0x34]));
		assert!(deserialized.builtin.is_some()); // Further tested in builtin.rs
	}

    #[test]
    fn account_pvss_deserialization() {
		let s = r#"{
			"balance": "1",
			"nonce": "0",
			"builtin": { "name": "ecrecover", "pricing": { "linear": { "base": 3000, "word": 0 } } },
            "pvss": {
              "public_key": "0x04823124f450ea06b3e1fcffadbebac9e3d00bc6531f23b4184b8a110f63b6f7596dd1a690c592c05755584fa1860d704be9add478575cd067906afde0d2df9085",
              "private_key": "0xfff1b7d4a600d44039402d26ecadcbc8da456d8be96b4090af9791adb7a7584b"
            },
			"code": "1234"
		}"#;

		let deserialized: Account = serde_json::from_str(s).unwrap();

        let pvss = deserialized.pvss.unwrap();
        assert!(pvss.private_key ==
            Some(pvss::crypto::PrivateKey::from_bytes(
                &FromHex::from_hex("fff1b7d4a600d44039402d26ecadcbc8da456d8be96b4090af9791adb7a7584b").unwrap())));
        assert!(pvss.public_key ==
            Some(pvss::crypto::PublicKey::from_bytes(
                &FromHex::from_hex("04823124f450ea06b3e1fcffadbebac9e3d00bc6531f23b4184b8a110f63b6f7596dd1a690c592c05755584fa1860d704be9add478575cd067906afde0d2df9085").unwrap())));

		assert_eq!(deserialized.balance.unwrap(), Uint(U256::from(1)));
		assert_eq!(deserialized.nonce.unwrap(), Uint(U256::from(0)));
		assert_eq!(deserialized.code.unwrap(), Bytes::new(vec![0x12, 0x34]));
		assert!(deserialized.builtin.is_some()); // Further tested in builtin.rs
    }

	#[test]
	fn account_storage_deserialization() {
		let s = r#"{
			"balance": "1",
			"nonce": "0",
			"code": "1234",
			"storage": { "0x7fffffffffffffff7fffffffffffffff": "0x1" }
		}"#;
		let deserialized: Account = serde_json::from_str(s).unwrap();
		assert_eq!(deserialized.balance.unwrap(), Uint(U256::from(1)));
		assert_eq!(deserialized.nonce.unwrap(), Uint(U256::from(0)));
		assert_eq!(deserialized.code.unwrap(), Bytes::new(vec![0x12, 0x34]));
		let mut storage = BTreeMap::new();
		storage.insert(Uint(U256::from("7fffffffffffffff7fffffffffffffff")), Uint(U256::from(1)));
		assert_eq!(deserialized.storage.unwrap(), storage);
	}
}
