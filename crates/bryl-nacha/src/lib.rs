//! [NACHA](https://www.nacha.org/) ACH files: fixed-width, 94-character
//! records laid out as
//!
//! ```text
//! FileHeader            1
//!   BatchHeader         5
//!     EntryDetail       6
//!       Addendum        7  (zero or more)
//!     ...
//!   BatchControl        8
//!   ...
//! FileControl           9
//! 9999...9999              (filler to a multiple of 10 lines)
//! ```
//!
//! [`Writer`] writes files: its nested guards make it impossible to write an
//! entry outside a batch or a batch outside a file, it computes every hash,
//! total, count and the block count, and it rejects entries that break NACHA
//! rules before writing them. [`Reader`] reads files record by record,
//! [`File::read`] reads them whole, and [`File::validate`] checks them.
//!
//! # Writing a file
//!
//! A file holds batches; a batch holds entries for one company, one
//! [standard entry class](StandardEntryClass) and one effective date. Each
//! guard must be finished: [`BatchWriter::finish`] writes the batch control
//! record and [`FileWriter::finish`] writes the file control record and
//! filler lines.
//!
//! ```
//! use chrono::NaiveDate;
//! use nacha::{
//!     AccountKind, BatchParams, EntryParams, FileParams, ServiceClassCode,
//!     StandardEntryClass, TransactionCode, Writer,
//! };
//!
//! let created_at = NaiveDate::from_ymd_opt(2024, 1, 31)
//!     .unwrap()
//!     .and_hms_opt(14, 30, 0)
//!     .unwrap();
//! let mut writer = Writer::new(Vec::new());
//! let mut file = writer.begin_file(
//!     FileParams::builder()
//!         .immediate_destination("091000019".parse()?) // your bank's routing number
//!         .immediate_destination_name("DEST BANK")
//!         .immediate_origin("1234567890")
//!         .immediate_origin_name("ACME LENDING")
//!         .created_at(created_at)
//!         .build(),
//! )?;
//!
//! // Collect loan payments: a debits-only consumer (PPD) batch.
//! let mut batch = file.begin_batch(
//!     BatchParams::builder()
//!         .service_class_code(ServiceClassCode::DebitsOnly)
//!         .company_name("ACME LENDING")
//!         .company_id("1234567890")
//!         .standard_entry_class(StandardEntryClass::Ppd)
//!         .company_entry_description("LOAN PMT")
//!         .originating_dfi_id(9_100_001) // first 8 digits of your bank's routing number
//!         .effective_entry_date(NaiveDate::from_ymd_opt(2024, 2, 1).unwrap())
//!         .build(),
//! )?;
//! let entry = batch.entry(
//!     EntryParams::builder()
//!         .transaction_code(
//!             TransactionCode::for_entry(-12_500, AccountKind::Checking, false, false)
//!                 .ok_or("no transaction code")?,
//!         )
//!         .receiving_dfi("021000021".parse()?)
//!         .account_number("000123456789")
//!         .amount(12_500) // cents
//!         .individual_id("LOAN0001")
//!         .individual_name("JANE SMITH")
//!         .addenda(vec!["PAYMENT FOR LOAN 0001".into()])
//!         .build(),
//! )?;
//! assert_eq!(entry.detail.transaction_code, TransactionCode::CheckingDebit);
//! assert_eq!(entry.detail.trace_number, 91_000_010_000_001); // ODFI + sequence
//! batch.finish()?;
//! let control = file.finish()?;
//!
//! assert_eq!(control.total_debit_amount, 12_500);
//! let ach = String::from_utf8(writer.into_inner())?;
//! assert_eq!(ach.lines().count(), 10); // 6 records + 4 filler lines
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Text is uppercased when written. Amounts are in cents. Trace numbers,
//! batch numbers, addenda sequence numbers and the addenda record indicator
//! are assigned automatically; pass [`EntryParams::trace_number`] to use your
//! own.
//!
//! # Choosing transaction codes
//!
//! A [`TransactionCode`] combines the receiving account kind (checking,
//! savings, general ledger or loan; see [`AccountKind`]) with the direction
//! and purpose of the entry. [`TransactionCode::for_entry`] picks one from a
//! signed amount: negative amounts are debits, positive amounts credits, and
//! a zero amount (or `is_prenote`) a prenote. It returns `None` for a loan
//! debit prenote, which does not exist.
//!
//! ```
//! use nacha::{AccountKind, TransactionCode};
//!
//! let code = |amount, account| TransactionCode::for_entry(amount, account, false, false);
//! assert_eq!(code(5_000, AccountKind::Savings), Some(TransactionCode::SavingsCredit));
//! assert_eq!(code(-5_000, AccountKind::Checking), Some(TransactionCode::CheckingDebit));
//! assert_eq!(code(0, AccountKind::Checking), Some(TransactionCode::CheckingPrenoteCredit));
//! // Disbursing a loan by crediting the borrower's loan account at another bank.
//! assert_eq!(code(250_000, AccountKind::Loan), Some(TransactionCode::LoanCredit));
//! assert_eq!(TransactionCode::for_entry(-5_000, AccountKind::Loan, false, true), None);
//! assert!(TransactionCode::CheckingPrenoteDebit.is_prenote());
//! ```
//!
//! Prenotes (zero-dollar test entries that verify an account before the
//! first real entry) must have a zero amount. Some banks do not accept them.
//! A loan debit ([`TransactionCode::LoanDebit`], 55) is allowed only in a
//! reversal batch. Zero-dollar remittance codes (24, 29, 34, 39, 44, 49, 54)
//! carry remittance data in their addenda; `for_entry` never picks them.
//!
//! The code tables ([`TransactionCode`], [`ReturnReasonCode`],
//! [`StandardEntryClass`], [`ServiceClassCode`]), [`AccountKind`] and
//! [`NachaRecord`] are `#[non_exhaustive]`: minor releases may add codes, so
//! a `match` on them needs a wildcard arm.
//!
//! ```
//! use nacha::TransactionCode;
//!
//! fn describe(code: TransactionCode) -> &'static str {
//!     match code {
//!         TransactionCode::CheckingCredit | TransactionCode::SavingsCredit => "credit",
//!         TransactionCode::CheckingDebit | TransactionCode::SavingsDebit => "debit",
//!         _ => "other",
//!     }
//! }
//! assert_eq!(describe(TransactionCode::SavingsDebit), "debit");
//! ```
//!
//! Without the wildcard, a match naming every current code does not compile:
//!
//! ```compile_fail,E0004
//! use nacha::TransactionCode;
//!
//! fn describe(code: TransactionCode) -> &'static str {
//!     match code {
//!         TransactionCode::CheckingReturnedCredit
//!         | TransactionCode::CheckingCredit
//!         | TransactionCode::CheckingPrenoteCredit
//!         | TransactionCode::SavingsReturnedCredit
//!         | TransactionCode::SavingsCredit
//!         | TransactionCode::SavingsPrenoteCredit => "credit",
//!         TransactionCode::CheckingReturnedDebit
//!         | TransactionCode::CheckingDebit
//!         | TransactionCode::CheckingPrenoteDebit
//!         | TransactionCode::SavingsReturnedDebit
//!         | TransactionCode::SavingsDebit
//!         | TransactionCode::SavingsPrenoteDebit => "debit",
//!     }
//! }
//! ```
//!
//! # Rules the writer enforces
//!
//! [`BatchWriter::entry`] checks each entry and [`FileWriter::begin_batch`]
//! checks each batch header before writing anything, so a rejected entry or
//! batch leaves the output unchanged:
//!
//! - **Service class.** A [`ServiceClassCode::CreditsOnly`] (220) batch holds
//!   only credits and a [`ServiceClassCode::DebitsOnly`] (225) batch only
//!   debits; [`ServiceClassCode::MixedDebitsAndCredits`] (200) holds both.
//! - **Entry class direction.** TEL and the check conversion classes (ARC,
//!   BOC, POP, RCK, TRC) are debit-only and CIE is credit-only, except in a
//!   reversal batch (company entry description [`REVERSAL`]).
//! - **Addenda.** At most one addendum for PPD, CCD and WEB entries, none
//!   for TEL, up to 9,999 for CTX; see
//!   [`StandardEntryClass::max_addenda`] and
//!   [`StandardEntryClass::min_addenda`].
//! - **Entry descriptions.** ENR batches must be described `AUTOENROLL` and
//!   RCK batches `REDEPCHECK`.
//! - **Prenotes** have a zero amount.
//! - **Zero-dollar remittance** entries (24, 29, 34, 39, 44, 49, 54) have a
//!   zero amount and at least one addendum, and appear only in CCD and CTX
//!   batches, or ACK and ATX batches for 24 and 34; see
//!   [`StandardEntryClass::allows_zero_dollar_remittance`].
//! - **Loan debits** ([`TransactionCode::LoanDebit`], 55) appear only in a
//!   reversal batch.
//! - **Returns.** A return entry (transaction code x1 or x6) carries exactly
//!   one return addendum (standard, dishonored or contested) whose trace
//!   number is its own, and no other addenda; only R14 and R15 returns may
//!   give a date of death, and the two-digit reason codes of dishonored and
//!   contested returns must name known codes.
//! - **Notifications of change.** Every entry in a COR batch has a zero
//!   amount, a return transaction code and exactly one [`NocAddendum`] or
//!   [`RefusedNocAddendum`] whose trace number is its own, and no other
//!   addenda; type-98 addenda appear only in COR batches.
//! - **Routing numbers** must pass the ABA checksum, which [`RoutingNumber`]
//!   checks when it is created.
//!
//! ```
//! use nacha::{IssueKind, ServiceClassCode, TransactionCode};
//! # use nacha::{BatchParams, EntryParams, FileParams, StandardEntryClass, Writer};
//! # let created_at = chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap().and_hms_opt(9, 0, 0).unwrap();
//! # let mut writer = Writer::new(Vec::new());
//! # let mut file = writer.begin_file(FileParams::builder().immediate_destination("091000019".parse()?)
//! #     .immediate_destination_name("DEST").immediate_origin("1234567890").immediate_origin_name("ACME")
//! #     .created_at(created_at).build())?;
//! # let mut batch = file.begin_batch(BatchParams::builder().service_class_code(ServiceClassCode::CreditsOnly)
//! #     .company_name("ACME").company_id("1234567890").standard_entry_class(StandardEntryClass::Ppd)
//! #     .company_entry_description("PAYOUT").originating_dfi_id(9_100_001).build())?;
//! # let params = EntryParams::builder().transaction_code(TransactionCode::CheckingDebit)
//! #     .receiving_dfi("021000021".parse()?).account_number("123").amount(100)
//! #     .individual_id("ID").individual_name("NAME").build();
//! // `batch` is a credits-only batch; `params` describes a debit.
//! let err = batch.entry(params).unwrap_err();
//! assert!(matches!(
//!     err,
//!     nacha::Error::InvalidEntry(IssueKind::ServiceClass { .. })
//! ));
//! assert_eq!(
//!     err.to_string(),
//!     "invalid entry: transaction code 27 is not allowed in a 220 batch"
//! );
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Dropping a guard without calling `finish` leaves the output without its
//! control record, which is how to abandon a file after an error.
//!
//! # Reading and validating a file
//!
//! [`File::read`] parses a whole file and checks its structure (record order,
//! record lengths, field formats, routing number checksums). Then
//! [`File::validate`] checks everything else and returns every problem it
//! finds: control totals, entry hashes, counts, batch header and control
//! agreement, the rules above, trace and batch number order, and block
//! padding.
//!
//! ```
//! # fn sample() -> Result<String, Box<dyn std::error::Error>> {
//! #     use nacha::*;
//! #     let created_at = chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap().and_hms_opt(9, 0, 0).unwrap();
//! #     let mut writer = Writer::new(Vec::new());
//! #     let mut file = writer.begin_file(FileParams::builder().immediate_destination("091000019".parse()?)
//! #         .immediate_destination_name("DEST").immediate_origin("1234567890").immediate_origin_name("ACME")
//! #         .created_at(created_at).build())?;
//! #     let mut batch = file.begin_batch(BatchParams::builder().service_class_code(ServiceClassCode::CreditsOnly)
//! #         .company_name("ACME").company_id("1234567890").standard_entry_class(StandardEntryClass::Ppd)
//! #         .company_entry_description("PAYOUT").originating_dfi_id(9_100_001).build())?;
//! #     for amount in [1_000, 2_500] {
//! #         batch.entry(EntryParams::builder().transaction_code(TransactionCode::CheckingCredit)
//! #             .receiving_dfi("021000021".parse()?).account_number("123").amount(amount)
//! #             .individual_id("ID").individual_name("JANE SMITH").build())?;
//! #     }
//! #     batch.finish()?;
//! #     file.finish()?;
//! #     Ok(String::from_utf8(writer.into_inner())?)
//! # }
//! # let ach = sample()?;
//! let file = nacha::File::read(ach.as_bytes())?;
//! for batch in &file.batches {
//!     for entry in &batch.entries {
//!         println!("{} {}", entry.detail.individual_name, entry.detail.amount);
//!     }
//! }
//! assert_eq!(file.control.total_credit_amount, 3_500);
//!
//! let issues = file.validate();
//! for issue in &issues {
//!     eprintln!("{issue}"); // e.g. "batch 1: entry_hash is 4200002, computed 4200001"
//! }
//! assert!(issues.is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! For large files, or to stop at the first problem, read record by record
//! with [`Reader`]; see its documentation. Read errors name the input (set
//! with [`Reader::with_name`]) and the line, e.g.
//! `ach.txt @ line 4 - unexpected record type FileControl, expected BatchControl`.
//!
//! # Returns
//!
//! A return entry has a "returned" transaction code (x1 or x6, such as 21 or
//! 26) and a type-99 [`ReturnAddendum`] instead of type-05 addenda, held in
//! [`Entry::special_addendum`] and returned by [`Entry::return_addendum`]. It gives the [`ReturnReasonCode`] and the trace number
//! of the entry being returned:
//!
//! ```
//! # fn sample() -> Result<String, Box<dyn std::error::Error>> {
//! #     use nacha::*;
//! #     let created_at = chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap().and_hms_opt(9, 0, 0).unwrap();
//! #     let mut writer = Writer::new(Vec::new());
//! #     let mut file = writer.begin_file(FileParams::builder().immediate_destination("091000019".parse()?)
//! #         .immediate_destination_name("ACME BANK").immediate_origin("0210000210").immediate_origin_name("RDFI")
//! #         .created_at(created_at).build())?;
//! #     let mut batch = file.begin_batch(BatchParams::builder().service_class_code(ServiceClassCode::DebitsOnly)
//! #         .company_name("ACME").company_id("1234567890").standard_entry_class(StandardEntryClass::Ppd)
//! #         .company_entry_description("LOAN PMT").originating_dfi_id(2_100_002).build())?;
//! #     batch.entry(EntryParams::builder().transaction_code(TransactionCode::CheckingReturnedDebit)
//! #         .receiving_dfi("091000019".parse()?).account_number("123").amount(2_500)
//! #         .individual_id("LOAN0001").individual_name("JANE SMITH")
//! #         .special_addendum(ReturnParams::builder().return_reason_code(ReturnReasonCode::InsufficientFunds)
//! #             .original_entry_trace_number(91_000_010_000_001).original_receiving_dfi_id(2_100_002).build())
//! #         .build())?;
//! #     batch.finish()?;
//! #     file.finish()?;
//! #     Ok(String::from_utf8(writer.into_inner())?)
//! # }
//! # let returns = sample()?;
//! use nacha::ReturnReasonCode;
//!
//! let file = nacha::File::read(returns.as_bytes())?;
//! for entry in file.batches.iter().flat_map(|batch| &batch.entries) {
//!     if let Some(returned) = entry.return_addendum() {
//!         println!(
//!             "{:015} returned: {}",
//!             returned.original_entry_trace_number, returned.return_reason_code
//!         );
//!         assert_eq!(returned.return_reason_code, ReturnReasonCode::InsufficientFunds);
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! To write a return, give [`EntryParams::special_addendum`] a
//! [`ReturnParams`]; the writer fills in the addendum's trace number.
//!
//! An ODFI that receives a return it considers untimely or wrong dishonors
//! it with a [`DishonoredReturnAddendum`] ([`DishonoredReturnCode`] R61, R62,
//! R67–R70), and the RDFI can contest the dishonor with a
//! [`ContestedReturnAddendum`] ([`ContestedReturnCode`] R71–R77). Both are
//! type-99 addenda with their own layouts; the reader picks the layout from
//! the code, and [`Entry::dishonored_return_addendum`] and
//! [`Entry::contested_return_addendum`] return them. Their two-digit reason
//! codes convert with `return_reason()`. Write them with
//! [`DishonoredReturnParams`] and [`ContestedReturnParams`].
//!
//! # Notifications of change
//!
//! A receiving bank that posts an entry despite wrong details (an outdated
//! account number, checking instead of savings) sends a notification of
//! change (NOC): a zero-amount entry in a COR batch, with a return
//! transaction code and a type-98 [`NocAddendum`] in place of type-05
//! addenda. Its [`ChangeCode`] says what to correct and its corrected data
//! holds the new value; update your records before the next entry. An
//! originator can refuse a notification with a [`RefusedNocAddendum`] and a
//! [`RefusedChangeCode`].
//!
//! ```
//! # fn sample() -> Result<String, Box<dyn std::error::Error>> {
//! #     use nacha::*;
//! #     let created_at = chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap().and_hms_opt(9, 0, 0).unwrap();
//! #     let mut writer = Writer::new(Vec::new());
//! #     let mut file = writer.begin_file(FileParams::builder().immediate_destination("091000019".parse()?)
//! #         .immediate_destination_name("ACME BANK").immediate_origin("0210000210").immediate_origin_name("RDFI")
//! #         .created_at(created_at).build())?;
//! #     let mut batch = file.begin_batch(BatchParams::builder().service_class_code(ServiceClassCode::MixedDebitsAndCredits)
//! #         .company_name("ACME").company_id("1234567890").standard_entry_class(StandardEntryClass::Cor)
//! #         .company_entry_description("NOC").originating_dfi_id(2_100_002).build())?;
//! #     batch.entry(EntryParams::builder().transaction_code(TransactionCode::CheckingReturnedDebit)
//! #         .receiving_dfi("091000019".parse()?).account_number("123").amount(0)
//! #         .individual_id("LOAN0001").individual_name("JANE SMITH")
//! #         .special_addendum(NocParams::builder().change_code(ChangeCode::IncorrectAccountNumber)
//! #             .original_entry_trace_number(91_000_010_000_001).original_receiving_dfi_id(9_100_001)
//! #             .corrected_data("987654321").build())
//! #         .build())?;
//! #     batch.finish()?;
//! #     file.finish()?;
//! #     Ok(String::from_utf8(writer.into_inner())?)
//! # }
//! # let notifications = sample()?;
//! use nacha::ChangeCode;
//!
//! let file = nacha::File::read(notifications.as_bytes())?;
//! assert_eq!(file.validate(), []);
//! for entry in file.batches.iter().flat_map(|batch| &batch.entries) {
//!     if let Some(noc) = entry.noc_addendum() {
//!         if noc.change_code == ChangeCode::IncorrectAccountNumber {
//!             println!(
//!                 "entry {:015}: new account number {}",
//!                 noc.original_entry_trace_number,
//!                 noc.corrected_data.trim()
//!             );
//!         }
//!         assert_eq!(noc.corrected_data, "987654321");
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! To write one, give [`EntryParams::special_addendum`] a [`NocParams`] or
//! [`RefusedNocParams`]. The corrected data is kept as written; see
//! [`ChangeCode`] for what it holds for each code.
//!
//! # Configuration
//!
//! ```
//! use nacha::{LineEnding, Writer};
//!
//! let writer = Writer::new(Vec::new())
//!     .line_ending(LineEnding::CrLf) // some banks require CRLF; LF by default
//!     .pad_blocks(true) // filler to a multiple of 10 lines (the default)
//!     .sanitize(nacha::Sanitize::UPPER); // uppercase text (the default)
//! ```
//!
//! Reading accepts both LF and CRLF.
//!
//! The `clock` feature, on by default, provides [`FileParams::now()`], which
//! reads the system clock. For targets without one, such as
//! `wasm32-unknown-unknown`, turn it off (`default-features = false`) and set
//! [`FileParams::created_at`] yourself.
//!
//! # Bank differences
//!
//! NACHA leaves some details to the bank receiving the file. Check its file
//! specification for:
//!
//! - **Line endings.** Some banks require CRLF ([`LineEnding::CrLf`]).
//! - **Prenotes.** Some banks (Chase) do not accept them.
//! - **Service class.** Some banks (Chase) want credits-only (220) or
//!   debits-only (225) batches rather than mixed (200).
//! - **Immediate origin.** Usually the originating bank's routing number
//!   preceded by a blank; some banks ask for a 10-character company ID.
//! - **Characters.** NACHA requires uppercase codes. Rules for free text
//!   range from permissive (Chase accepts lowercase and `. / ( ) & ' -`) to
//!   strict (Banc of California: uppercase only, no special characters
//!   outside addenda). The writer uppercases text and accepts printable
//!   ASCII; it never writes anything else, so files are plain ASCII.
//!
//! # Not supported
//!
//! - Type 02 addenda (MTE, POS, SHR). Entries of these classes cannot carry
//!   their required addenda, and reading a file that contains them fails.
//! - International entries (IAT), their addenda types 10–18 and their
//!   returns (R80–R85).
//!
//! # Sources
//!
//! The rules follow these public sources. Where a rule rests on a single
//! source, its documentation says so.
//!
//! - [NACHA's ACH developer guide](https://achdevguide.nacha.org/ach-file-details):
//!   record layouts; filler lines and the block count; the immediate
//!   destination and origin; trace numbers ascending within a batch and
//!   unique within the file; ascending batch numbers; the file ID modifier;
//!   addenda limits for PPD, CCD, WEB, TEL and CTX; debit-only TEL entries;
//!   uppercase codes.
//! - Bank file specifications from
//!   [Chase](https://www.chase.com/content/dam/chaseonline/en/demos/cbo/pdfs/cbo_nacha_filespecs.pdf),
//!   [Regions](https://www.regions.com/-/media/pdfs/treasury-management/NACHA_File_Layout_Guide.pdf),
//!   [First Citizens](https://www.firstcitizens.com/content/dam/firstcitizens/pdfs/commercial/commercial-advantage/nacha-file-specs.pdf)
//!   and
//!   [Banc of California](https://dam.bancofcal.com/m/5ec324636794be06/original/UG32-NACHA-File-Format-Specifications.pdf):
//!   left-justified account numbers, addenda sequence numbers from `0001`,
//!   zero-amount prenotes, required filler, line endings, character sets.
//! - [moov-io/ach](https://github.com/moov-io/ach) (`batch*.go`): addenda
//!   limits for the other entry classes, debit- and credit-only entry classes,
//!   and the `AUTOENROLL` and `REDEPCHECK` entry descriptions;
//!   (`addenda99.go`) the return addendum layout and the R90 return code.
//! - [Personify's addenda record reference](https://resource1.personifycorp.com/PersonifyOnlineHelp/7.7.0/mergedProjects/eft/Addenda_Record.htm):
//!   the return addendum layout, and the date of death used only with R14
//!   and R15.
//! - [Stripe's list of ACH return codes](https://stripe.com/resources/more/the-complete-list-of-ach-rejection-codes-why-they-happen-and-how-to-handle-them):
//!   current return code titles, such as R11 (not in accordance with the
//!   authorization) and R13 (invalid routing number).

