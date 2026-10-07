//! Metro 2 records and segments (character format).
//!
//! Field positions match moov-io/metro2's Go implementation. Optional fields
//! default to blanks, zeros or `None` (zero-filled dates).

use bon::Builder;
use bryl::{Const, Record};
use chrono::{NaiveDate, NaiveDateTime};

use crate::codes::{
    AccountStatus, AccountType, AddressIndicator, AgencyIdentifier, ChangeIndicator,
    ComplianceConditionCode, ConsumerInformationIndicator, CreditorClassification, EcoaCode,
    GenerationCode, InterestTypeIndicator, PaymentRating, PortfolioType, PurchasedIndicator,
    ResidenceCode, SpecialComment, SpecializedPaymentIndicator, TermsFrequency,
};
use crate::profile::PaymentHistoryProfile;

/// Header record (426 characters).
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 426)]
#[builder(on(String, into))]
pub struct HeaderRecord {
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(4), constant = 426)]
    #[builder(skip)]
    pub record_descriptor_word: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(6), constant = "HEADER")]
    #[builder(skip)]
    pub record_identifier: Const,
    /// Reporting cycle, for furnishers that report in cycles.
    #[bryl(alpha(2))]
    #[builder(default)]
    pub cycle_identifier: String,
    /// Innovis program identifier.
    #[bryl(alpha(10))]
    #[builder(default)]
    pub innovis_program_identifier: String,
    /// Equifax program identifier.
    #[bryl(alpha(10))]
    #[builder(default)]
    pub equifax_program_identifier: String,
    /// Experian program identifier.
    #[bryl(alpha(5))]
    #[builder(default)]
    pub experian_program_identifier: String,
    /// TransUnion program identifier.
    #[bryl(alpha(10))]
    #[builder(default)]
    pub transunion_program_identifier: String,
    /// Date the account information is as of.
    #[bryl(date("MMDDYYYY"))]
    pub activity_date: NaiveDate,
    /// Date the file was created.
    #[bryl(date("MMDDYYYY"))]
    pub date_created: NaiveDate,
    /// Date the furnisher's Metro 2 program was first reported.
    #[bryl(date("MMDDYYYY"))]
    pub program_date: Option<NaiveDate>,
    /// Date the furnisher's Metro 2 program was last revised.
    #[bryl(date("MMDDYYYY"))]
    pub program_revision_date: Option<NaiveDate>,
    /// Furnisher name.
    #[bryl(alpha(40))]
    pub reporter_name: String,
    /// Furnisher address.
    #[bryl(alpha(96))]
    pub reporter_address: String,
    /// Furnisher telephone number.
    #[bryl(numeric(10))]
    pub reporter_telephone_number: u64,
    /// Software vendor, if a vendor product created the file.
    #[bryl(alpha(40))]
    #[builder(default)]
    pub software_vendor_name: String,
    /// Software version.
    #[bryl(alpha(5))]
    #[builder(default)]
    pub software_version_number: String,
    /// PRBC program identifier.
    #[bryl(alpha(10))]
    #[builder(default)]
    pub prbc_program_identifier: String,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(146), reserved)]
    #[builder(skip)]
    pub reserved: Const,
}

