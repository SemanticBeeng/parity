use pvss;

use engines::validator_set::ValidatorSet;

pub struct PvssSecret {
    escrow: pvss::simple::Escrow,
    commitments: Vec<pvss::simple::Commitment>,
    shares: Vec<pvss::simple::EncryptedShare>,
}

unsafe impl Send for PvssSecret {}
unsafe impl Sync for PvssSecret {}

impl PvssSecret {
    pub fn new(validator_set: &Box<ValidatorSet>, public_keys: &[Vec<u8>]) -> Self {
        // Calculate the threshold in the same way as cardano does https://github.com/input-output-hk/cardano-sl/blob/9d527fd/godtossing/Pos/Ssc/GodTossing/Functions.hs#L138-L141
        let num_validators = validator_set.validators().len();
        let threshold = num_validators / 2 + num_validators % 2;

        let public_keys: Vec<_> = public_keys.iter().map(|bytes| {
            pvss::crypto::PublicKey::from_bytes(bytes)
        }).collect();

        let escrow = pvss::simple::escrow(threshold as u32);
        let commitments = pvss::simple::commitments(&escrow);
        let shares = pvss::simple::create_shares(&escrow, &public_keys);

        PvssSecret {
            escrow,
            commitments,
            shares,
        }
    }

    pub fn escrow(&self) -> &pvss::simple::Escrow {
        &self.escrow
    }

    pub fn commitments(&self) -> &[pvss::simple::Commitment] {
        &self.commitments
    }

    pub fn shares(&self) -> &[pvss::simple::EncryptedShare] {
        &self.shares
    }
}
