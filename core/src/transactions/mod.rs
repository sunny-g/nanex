use std::collections::HashMap;
use hdk::holochain_core_types::{
    error::HolochainError,
    json::JsonString,
};
use holochain_core_types_derive::DefaultJson;
use quick_error::quick_error;
use serde_derive::{Serialize, Deserialize};
use self::{
    base::*,
    basic::{Error as BasicError, *},
    start_htl::{Error as StartHTLError, *},
    end_htl::{Error as EndHTLError, *},
};

pub mod base;
pub mod basic;
pub mod start_htl;
pub mod end_htl;

/// Stores all transactions relevant to multiple ledgers' histories
pub type TransactionMap<'a> =
    HashMap<TransactionId<'a>, MultiLedgerTransaction<'a>>;

pub const EntryType: &'static str = "transaction";

// TODO: #[derive(Serialize, Deserialize, DefaultJson, Debug)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum MultiLedgerTransaction<'a> {
    #[serde(borrow)]
    Basic(BasicTransaction<'a>),
    #[serde(borrow)]
    StartHTL(StartHTLTransaction<'a>),
    #[serde(borrow)]
    EndHTL(EndHTLTransaction<'a>),
}

impl<'a> MultiLedgerTransaction<'a> {
    /// Validates and applies the transaction and it's operations against the
    /// ledgers available in `MultiLedgerHistory`
    pub fn validate_and_apply<H: MultiLedgerHistory>(
        &self,
        transactions: &TransactionMap,
        multiledger_history: H,
    ) -> Result<H, Error> {
        self.is_valid_seq_no(&multiledger_history)?;

        match self {
            MultiLedgerTransaction::Basic(tx) => tx
                .validate_and_apply(multiledger_history)
                .map_err(Error::BasicError),
            MultiLedgerTransaction::StartHTL(tx) => tx
                .validate_and_apply(multiledger_history)
                .map_err(Error::StartHTLError),
            MultiLedgerTransaction::EndHTL(tx) => transactions
                .get(&tx.start_htl_id())
                .and_then(|start_htl| start_htl.unwrap_start_htl())
                .ok_or(Error::InvalidStartHTLError)
                .and_then(|start_htl| tx
                    .validate_and_apply(start_htl, multiledger_history)
                    .map_err(Error::EndHTLError)
                ),
        }
    }

    /// Validates and applies the transaction and it's operations against the
    /// ledgers available in `MultiLedgerHistory`, but also guarantees that all
    /// ledgers required by the transaction are available.
    pub fn validate_and_apply_new<H: MultiLedgerHistory>(
        &self,
        transactions: &TransactionMap,
        multiledger_history: H,
    ) -> Result<H, Error> {
        // ensure no ops require ledgers not in multiledger_history
        self.required_ledger_ids(transactions)
            .ok_or(Error::InvalidEndHTLError)
            .and_then(|ref required_ledger_ids| {
                if multiledger_history.has_all_histories(required_ledger_ids) {
                    self.validate_and_apply(transactions, multiledger_history)
                } else {
                    Err(Error::InvalidEndHTLError)
                }
            })
    }

    /// Checks that the transaction's ledger sequence number bumps are valid
    /// against multi ledger history.
    ///
    /// NOTE: only checks against `MultiLedgerState`s present in
    /// `multiledger_history`
    fn is_valid_seq_no<H: MultiLedgerHistory>(
        &self,
        multiledger_history: &H,
    ) -> Result<(), Error> {
        self.seq_nos()
            .iter()
            .filter(|(id, _)| multiledger_history.has_history(id))
            .map(|(ledger_id, tx_seq_no)| multiledger_history
                .get(ledger_id)
                .and_then(|op_history| op_history.current_seq_no())
                .map(|ledger_seq_no| (ledger_seq_no, tx_seq_no))
            )
            .fold(Ok(()), |seq_nos_are_valid, seq_nos| seq_nos_are_valid
                .and_then(|_| seq_nos.ok_or(Error::InvalidSequenceNumberError))
                .and_then(|(ledger_seq_no, &tx_seq_no)| {
                    let new_seq_no = ledger_seq_no + 1;
                    if tx_seq_no.lt(&new_seq_no) {
                        Err(Error::RepeatedSequenceNumberError)
                    } else if tx_seq_no.gt(&new_seq_no) {
                        Err(Error::SkippedSequenceNumberError)
                    } else {
                        Ok(())
                    }
                })
            )
    }

    fn required_ledger_ids(
        &self,
        transactions: &TransactionMap,
    ) -> Option<LedgerIds> {
        match self {
            MultiLedgerTransaction::Basic(tx) => tx.required_ledger_ids(),
            MultiLedgerTransaction::StartHTL(tx) => tx.required_ledger_ids(),
            MultiLedgerTransaction::EndHTL(tx) => transactions
                .get(&tx.start_htl_id())
                .and_then(|start_htl_mlt| start_htl_mlt.unwrap_start_htl())
                .and_then(|start_htl| tx.required_ledger_ids(start_htl)),
        }
    }

    /// Retrieve the nested `StartHTLTransaction` from the container
    /// `MultiLedgerTransaction`, if possible
    fn unwrap_start_htl(&self) -> Option<&StartHTLTransaction> {
        match self {
            MultiLedgerTransaction::StartHTL(tx) => Some(tx),
            _ => None,
        }
    }
}

impl<'a> Transaction<'a, Error> for MultiLedgerTransaction<'a> {
    fn id(&self) -> TransactionId<'a> {
        match self {
            MultiLedgerTransaction::Basic(tx) => tx.id(),
            MultiLedgerTransaction::StartHTL(tx) => tx.id(),
            MultiLedgerTransaction::EndHTL(tx) => tx.id(),
        }
    }

    fn seq_nos(&self) -> &SequenceNumbers<'a> {
        match self {
            MultiLedgerTransaction::Basic(tx) => tx.seq_nos(),
            MultiLedgerTransaction::StartHTL(tx) => tx.seq_nos(),
            MultiLedgerTransaction::EndHTL(tx) => tx.seq_nos(),
        }
    }

    fn operations(&self) -> Option<&Operations<'a>> {
        match self {
            MultiLedgerTransaction::Basic(tx) => tx.operations(),
            MultiLedgerTransaction::StartHTL(tx) => tx.operations(),
            MultiLedgerTransaction::EndHTL(tx) => tx.operations(),
        }
    }

    fn operation_ledger_ids(&self) -> LedgerIds<'a> {
        match self {
            MultiLedgerTransaction::Basic(tx) =>
                tx.operation_ledger_ids(),
            MultiLedgerTransaction::StartHTL(tx) =>
                tx.operation_ledger_ids(),
            MultiLedgerTransaction::EndHTL(tx) =>
                tx.operation_ledger_ids(),
        }
    }

    fn required_ledger_ids(&self) -> Option<LedgerIds<'a>> {
        panic!("Use `required_ledger_ids(&self, txs: &TransactionMap)");
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        NonExistantStartHTLError {
            description("StartHTL transaction (referred to by an EndHTL transaction) does not exist")
        }
        InvalidStartHTLError {
            description("StartHTL transaction is invalid")
        }
        InvalidEndHTLError {
            description("EndHTL transaction is invalid")
        }
        InvalidSequenceNumberError {
            description("Transaction conflicts with current ledger sequence number")
        }
        RepeatedSequenceNumberError {
            description("Transaction requires reusing a ledger sequence number")
        }
        SkippedSequenceNumberError {
            description("Transaction requires skipping a ledger sequence number; some transactions may not have been applied")
        }
        BasicError(err: BasicError) {
            description(err.description())
        }
        StartHTLError(err: StartHTLError) {
            description(err.description())
        }
        EndHTLError(err: EndHTLError) {
            description(err.description())
        }
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn it_works() {
        assert!(true);
    }
}
