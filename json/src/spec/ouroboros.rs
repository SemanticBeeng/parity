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

//! Ouroboros params deserialization.

use uint::Uint;
use super::ValidatorSet;

/// Ouroboros params deserialization.
#[derive(Debug, PartialEq, Deserialize)]
pub struct OuroborosParams {
	/// Slot duration.
	#[serde(rename="slotDuration")]
	pub slot_duration: Uint,
	/// Valid authorities
	pub validators: ValidatorSet,
}

/// Ouroboros engine deserialization.
#[derive(Debug, PartialEq, Deserialize)]
pub struct Ouroboros {
	/// Ethash params.
	pub params: OuroborosParams,
}

#[cfg(test)]
mod tests {
	use serde_json;
	use spec::ouroboros::Ouroboros;

	#[test]
	fn ouroboros_deserialization() {
		let s = r#"{
			"params": {
				"slotDuration": "0x02",
				"validators": {
					"list" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				}
			}
		}"#;

		let _deserialized: Ouroboros = serde_json::from_str(s).unwrap();
	}
}
