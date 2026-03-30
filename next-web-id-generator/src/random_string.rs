use rand::Rng;

use crate::error::IdGeneratorError;
use crate::generator::IdGenerator;

const BASE62_ALPHABET: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// Generates random string IDs from a configurable alphabet.
#[derive(Debug, Clone)]
pub struct RandomStringIdGenerator {
    alphabet: Vec<char>,
    length: usize,
}

impl RandomStringIdGenerator {
    /// Creates a generator using the default base62 alphabet.
    pub fn base62(length: usize) -> Result<Self, IdGeneratorError> {
        Self::new(BASE62_ALPHABET, length)
    }

    /// Creates a generator using a custom alphabet.
    pub fn new(alphabet: impl AsRef<str>, length: usize) -> Result<Self, IdGeneratorError> {
        let alphabet: Vec<char> = alphabet.as_ref().chars().collect();
        if alphabet.is_empty() {
            return Err(IdGeneratorError::EmptyAlphabet);
        }
        if length == 0 {
            return Err(IdGeneratorError::InvalidLength);
        }

        Ok(Self { alphabet, length })
    }

    /// Returns the configured output length.
    pub fn length(&self) -> usize {
        self.length
    }

    /// Returns the configured alphabet.
    pub fn alphabet(&self) -> &[char] {
        &self.alphabet
    }
}

impl IdGenerator<String> for RandomStringIdGenerator {
    fn next_id(&self) -> Result<String, IdGeneratorError> {
        let mut rng = rand::thread_rng();
        let mut output = String::with_capacity(self.length);

        for _ in 0..self.length {
            let idx = rng.gen_range(0..self.alphabet.len());
            output.push(self.alphabet[idx]);
        }

        Ok(output)
    }
}
