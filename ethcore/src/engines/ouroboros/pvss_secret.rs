use bincode::{serialize, Infinite};

use pvss;

/// Which method of PVSS to use
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum PvssMethod {
    Simple,
    Scrape,
}

enum SecretType {
    Simple(PvssSimple),
    Scrape(PvssScrape),
}

pub struct PvssSecret {
    secret_type: SecretType,
}

unsafe impl Send for PvssSecret {}
unsafe impl Sync for PvssSecret {}

impl PvssSecret {
    pub fn new(pvss_method: PvssMethod, public_keys: &[Vec<u8>]) -> Self {
        match pvss_method {
            PvssMethod::Simple => PvssSecret {
                secret_type: SecretType::Simple(PvssSimple::new(public_keys))
            },
            PvssMethod::Scrape =>PvssSecret {
                secret_type: SecretType::Scrape(PvssScrape::new(public_keys))
            },
        }
    }

    pub fn secret_bytes(&self) -> Vec<u8> {
        match self.secret_type {
            SecretType::Simple(ref simple) => simple.secret_bytes(),
            SecretType::Scrape(ref scrape) => scrape.secret_bytes(),
        }
    }

    pub fn commitment_bytes(&self) -> Vec<u8> {
        match self.secret_type {
            SecretType::Simple(ref simple) => simple.commitment_bytes(),
            SecretType::Scrape(ref scrape) => scrape.commitment_bytes(),
        }
    }

    pub fn share_bytes(&self) -> Vec<u8> {
        match self.secret_type {
            SecretType::Simple(ref simple) => simple.share_bytes(),
            SecretType::Scrape(ref scrape) => scrape.share_bytes(),
        }
    }

    pub fn verify_encrypted(&self) -> bool {
        match self.secret_type {
            SecretType::Simple(ref simple) => simple.verify_encrypted(),
            SecretType::Scrape(ref scrape) => scrape.verify_encrypted(),
        }
    }
}

struct PvssSimple {
    public_keys: Vec<pvss::crypto::PublicKey>,
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
            public_keys,
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

    pub fn verify_encrypted(&self) -> bool {
        for share in &self.shares {
            // TODO: investigate why pvss::simple's share.verify needs the share.id passed in
            // when it's coming from the share anyway.......
            let idx = share.id as usize;
            if share.verify(
                    share.id,
                    &self.public_keys[idx],
                    &self.escrow.extra_generator,
                    &self.commitments
            ) {
                continue;
            } else {
                return false;
            }
        }
        true
    }
}

pub struct PvssScrape {
    public_keys: Vec<pvss::crypto::PublicKey>,
    escrow: pvss::scrape::Escrow,
    public_shares: pvss::scrape::PublicShares,
}

impl PvssScrape {
    pub fn new(public_keys: &[Vec<u8>]) -> Self {
        // Calculate the threshold in the same way as cardano does https://github.com/input-output-hk/cardano-sl/blob/9d527fd/godtossing/Pos/Ssc/GodTossing/Functions.hs#L138-L141
        let num_stakeholders = public_keys.len();
        // error "cannot create SCRAPE with less than 3 participants"
        // https://github.com/input-output-hk/pvss-haskell/blob/master/src/Crypto/SCRAPE.hs#L166
        assert!(num_stakeholders > 2, "cannot create SCRAPE with fewer than 3 participants");
        let threshold = num_stakeholders / 2 + num_stakeholders % 2;

        // error valid values of threshold are: threshold + 2 <= num participants
        // https://github.com/input-output-hk/pvss-haskell/blob/master/src/Crypto/SCRAPE.hs#L167
        assert!(
            threshold + 2 <= num_stakeholders,
            format!(
                "cannot create SCRAPE with threshold={} participants={}. valid values of threshold are: t + 2 <= n",
                threshold,
                num_stakeholders
            )
        );

        let public_keys: Vec<_> = public_keys.iter().map(|bytes| {
            pvss::crypto::PublicKey::from_bytes(bytes)
        }).collect();

        let escrow = pvss::scrape::escrow(threshold as u32);
        let public_shares = pvss::scrape::create_shares(&escrow, &public_keys);

        PvssScrape {
            public_keys,
            escrow,
            public_shares,
        }
    }

    fn secret_bytes(&self) -> Vec<u8> {
        serialize(&self.escrow.secret, Infinite).expect("could not serialize secret")
    }

    fn commitment_bytes(&self) -> Vec<u8> {
        serialize(&self.public_shares.commitments, Infinite)
            .expect("could not serialize commitments")
    }

    fn share_bytes(&self) -> Vec<u8> {
        serialize(&self.public_shares.encrypted_shares, Infinite)
            .expect("could not serialize shares")
    }

    pub fn verify_encrypted(&self) -> bool {
        self.public_shares.verify(&self.public_keys)
    }
}

#[cfg(test)]
mod tests {
    use super::{PvssSecret, PvssMethod};
    use rustc_serialize::hex::FromHex;

    fn public_keys() -> Vec<Vec<u8>> {
        vec![
            FromHex::from_hex(
                "04823124f450ea06b3e1fcffadbebac9e3d00bc6531f23b4184b8a110f63b6f7596dd1a690c592c05755584fa1860d704be9add478575cd067906afde0d2df9085"
            ).unwrap(),
            FromHex::from_hex(
                "04343e9b4a46c221fdf15d5b3a8ff720a09d1880eec9d5ac91f89ac2b7ab307f548f08ea9a749b1a86fc13d1ca837026ada06dcfcf59d88e1b7330e8e038047ed2"
            ).unwrap(),
            FromHex::from_hex(
                "0316ab5795f85d121bc38f1b72a3ef8e322b223ea4dcaace1a6afa24ce2e76015a"
            ).unwrap(),
            FromHex::from_hex(
                "0371ce6e8d367b0b7f3c2da66613e03df62d00f00639fa151411418d66dad7ddb3"
            ).unwrap(),
        ]
    }

    #[test]
    fn simple_with_two_keys_validates() {
        let pvss_secret = PvssSecret::new(PvssMethod::Simple, &public_keys()[..2]);
        assert!(pvss_secret.verify_encrypted());
    }

    #[test]
    #[should_panic(expected = "cannot create SCRAPE with fewer than 3 participants")]
    fn scrape_with_two_keys_panics() {
        let pvss_secret = PvssSecret::new(PvssMethod::Scrape, &public_keys()[..2]);
        assert!(pvss_secret.verify_encrypted());
    }

    #[test]
    fn simple_with_three_keys_validates() {
        let pvss_secret = PvssSecret::new(PvssMethod::Simple, &public_keys()[..3]);
        assert!(pvss_secret.verify_encrypted());
    }

    #[test]
    #[should_panic(expected = "cannot create SCRAPE with threshold=2 participants=3. valid values of threshold are: t + 2 <= n")]
    fn scrape_with_three_keys_panics() {
        let pvss_secret = PvssSecret::new(PvssMethod::Scrape, &public_keys()[..3]);
        assert!(pvss_secret.verify_encrypted());
    }

    #[test]
    fn scrape_with_four_keys_validates() {
        let pvss_secret = PvssSecret::new(PvssMethod::Scrape, &public_keys()[..4]);
        assert!(pvss_secret.verify_encrypted());
    }
}