/// Base segment (426 characters): one account.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 426)]
#[builder(on(String, into))]
pub struct BaseSegment {
    /// Length of the whole data record (base plus appended segments).
    ///
    /// Read from the file when decoding. When writing a
    /// [`DataRecord`](crate::DataRecord) it is computed from the segments
    /// and this value is ignored; it defaults to 426.
    #[bryl(numeric(4))]
    #[builder(default = 426)]
    pub record_descriptor_word: u16,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(1), constant = 1)]
    #[builder(skip)]
    pub processing_indicator: Const,
    /// When the account was last updated.
    #[bryl(datetime("MMDDYYYYhhmmss"))]
    pub time_stamp: Option<NaiveDateTime>,
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(1), reserved)]
    #[builder(skip)]
    pub reserved_1: Const,
    /// Furnisher identification number assigned by the bureaus.
    #[bryl(alpha(20))]
    pub identification_number: String,
    /// Reporting cycle.
    #[bryl(alpha(2))]
    #[builder(default)]
    pub cycle_identifier: String,
    /// Consumer account number.
    #[bryl(alpha(30))]
    pub consumer_account_number: String,
    /// Portfolio type.
    #[bryl(alpha(1))]
    pub portfolio_type: PortfolioType,
    /// Account type.
    #[bryl(alpha(2))]
    pub account_type: AccountType,
    /// Date the account was opened.
    #[bryl(date("MMDDYYYY"))]
    pub date_opened: NaiveDate,
    /// Credit limit, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub credit_limit: u32,
    /// Highest credit or original loan amount, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub highest_credit: u32,
    /// Terms duration, e.g. `036` months or `LOC`.
    #[bryl(alpha(3))]
    #[builder(default)]
    pub terms_duration: String,
    /// Terms frequency.
    #[bryl(alpha(1))]
    pub terms_frequency: Option<TermsFrequency>,
    /// Scheduled monthly payment, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub scheduled_monthly_payment: u32,
    /// Actual payment amount, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub actual_payment_amount: u32,
    /// Account status.
    #[bryl(alpha(2))]
    pub account_status: AccountStatus,
    /// Payment rating.
    #[bryl(alpha(1))]
    pub payment_rating: Option<PaymentRating>,
    /// 24-month payment history, most recent first.
    #[bryl(alpha(24))]
    #[builder(default)]
    pub payment_history_profile: PaymentHistoryProfile,
    /// Special comment.
    #[bryl(alpha(2))]
    pub special_comment: Option<SpecialComment>,
    /// Compliance condition code.
    #[bryl(alpha(2))]
    pub compliance_condition_code: Option<ComplianceConditionCode>,
    /// Current balance, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub current_balance: u32,
    /// Amount past due, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub amount_past_due: u32,
    /// Original charge-off amount, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub original_charge_off_amount: u32,
    /// Date the account information is as of.
    #[bryl(date("MMDDYYYY"))]
    pub date_of_account_information: NaiveDate,
    /// Date of first delinquency.
    #[bryl(date("MMDDYYYY"))]
    pub date_of_first_delinquency: Option<NaiveDate>,
    /// Date the account was closed.
    #[bryl(date("MMDDYYYY"))]
    pub date_closed: Option<NaiveDate>,
    /// Date of the last payment.
    #[bryl(date("MMDDYYYY"))]
    pub date_of_last_payment: Option<NaiveDate>,
    /// Interest type indicator.
    #[bryl(alpha(1))]
    pub interest_type_indicator: Option<InterestTypeIndicator>,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(17), reserved)]
    #[builder(skip)]
    pub reserved_2: Const,
    /// Consumer surname.
    #[bryl(alpha(25))]
    pub surname: String,
    /// Consumer first name.
    #[bryl(alpha(20))]
    pub first_name: String,
    /// Consumer middle name.
    #[bryl(alpha(20))]
    #[builder(default)]
    pub middle_name: String,
    /// Generation code.
    #[bryl(alpha(1))]
    pub generation_code: Option<GenerationCode>,
    /// Social Security number; 0 when unknown.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub social_security_number: u32,
    /// Date of birth.
    #[bryl(date("MMDDYYYY"))]
    pub date_of_birth: Option<NaiveDate>,
    /// Telephone number; 0 when unknown.
    #[bryl(numeric(10))]
    #[builder(default)]
    pub telephone_number: u64,
    /// ECOA code.
    #[bryl(alpha(1))]
    pub ecoa_code: EcoaCode,
    /// Consumer information indicator.
    #[bryl(alpha(2))]
    pub consumer_information_indicator: Option<ConsumerInformationIndicator>,
    /// Country code.
    #[bryl(alpha(2))]
    #[builder(default)]
    pub country_code: String,
    /// First address line.
    #[bryl(alpha(32))]
    pub first_line_of_address: String,
    /// Second address line.
    #[bryl(alpha(32))]
    #[builder(default)]
    pub second_line_of_address: String,
    /// City.
    #[bryl(alpha(20))]
    pub city: String,
    /// State abbreviation.
    #[bryl(alpha(2))]
    pub state: String,
    /// ZIP code.
    #[bryl(alpha(9))]
    pub zip_code: String,
    /// Address indicator.
    #[bryl(alpha(1))]
    pub address_indicator: Option<AddressIndicator>,
    /// Residence code.
    #[bryl(alpha(1))]
    pub residence_code: Option<ResidenceCode>,
}

