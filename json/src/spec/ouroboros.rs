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

/// Which method of PVSS to use
#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PvssMethodParam {
    Simple,
    Scrape,
}

impl Default for PvssMethodParam {
    fn default() -> Self {
        PvssMethodParam::Scrape
    }
}

/// Ouroboros params deserialization.
#[derive(Debug, PartialEq, Deserialize)]
pub struct OuroborosParams {
	/// Time to wait before next block or authority switching, in seconds.
    /// Equivalent to slot duration in the Ouroboros paper.
	#[serde(rename="stepDuration")]
	pub step_duration: Uint,
	/// Validators. Equivalent to stakeholders/leaders in the Ouroboros paper.
	pub validators: ValidatorSet,
    /// Security parameter k. A transaction is declared stable if and only if
    /// it is in a block that is more than this many blocks deep in the
    /// ledger. Equivalent to blkSecurityParam in cardano.
    #[serde(rename="securityParameterK")]
    pub security_parameter_k: u64,
    /// The mutually agreed-on time of when the entire chain came into being.
    #[serde(rename="networkWideStartTime")]
    pub network_wide_start_time: Option<Uint>,
    /// Whether to use pvss::simple or pvss::scrape
    #[serde(rename="pvssMethod", default)]
    pub pvss_method: PvssMethodParam,
	/// Starting step. Determined automatically if not specified.
	/// To be used for testing only, similarly to how Authority Round is tested.
	#[serde(rename="startStep")]
	pub start_step: Option<Uint>,
	/// Gas limit divisor. Needed by Parity/Authority Round, so including to be comparable.
	#[serde(rename="gasLimitBoundDivisor")]
	pub gas_limit_bound_divisor: Uint,
	/// Number of first block where EIP-155 rules are validated.
    /// Needed by Parity/Authority Round, so including to be comparable.
	#[serde(rename="eip155Transition")]
	pub eip155_transition: Option<Uint>,
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
    use spec::ouroboros::PvssMethodParam;

	#[test]
	fn ouroboros_deserialization() {
		let s = r#"{
			"params": {
				"gasLimitBoundDivisor": "0x0400",
				"stepDuration": "0x02",
                "networkWideStartTime": "0x596d1d34",
				"validators": {
					"list" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				},
                "securityParameterK": 60,
				"eip155Transition": "0x42"
			}
		}"#;

		let deserialized: Ouroboros = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized.params.pvss_method, PvssMethodParam::Scrape);
	}

	#[test]
	fn ouroboros_deserialization_with_pvss_method() {
		let s = r#"{
			"params": {
				"gasLimitBoundDivisor": "0x0400",
				"stepDuration": "0x02",
                "networkWideStartTime": "0x596d1d34",
                "pvssMethod": "simple",
				"validators": {
					"list" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				},
                "securityParameterK": 60,
				"eip155Transition": "0x42"
			}
		}"#;

		let deserialized: Ouroboros = serde_json::from_str(s).unwrap();
        assert_eq!(deserialized.params.pvss_method, PvssMethodParam::Simple);
	}

	#[test]
	fn ouroboros_deserialization_with_invalid_pvss_method() {
		let s = r#"{
			"params": {
				"gasLimitBoundDivisor": "0x0400",
				"stepDuration": "0x02",
                "networkWideStartTime": "0x596d1d34",
                "pvssMethod": "invalid",
				"validators": {
					"list" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				},
                "securityParameterK": 60,
				"eip155Transition": "0x42"
			}
		}"#;

		let deserialized: Result<Ouroboros, _> = serde_json::from_str(s);
        assert!(deserialized.is_err());
	}
}
