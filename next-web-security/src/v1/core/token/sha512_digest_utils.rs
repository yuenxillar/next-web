use sha2::{Digest, Sha512};

/// Provides SHA512 digest methods.
/// Based on Commons Codec, which does not presently provide SHA512 support.
pub struct Sha512DigestUtils;

impl Sha512DigestUtils {
    /// Calculates the SHA digest and returns the value as a Vec<u8>.
    pub fn sha(data: impl AsRef<[u8]>) -> Vec<u8> {
        Sha512::digest(data.as_ref()).to_vec()
    }

    /// Calculates the SHA512 digest and returns the value as a hex string.
    pub fn sha_hex(data: impl AsRef<[u8]>) -> String {
        hex::encode(Self::sha(data))
    }
}

#[cfg(test)]
mod tests {
    use super::Sha512DigestUtils;

    #[test]
    fn sha_hex_matches_known_sha512_value() {
        assert_eq!(
            Sha512DigestUtils::sha_hex("abc"),
            "ddaf35a193617abacc417349ae204131\
             12e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd\
             454d4423643ce80e2a9ac94fa54ca49f"
                .replace(' ', "")
        );
    }
}