mod codes;
mod entry;
mod error;
mod file;
mod reader;
mod records;
mod totals;
mod types;
mod validate;
mod writer;

/// Text normalization applied when writing; see [`Writer::sanitize`].
pub use bryl::Sanitize;
pub use codes::{
    AccountKind, ChangeCode, ContestedReturnCode, DishonoredReturnCode, REVERSAL,
    RefusedChangeCode, ReturnReasonCode, ServiceClassCode, StandardEntryClass, TransactionCode,
};
pub use entry::{Entry, SpecialAddendum};
pub use error::Error;
pub use file::{Batch, File};
pub use reader::{Entries, Reader};
pub use records::{
    Addendum, BatchControl, BatchHeader, ContestedReturnAddendum, DishonoredReturnAddendum,
    EntryDetail, FILLER, FileControl, FileHeader, NachaRecord, NocAddendum, RECORD_LENGTH,
    RefusedNocAddendum, ReturnAddendum,
};
pub use totals::{HASH_MODULUS, Totals};
pub use types::{FileIdModifier, FileIdModifierError, RoutingNumber, RoutingNumberError};
pub use validate::{Issue, IssueKind};
pub use writer::{
    BatchParams, BatchWriter, ContestedReturnParams, DishonoredReturnParams, EntryParams,
    FileParams, FileWriter, LineEnding, NocParams, RefusedNocParams, ReturnParams,
    SpecialAddendumParams, Writer,
};

/// Runs the code in README.md as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
