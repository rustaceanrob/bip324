//! HMAC-based Extract-and-Expand Key Derivation Function (HKDF).
//!
//! The interface is limited to the BIP324 use case for now. This
//! includes hardcoding to the SHA256 hash implementation, as well
//! as requiring an extract step.

use bitcoin_hashes::{sha256, Hash, HashEngine, Hmac, HmacEngine};
use core::fmt;

// Hardcoded hash length for SHA256 backed implementation.
const HASH_LENGTH_BYTES: usize = sha256::Hash::LEN;
// Output keying material max length multiple.
const MAX_OUTPUT_BLOCKS: usize = 255;

#[derive(Copy, Clone, Debug)]
pub struct InvalidLength;

impl fmt::Display for InvalidLength {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "too large output")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidLength {}

/// HMAC-based Extract-and-Expand Key Derivation Function (HKDF).
pub struct Hkdf {
    /// Pseudorandom key based on the extract step.
    prk: [u8; HASH_LENGTH_BYTES],
}

impl Hkdf {
    /// Initialize a HKDF by performing the extract step.
    pub fn extract(salt: &[u8], ikm: &[u8]) -> Self {
        // Hardcoding SHA256 for now, might be worth parameterizing hash function.
        let mut hmac_engine: HmacEngine<sha256::Hash> = HmacEngine::new(salt);
        hmac_engine.input(ikm);
        Self {
            prk: Hmac::from_engine(hmac_engine).to_byte_array(),
        }
    }

    /// Expand the key to generate output key material in okm.
    pub fn expand(&self, info: &[u8], okm: &mut [u8]) -> Result<(), InvalidLength> {
        // Length of output keying material must be less than 255 * hash length.
        if okm.len() > (MAX_OUTPUT_BLOCKS * HASH_LENGTH_BYTES) {
            return Err(InvalidLength);
        }

        // Counter starts at "1" based on RFC5869 spec and is committed to in the hash.
        let mut counter = 1u8;
        // Ceiling calculation for the total number of blocks (iterations) required for the expand.
        let total_blocks = (okm.len() + HASH_LENGTH_BYTES - 1) / HASH_LENGTH_BYTES;

        while counter <= total_blocks as u8 {
            let mut hmac_engine: HmacEngine<sha256::Hash> = HmacEngine::new(&self.prk);

            // First block does not have a previous block,
            // all other blocks include last block in the HMAC input.
            if counter != 1u8 {
                let previous_start_index = (counter as usize - 2) * HASH_LENGTH_BYTES;
                let previous_end_index = (counter as usize - 1) * HASH_LENGTH_BYTES;
                hmac_engine.input(&okm[previous_start_index..previous_end_index]);
            }
            hmac_engine.input(info);
            hmac_engine.input(&[counter]);

            let t = Hmac::from_engine(hmac_engine);
            let start_index = (counter as usize - 1) * HASH_LENGTH_BYTES;
            // Last block might not take full hash length.
            let end_index = if counter == (total_blocks as u8) {
                okm.len()
            } else {
                counter as usize * HASH_LENGTH_BYTES
            };

            okm[start_index..end_index]
                .copy_from_slice(&t.to_byte_array()[0..(end_index - start_index)]);

            counter += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc5869_basic() {
        let salt = hex::decode("000102030405060708090a0b0c").unwrap();
        let ikm = hex::decode("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b").unwrap();
        let info = hex::decode("f0f1f2f3f4f5f6f7f8f9").unwrap();

        let hkdf = Hkdf::extract(&salt, &ikm);
        let mut okm = [0u8; 42];
        hkdf.expand(&info, &mut okm).unwrap();

        assert_eq!(
            hex::encode(okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );
    }

    #[test]
    fn test_rfc5869_longer_inputs_outputs() {
        let salt = hex::decode(
            "606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeaf"
        ).unwrap();
        let ikm = hex::decode(
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f"
        ).unwrap();
        let info = hex::decode(
            "b0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"
        ).unwrap();

        let hkdf = Hkdf::extract(&salt, &ikm);
        let mut okm = [0u8; 82];
        hkdf.expand(&info, &mut okm).unwrap();

        assert_eq!(
            hex::encode(okm),
            "b11e398dc80327a1c8e7f78c596a49344f012eda2d4efad8a050cc4c19afa97c59045a99cac7827271cb41c65e590e09da3275600c2f09b8367793a9aca3db71cc30c58179ec3e87c14c01d5c1f3434f1d87"
        );
    }

    #[test]
    fn test_too_long_okm() {
        let salt = hex::decode("000102030405060708090a0b0c").unwrap();
        let ikm = hex::decode("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b").unwrap();
        let info = hex::decode("f0f1f2f3f4f5f6f7f8f9").unwrap();

        let hkdf = Hkdf::extract(&salt, &ikm);
        let mut okm = [0u8; 256 * 32];
        let e = hkdf.expand(&info, &mut okm);

        assert!(e.is_err());
    }
}
