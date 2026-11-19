//! Export payout attempts as input CSV for web3testproj1 (`payment-reconcile`).
//!
//! web3testproj1 owns the input format; this module only produces it. Every attempt becomes
//! one row, so that the reconciler can independently notice a payout that was paid twice.

use crate::decision::NextAction;
use crate::record::{Attempt, PayoutRecord};
use std::fmt;

/// The header web3testproj1 requires, in its fixed order.
pub const RECONCILE_CSV_HEADER: &str = "payment_id,tx_signature,expected_sender_owner,expected_recipient_owner,expected_mint,expected_amount_atomic";

/// Longest payment id web3testproj1 accepts, in bytes.
pub const MAX_PAYMENT_ID_LEN: usize = 256;

/// Why a payout cannot be written in the format web3testproj1 accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExportError {
    /// The payout id is empty or longer than [`MAX_PAYMENT_ID_LEN`] bytes.
    PayoutIdLength(String),
    /// The payout id contains a line break, which the format does not allow inside a field.
    PayoutIdLineBreak(String),
    /// The amount is zero; the format accepts amounts from 1 upward.
    ZeroAmount(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayoutIdLength(id) => write!(
                f,
                "payout id {id:?} must be 1 to {MAX_PAYMENT_ID_LEN} bytes"
            ),
            Self::PayoutIdLineBreak(id) => write!(f, "payout id {id:?} contains a line break"),
            Self::ZeroAmount(id) => write!(f, "payout {id:?} has a zero amount"),
        }
    }
}

impl std::error::Error for ExportError {}

/// CSV rows, without the header, for one payout: one row per attempt. The attempt the decision
/// calls done comes first; the others follow in signing order. A payout that is not done keeps
/// signing order throughout.
pub fn reconcile_rows(
    record: &PayoutRecord,
    action: &NextAction,
) -> Result<Vec<String>, ExportError> {
    validate(record)?;
    let mut attempts: Vec<&Attempt> = record.attempts.iter().collect();
    if let NextAction::Done { signature } = action {
        // A stable sort keeps the remaining attempts in signing order.
        attempts.sort_by_key(|attempt| attempt.signature != *signature);
    }
    let payment_id = csv_field(&record.payout_id);
    Ok(attempts
        .into_iter()
        .map(|attempt| {
            format!(
                "{payment_id},{},{},{},{},{}",
                attempt.signature,
                record.sender_owner,
                record.recipient_owner,
                record.mint,
                record.amount_atomic
            )
        })
        .collect())
}

/// A complete CSV, header included, for several payouts.
pub fn reconcile_csv<'a, I>(payouts: I) -> Result<String, ExportError>
where
    I: IntoIterator<Item = (&'a PayoutRecord, &'a NextAction)>,
{
    let mut csv = format!("{RECONCILE_CSV_HEADER}\n");
    for (record, action) in payouts {
        for row in reconcile_rows(record, action)? {
            csv.push_str(&row);
            csv.push('\n');
        }
    }
    Ok(csv)
}

fn validate(record: &PayoutRecord) -> Result<(), ExportError> {
    let id = &record.payout_id;
    if id.is_empty() || id.len() > MAX_PAYMENT_ID_LEN {
        return Err(ExportError::PayoutIdLength(id.clone()));
    }
    if id.contains(['\n', '\r']) {
        return Err(ExportError::PayoutIdLineBreak(id.clone()));
    }
    if record.amount_atomic == 0 {
        return Err(ExportError::ZeroAmount(id.clone()));
    }
    Ok(())
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Address, Signature};

    fn record_with_attempts(id: &str, attempts: u8) -> PayoutRecord {
        let mut record = PayoutRecord::new(
            id,
            Address([1; 32]),
            Address([2; 32]),
            Address([3; 32]),
            120_000_000,
        );
        record.attempts = (1..=attempts)
            .map(|n| Attempt {
                signature: Signature([n; 64]),
                signed_tx: vec![n],
                last_valid_block_height: 1_150,
                blockhash_fetch_slot: 1_000,
            })
            .collect();
        record
    }

    fn signature_column(row: &str) -> Signature {
        Signature::from_base58(row.split(',').nth(1).unwrap()).unwrap()
    }

    #[test]
    fn header_matches_web3testproj1() {
        let csv = reconcile_csv([]).unwrap();
        assert_eq!(
            csv,
            "payment_id,tx_signature,expected_sender_owner,expected_recipient_owner,expected_mint,expected_amount_atomic\n"
        );
    }

    #[test]
    fn done_attempt_comes_first_and_the_rest_keep_signing_order() {
        let record = record_with_attempts("INV-001", 3);
        let rows = reconcile_rows(
            &record,
            &NextAction::Done {
                signature: Signature([2; 64]),
            },
        )
        .unwrap();
        let order: Vec<_> = rows.iter().map(|row| signature_column(row)).collect();
        assert_eq!(
            order,
            [Signature([2; 64]), Signature([1; 64]), Signature([3; 64])]
        );
    }

    #[test]
    fn a_payout_that_is_not_done_keeps_signing_order() {
        let record = record_with_attempts("INV-001", 2);
        let rows = reconcile_rows(&record, &NextAction::SignNewAttempt).unwrap();
        let order: Vec<_> = rows.iter().map(|row| signature_column(row)).collect();
        assert_eq!(order, [Signature([1; 64]), Signature([2; 64])]);
    }

    #[test]
    fn row_fields_are_base58_and_a_plain_integer_amount() {
        let record = record_with_attempts("INV-001", 1);
        let rows = reconcile_rows(
            &record,
            &NextAction::Done {
                signature: Signature([1; 64]),
            },
        )
        .unwrap();
        let fields: Vec<_> = rows[0].split(',').collect();
        assert_eq!(fields[0], "INV-001");
        assert_eq!(Address::from_base58(fields[2]), Ok(Address([1; 32])));
        assert_eq!(Address::from_base58(fields[3]), Ok(Address([2; 32])));
        assert_eq!(Address::from_base58(fields[4]), Ok(Address([3; 32])));
        assert_eq!(fields[5], "120000000");
    }

    #[test]
    fn ids_with_commas_or_quotes_are_quoted() {
        let record = record_with_attempts("INV,\"1\"", 1);
        let rows = reconcile_rows(&record, &NextAction::SignNewAttempt).unwrap();
        assert!(rows[0].starts_with("\"INV,\"\"1\"\"\","));
    }

    #[test]
    fn values_the_format_rejects_are_errors() {
        let mut zero = record_with_attempts("INV-001", 1);
        zero.amount_atomic = 0;
        assert_eq!(
            reconcile_rows(&zero, &NextAction::SignNewAttempt),
            Err(ExportError::ZeroAmount("INV-001".into()))
        );
        let long = "a".repeat(MAX_PAYMENT_ID_LEN + 1);
        assert!(matches!(
            reconcile_rows(&record_with_attempts(&long, 1), &NextAction::SignNewAttempt),
            Err(ExportError::PayoutIdLength(_))
        ));
        assert!(matches!(
            reconcile_rows(
                &record_with_attempts("INV\n1", 1),
                &NextAction::SignNewAttempt
            ),
            Err(ExportError::PayoutIdLineBreak(_))
        ));
    }
}
