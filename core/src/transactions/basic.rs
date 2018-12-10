use std::rc::Rc;
use quick_error::quick_error;
use serde_derive::{Serialize, Deserialize};
use crate::types::*;
use super::base::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct BasicTransaction<'a> {
    #[serde(borrow)]
    id: TransactionId<'a>,

    sender: Rc<Hash>,

    #[serde(borrow)]
    seq_nos: SequenceNumbers<'a>,

    metadata: Option<TransactionMetadata>,

    #[serde(borrow)]
    operations: Operations<'a>,
}

impl<'a> BasicTransaction<'a> {
    pub fn validate_and_apply<H: MultiLedgerHistory>(
        &self,
        _multiledger_history: H,
    ) -> Result<H, Error> {
        // ensure no ops require ledgers not listed in seq_nos
        // ensure sender is owner on all used ledgers
        // ensure this txn's seq_no is one greater than seq_no in ledger
        // ensure that each operation is valid and applied

        Err(Error::InvalidTransaction)
    }
}

impl<'a> Transaction<'a, Error> for BasicTransaction<'a> {
    fn id(&self) -> TransactionId<'a> { &self.id }
    fn seq_nos(&self) -> &SequenceNumbers<'a> { &self.seq_nos }
    fn operations(&self) -> Option<&Operations<'a>> { Some(&self.operations) }
}

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        InvalidTransaction {
            description("Invalid transaction")
        }
    }
}
