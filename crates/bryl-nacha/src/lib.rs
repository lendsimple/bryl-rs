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
//!         .transaction_code(TransactionCode::for_entry(-12_500, AccountKind::Checking, false, false))
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
//! A [`TransactionCode`] combines the account type (checking or savings) with
//! the direction and purpose of the entry. [`TransactionCode::for_entry`]
//! picks one from a signed amount: negative amounts are debits, positive
//! amounts credits, and a zero amount (or `is_prenote`) a prenote.
//!
//! ```
//! use nacha::{AccountKind, TransactionCode};
//!
//! assert_eq!(TransactionCode::for_entry(5_000, AccountKind::Savings, false, false), TransactionCode::SavingsCredit);
//! assert_eq!(TransactionCode::for_entry(-5_000, AccountKind::Checking, false, false), TransactionCode::CheckingDebit);
//! assert_eq!(TransactionCode::for_entry(0, AccountKind::Checking, false, false), TransactionCode::CheckingPrenoteCredit);
//! assert!(TransactionCode::CheckingPrenoteDebit.is_prenote());
//! ```
//!
//! Prenotes (zero-dollar test entries that verify an account before the
//! first real entry) must have a zero amount. Some banks do not accept them.
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
//!   and the `AUTOENROLL` and `REDEPCHECK` entry descriptions.

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
pub use codes::{AccountKind, REVERSAL, ServiceClassCode, StandardEntryClass, TransactionCode};
pub use entry::Entry;
pub use error::Error;
pub use file::{Batch, File};
pub use reader::{Entries, Reader};
pub use records::{
    Addendum, BatchControl, BatchHeader, EntryDetail, FILLER, FileControl, FileHeader, NachaRecord,
    RECORD_LENGTH,
};
pub use totals::{HASH_MODULUS, Totals};
pub use types::{FileIdModifier, FileIdModifierError, RoutingNumber, RoutingNumberError};
pub use validate::{Issue, IssueKind};
pub use writer::{
    BatchParams, BatchWriter, EntryParams, FileParams, FileWriter, LineEnding, Writer,
};

/// Runs the code in README.md as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
