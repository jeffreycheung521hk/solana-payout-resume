//! Payout records: everything written to disk about one payout.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Why base58 text could not be turned into fixed-size bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base58Error {
    /// The text is not valid base58.
    NotBase58,
    /// The text decoded to the wrong number of bytes.
    WrongLength {
        /// Bytes the type needs.
        expected: usize,
        /// Bytes the text decoded to.
        found: usize,
    },
}

impl fmt::Display for Base58Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotBase58 => f.write_str("not valid base58"),
            Self::WrongLength { expected, found } => {
                write!(f, "decoded to {found} bytes, expected {expected}")
            }
        }
    }
}

impl std::error::Error for Base58Error {}

macro_rules! base58_array {
    ($(#[$doc:meta])* $name:ident, $len:literal) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub [u8; $len]);

        impl $name {
            /// Parses base58 text that must decode to exactly the type's length.
            pub fn from_base58(text: &str) -> Result<Self, Base58Error> {
                let bytes = bs58::decode(text)
                    .into_vec()
                    .map_err(|_| Base58Error::NotBase58)?;
                let found = bytes.len();
                let array = <[u8; $len]>::try_from(bytes)
                    .map_err(|_| Base58Error::WrongLength { expected: $len, found })?;
                Ok(Self(array))
            }

            /// Encodes the bytes as base58.
            pub fn to_base58(&self) -> String {
                bs58::encode(&self.0).into_string()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.to_base58())
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.to_base58())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.to_base58())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                Self::from_base58(&text).map_err(serde::de::Error::custom)
            }
        }
    };
}

base58_array!(
    /// A 32-byte Solana address, such as a wallet owner or a mint, written in base58.
    Address,
    32
);

base58_array!(
    /// A transaction's first signature, which is also the transaction's id, written in base58.
    Signature,
    64
);

/// One signed transaction of a payout, written to the record before it is broadcast.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attempt {
    /// The transaction's first signature.
    pub signature: Signature,
    /// The signed transaction bytes, resent unchanged on every rebroadcast.
    #[serde(with = "base58_bytes")]
    pub signed_tx: Vec<u8>,
    /// Once the finalized block height passes this, the transaction can never be processed.
    pub last_valid_block_height: u64,
    /// The slot at which the blockhash was fetched; the transaction cannot land before it.
    pub blockhash_fetch_slot: u64,
}

/// Everything written about one payout: what to pay, and every attempt so far.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayoutRecord {
    /// Business identifier of the payout, such as an invoice number.
    pub payout_id: String,
    /// Wallet owner who pays.
    pub sender_owner: Address,
    /// Wallet owner who receives; a wallet owner, not a token account.
    pub recipient_owner: Address,
    /// Token mint, such as USDC.
    pub mint: Address,
    /// Amount in the mint's smallest unit.
    pub amount_atomic: u64,
    /// Attempts in the order they were signed.
    pub attempts: Vec<Attempt>,
}

impl PayoutRecord {
    /// Creates a record that has no attempts yet.
    pub fn new(
        payout_id: impl Into<String>,
        sender_owner: Address,
        recipient_owner: Address,
        mint: Address,
        amount_atomic: u64,
    ) -> Self {
        Self {
            payout_id: payout_id.into(),
            sender_owner,
            recipient_owner,
            mint,
            amount_atomic,
            attempts: Vec::new(),
        }
    }

    /// Finds the attempt with this signature.
    pub fn attempt(&self, signature: &Signature) -> Option<&Attempt> {
        self.attempts
            .iter()
            .find(|attempt| attempt.signature == *signature)
    }
}

/// Serializes byte vectors as base58 text so the record stays readable.
mod base58_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&bs58::encode(bytes).into_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(deserializer)?;
        bs58::decode(&text)
            .into_vec()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> PayoutRecord {
        let mut record = PayoutRecord::new(
            "INV-001",
            Address([1; 32]),
            Address([2; 32]),
            Address([3; 32]),
            120_000_000,
        );
        record.attempts.push(Attempt {
            signature: Signature([7; 64]),
            signed_tx: vec![0, 1, 2, 255],
            last_valid_block_height: 1_150,
            blockhash_fetch_slot: 1_000,
        });
        record
    }

    #[test]
    fn base58_round_trips_for_both_lengths() {
        let address = Address([9; 32]);
        assert_eq!(Address::from_base58(&address.to_base58()), Ok(address));
        let signature = Signature([4; 64]);
        assert_eq!(
            Signature::from_base58(&signature.to_base58()),
            Ok(signature)
        );
    }

    #[test]
    fn base58_rejects_wrong_length_and_invalid_text() {
        let short = Address([9; 32]).to_base58();
        assert_eq!(
            Signature::from_base58(&short),
            Err(Base58Error::WrongLength {
                expected: 64,
                found: 32
            })
        );
        assert_eq!(Address::from_base58("0OIl"), Err(Base58Error::NotBase58));
    }

    #[test]
    fn record_round_trips_through_json_with_base58_fields() {
        let record = sample_record();
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains(&Signature([7; 64]).to_base58()));
        assert!(json.contains(&format!(
            "\"signed_tx\":\"{}\"",
            bs58::encode([0, 1, 2, 255]).into_string()
        )));
        let back: PayoutRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }

    #[test]
    fn attempt_lookup_finds_by_signature() {
        let record = sample_record();
        assert!(record.attempt(&Signature([7; 64])).is_some());
        assert!(record.attempt(&Signature([8; 64])).is_none());
    }
}
