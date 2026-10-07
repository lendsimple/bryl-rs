use crate::entry::Entry;

/// Entry hashes keep their ten rightmost digits.
pub const HASH_MODULUS: u64 = 10_000_000_000;

/// Running control totals for a batch or a file.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    /// Entry detail records plus addenda records.
    pub entry_addenda_count: u32,
    /// Sum of receiving DFI TRNs, modulo 10^10.
    pub entry_hash: u64,
    /// Sum of debit amounts, in cents.
    pub total_debit_amount: u64,
    /// Sum of credit amounts, in cents.
    pub total_credit_amount: u64,
}

impl Totals {
    /// Adds one entry and its addenda.
    pub fn add_entry(&mut self, entry: &Entry) {
        let records = u32::try_from(entry.addenda_count())
            .unwrap_or(u32::MAX)
            .saturating_add(1);
        self.entry_addenda_count = self.entry_addenda_count.saturating_add(records);
        self.entry_hash =
            (self.entry_hash + u64::from(entry.detail.receiving_dfi.trn())) % HASH_MODULUS;
        let code = entry.detail.transaction_code;
        let amount = entry.detail.amount;
        if code.is_debit() {
            self.total_debit_amount = self.total_debit_amount.saturating_add(amount);
        } else if code.is_credit() {
            self.total_credit_amount = self.total_credit_amount.saturating_add(amount);
        }
    }

    /// Adds another set of totals, such as a finished batch to its file.
    pub fn add(&mut self, other: &Self) {
        self.entry_addenda_count = self
            .entry_addenda_count
            .saturating_add(other.entry_addenda_count);
        self.entry_hash = (self.entry_hash + other.entry_hash) % HASH_MODULUS;
        self.total_debit_amount = self
            .total_debit_amount
            .saturating_add(other.total_debit_amount);
        self.total_credit_amount = self
            .total_credit_amount
            .saturating_add(other.total_credit_amount);
    }
}