/// J1 segment (100 characters): associated consumer at the same address.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 100)]
#[builder(on(String, into))]
pub struct J1Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "J1")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), reserved)]
    #[builder(skip)]
    pub reserved_1: Const,
    /// Surname.
    #[bryl(alpha(25))]
    pub surname: String,
    /// First name.
    #[bryl(alpha(20))]
    pub first_name: String,
    /// Middle name.
    #[bryl(alpha(20))]
    #[builder(default)]
    pub middle_name: String,
    /// Generation code.
    #[bryl(alpha(1))]
    pub generation_code: Option<GenerationCode>,
    /// Social Security number; 0 when unknown.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub social_security_number: u32,
    /// Date of birth.
    #[bryl(date("MMDDYYYY"))]
    pub date_of_birth: Option<NaiveDate>,
    /// Telephone number; 0 when unknown.
    #[bryl(numeric(10))]
    #[builder(default)]
    pub telephone_number: u64,
    /// ECOA code.
    #[bryl(alpha(1))]
    pub ecoa_code: EcoaCode,
    /// Consumer information indicator.
    #[bryl(alpha(2))]
    pub consumer_information_indicator: Option<ConsumerInformationIndicator>,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), reserved)]
    #[builder(skip)]
    pub reserved_2: Const,
}

/// J2 segment (200 characters): associated consumer at a different address.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 200)]
#[builder(on(String, into))]
pub struct J2Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "J2")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), reserved)]
    #[builder(skip)]
    pub reserved_1: Const,
    /// Surname.
    #[bryl(alpha(25))]
    pub surname: String,
    /// First name.
    #[bryl(alpha(20))]
    pub first_name: String,
    /// Middle name.
    #[bryl(alpha(20))]
    #[builder(default)]
    pub middle_name: String,
    /// Generation code.
    #[bryl(alpha(1))]
    pub generation_code: Option<GenerationCode>,
    /// Social Security number; 0 when unknown.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub social_security_number: u32,
    /// Date of birth.
    #[bryl(date("MMDDYYYY"))]
    pub date_of_birth: Option<NaiveDate>,
    /// Telephone number; 0 when unknown.
    #[bryl(numeric(10))]
    #[builder(default)]
    pub telephone_number: u64,
    /// ECOA code.
    #[bryl(alpha(1))]
    pub ecoa_code: EcoaCode,
    /// Consumer information indicator.
    #[bryl(alpha(2))]
    pub consumer_information_indicator: Option<ConsumerInformationIndicator>,
    /// Country code.
    #[bryl(alpha(2))]
    #[builder(default)]
    pub country_code: String,
    /// First address line.
    #[bryl(alpha(32))]
    pub first_line_of_address: String,
    /// Second address line.
    #[bryl(alpha(32))]
    #[builder(default)]
    pub second_line_of_address: String,
    /// City.
    #[bryl(alpha(20))]
    pub city: String,
    /// State abbreviation.
    #[bryl(alpha(2))]
    pub state: String,
    /// ZIP code.
    #[bryl(alpha(9))]
    pub zip_code: String,
    /// Address indicator.
    #[bryl(alpha(1))]
    pub address_indicator: Option<AddressIndicator>,
    /// Residence code.
    #[bryl(alpha(1))]
    pub residence_code: Option<ResidenceCode>,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), reserved)]
    #[builder(skip)]
    pub reserved_2: Const,
}

