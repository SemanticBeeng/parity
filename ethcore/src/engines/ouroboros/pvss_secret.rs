use bincode::{serialize, Infinite};

use pvss;

pub struct PvssSecret {
    simple_secret: PvssSimple,
}

unsafe impl Send for PvssSecret {}
unsafe impl Sync for PvssSecret {}

impl PvssSecret {
    pub fn new(public_keys: &[Vec<u8>]) -> Self {
        PvssSecret {
            simple_secret: PvssSimple::new(public_keys),
        }
    }

    pub fn secret_bytes(&self) -> Vec<u8> {
        self.simple_secret.secret_bytes()
    }

    pub fn commitment_bytes(&self) -> Vec<u8> {
        self.simple_secret.commitment_bytes()
    }

    pub fn share_bytes(&self) -> Vec<u8> {
        self.simple_secret.share_bytes()
    }
}

struct PvssSimple {
    escrow: pvss::simple::Escrow,
    commitments: Vec<pvss::simple::Commitment>,
    shares: Vec<pvss::simple::EncryptedShare>,
}

impl PvssSimple {
    fn new(public_keys: &[Vec<u8>]) -> Self {
        // Calculate the threshold in the same way as cardano does https://github.com/input-output-hk/cardano-sl/blob/9d527fd/godtossing/Pos/Ssc/GodTossing/Functions.hs#L138-L141
        let num_stakeholders = public_keys.len();
        let threshold = num_stakeholders / 2 + num_stakeholders % 2;

        let public_keys: Vec<_> = public_keys.iter().map(|bytes| {
            pvss::crypto::PublicKey::from_bytes(bytes)
        }).collect();

        let escrow = pvss::simple::escrow(threshold as u32);
        let commitments = pvss::simple::commitments(&escrow);
        let shares = pvss::simple::create_shares(&escrow, &public_keys);

        PvssSimple {
            escrow,
            commitments,
            shares,
        }
    }

    fn secret_bytes(&self) -> Vec<u8> {
        serialize(&self.escrow.secret, Infinite).expect("could not serialize secret")
    }

    fn commitment_bytes(&self) -> Vec<u8> {
        serialize(&self.commitments, Infinite).expect("could not serialize commitments")
    }

    fn share_bytes(&self) -> Vec<u8> {
        serialize(&self.shares, Infinite).expect("could not serialize shares")
    }
}