/// K1 segment (34 characters): original creditor.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 34)]
#[builder(on(String, into))]
pub struct K1Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "K1")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// Original creditor name.
    #[bryl(alpha(30))]
    pub original_creditor_name: String,
    /// Creditor classification.
    #[bryl(numeric(2))]
    pub creditor_classification: CreditorClassification,
}

/// K2 segment (34 characters): purchased from or sold to.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 34)]
#[builder(on(String, into))]
pub struct K2Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "K2")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// Purchased from, sold to, or remove.
    #[bryl(numeric(1))]
    pub purchased_indicator: PurchasedIndicator,
    /// Name of the purchaser or seller.
    #[bryl(alpha(30))]
    pub purchased_from_sold_to_name: String,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), reserved)]
    #[builder(skip)]
    pub reserved: Const,
}

/// K3 segment (40 characters): mortgage information.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 40)]
#[builder(on(String, into))]
pub struct K3Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "K3")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// Agency (Fannie Mae, Freddie Mac) holding the loan.
    #[bryl(numeric(2))]
    pub agency_identifier: AgencyIdentifier,
    /// Agency account number.
    #[bryl(alpha(18))]
    pub account_number: String,
    /// Mortgage identification number (MIN).
    #[bryl(alpha(18))]
    pub mortgage_identification_number: String,
}

/// K4 segment (30 characters): specialized payment information.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 30)]
#[builder(on(String, into))]
pub struct K4Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "K4")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// Balloon or deferred payment.
    #[bryl(numeric(2))]
    pub specialized_payment_indicator: SpecializedPaymentIndicator,
    /// Start of the deferral.
    #[bryl(date("MMDDYYYY"))]
    pub deferred_payment_start_date: Option<NaiveDate>,
    /// Balloon payment due date.
    #[bryl(date("MMDDYYYY"))]
    pub balloon_payment_due_date: Option<NaiveDate>,
    /// Balloon payment amount, in whole dollars.
    #[bryl(numeric(9))]
    #[builder(default)]
    pub balloon_payment_amount: u32,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), reserved)]
    #[builder(skip)]
    pub reserved: Const,
}

/// L1 segment (54 characters): account number or identification change.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 54)]
#[builder(on(String, into))]
pub struct L1Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "L1")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// What changed.
    #[bryl(numeric(1))]
    pub change_indicator: ChangeIndicator,
    /// New consumer account number.
    #[bryl(alpha(30))]
    pub new_consumer_account_number: String,
    /// New identification number.
    #[bryl(alpha(20))]
    #[builder(default)]
    pub new_identification_number: String,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), reserved)]
    #[builder(skip)]
    pub reserved: Const,
}

/// N1 segment (146 characters): employment information.
#[derive(Record, Builder, Debug, Clone, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 146)]
#[builder(on(String, into))]
pub struct N1Segment {
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(2), constant = "N1")]
    #[builder(skip)]
    pub segment_identifier: Const,
    /// Employer name.
    #[bryl(alpha(30))]
    pub employer_name: String,
    /// First employer address line.
    #[bryl(alpha(32))]
    #[builder(default)]
    pub first_line_of_employer_address: String,
    /// Second employer address line.
    #[bryl(alpha(32))]
    #[builder(default)]
    pub second_line_of_employer_address: String,
    /// Employer city.
    #[bryl(alpha(20))]
    #[builder(default)]
    pub employer_city: String,
    /// Employer state.
    #[bryl(alpha(2))]
    #[builder(default)]
    pub employer_state: String,
    /// Employer ZIP code.
    #[bryl(alpha(9))]
    #[builder(default)]
    pub zip_code: String,
    /// Occupation.
    #[bryl(alpha(18))]
    #[builder(default)]
    pub occupation: String,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(1), reserved)]
    #[builder(skip)]
    pub reserved: Const,
}

/// Trailer record (426 characters): file totals. All counters default to 0.
#[derive(Record, Debug, Clone, Default, PartialEq, Eq)]
#[bryl(sanitize(upper), length = 426)]
#[allow(missing_docs)] // counter names are self-describing
pub struct TrailerRecord {
    /// Fixed by the format; stores nothing.
    #[bryl(numeric(4), constant = 426)]
    pub record_descriptor_word: Const,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(7), constant = "TRAILER")]
    pub record_identifier: Const,
    #[bryl(numeric(9))]
    pub total_base_records: u32,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(9), reserved)]
    pub reserved_1: Const,
    #[bryl(numeric(9))]
    pub total_status_code_df: u32,
    #[bryl(numeric(9))]
    pub total_j1_segments: u32,
    #[bryl(numeric(9))]
    pub total_j2_segments: u32,
    /// Number of blocks in a variable-blocked file; 0 for files that are not
    /// blocked.
    #[bryl(numeric(9))]
    pub block_count: u32,
    #[bryl(numeric(9))]
    pub total_status_code_da: u32,
    #[bryl(numeric(9))]
    pub total_status_code_05: u32,
    #[bryl(numeric(9))]
    pub total_status_code_11: u32,
    #[bryl(numeric(9))]
    pub total_status_code_13: u32,
    #[bryl(numeric(9))]
    pub total_status_code_61: u32,
    #[bryl(numeric(9))]
    pub total_status_code_62: u32,
    #[bryl(numeric(9))]
    pub total_status_code_63: u32,
    #[bryl(numeric(9))]
    pub total_status_code_64: u32,
    #[bryl(numeric(9))]
    pub total_status_code_65: u32,
    #[bryl(numeric(9))]
    pub total_status_code_71: u32,
    #[bryl(numeric(9))]
    pub total_status_code_78: u32,
    #[bryl(numeric(9))]
    pub total_status_code_80: u32,
    #[bryl(numeric(9))]
    pub total_status_code_82: u32,
    #[bryl(numeric(9))]
    pub total_status_code_83: u32,
    #[bryl(numeric(9))]
    pub total_status_code_84: u32,
    #[bryl(numeric(9))]
    pub total_status_code_88: u32,
    #[bryl(numeric(9))]
    pub total_status_code_89: u32,
    #[bryl(numeric(9))]
    pub total_status_code_93: u32,
    #[bryl(numeric(9))]
    pub total_status_code_94: u32,
    #[bryl(numeric(9))]
    pub total_status_code_95: u32,
    #[bryl(numeric(9))]
    pub total_status_code_96: u32,
    #[bryl(numeric(9))]
    pub total_status_code_97: u32,
    #[bryl(numeric(9))]
    pub total_ecoa_code_z: u32,
    #[bryl(numeric(9))]
    pub total_n1_segments: u32,
    #[bryl(numeric(9))]
    pub total_k1_segments: u32,
    #[bryl(numeric(9))]
    pub total_k2_segments: u32,
    #[bryl(numeric(9))]
    pub total_k3_segments: u32,
    #[bryl(numeric(9))]
    pub total_k4_segments: u32,
    #[bryl(numeric(9))]
    pub total_l1_segments: u32,
    #[bryl(numeric(9))]
    pub total_ssns_all_segments: u32,
    #[bryl(numeric(9))]
    pub total_ssns_base_segments: u32,
    #[bryl(numeric(9))]
    pub total_ssns_j1_segments: u32,
    #[bryl(numeric(9))]
    pub total_ssns_j2_segments: u32,
    #[bryl(numeric(9))]
    pub total_dobs_all_segments: u32,
    #[bryl(numeric(9))]
    pub total_dobs_base_segments: u32,
    #[bryl(numeric(9))]
    pub total_dobs_j1_segments: u32,
    #[bryl(numeric(9))]
    pub total_dobs_j2_segments: u32,
    /// Telephone numbers in base, J1 and J2 segments.
    #[bryl(numeric(9))]
    pub total_telephone_numbers: u32,
    /// Fixed by the format; stores nothing.
    #[bryl(alpha(19), reserved)]
    pub reserved_2: Const,
}
