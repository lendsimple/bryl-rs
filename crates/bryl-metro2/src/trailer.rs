//! Computing trailer totals.

use crate::codes::{AccountStatus, EcoaCode};
use crate::data_record::DataRecord;
use crate::records::TrailerRecord;
use crate::validate::{is_valid_dob, is_valid_phone, is_valid_ssn};

impl TrailerRecord {
    /// Computes the trailer for a sequence of data records.
    pub fn from_records<'a>(records: impl IntoIterator<Item = &'a DataRecord>) -> Self {
        let mut trailer = Self::default();
        for record in records {
            trailer.accumulate(record);
        }
        trailer.finalize();
        trailer
    }

    /// Adds one data record's counts. Call [`TrailerRecord::finalize`] after
    /// the last record.
    pub fn accumulate(&mut self, record: &DataRecord) {
        let base = &record.base;
        self.total_base_records += 1;
        *self.status_counter(base.account_status) += 1;

        if is_valid_ssn(base.social_security_number) {
            self.total_ssns_base_segments += 1;
        }
        if is_valid_dob(base.date_of_birth) {
            self.total_dobs_base_segments += 1;
        }
        self.count_contact(base.telephone_number, base.ecoa_code);

        for j1 in &record.j1 {
            self.total_j1_segments += 1;
            if is_valid_ssn(j1.social_security_number) {
                self.total_ssns_j1_segments += 1;
            }
            if is_valid_dob(j1.date_of_birth) {
                self.total_dobs_j1_segments += 1;
            }
            self.count_contact(j1.telephone_number, j1.ecoa_code);
        }
        for j2 in &record.j2 {
            self.total_j2_segments += 1;
            if is_valid_ssn(j2.social_security_number) {
                self.total_ssns_j2_segments += 1;
            }
            if is_valid_dob(j2.date_of_birth) {
                self.total_dobs_j2_segments += 1;
            }
            self.count_contact(j2.telephone_number, j2.ecoa_code);
        }

        self.total_k1_segments += u32::from(record.k1.is_some());
        self.total_k2_segments += u32::from(record.k2.is_some());
        self.total_k3_segments += u32::from(record.k3.is_some());
        self.total_k4_segments += u32::from(record.k4.is_some());
        self.total_l1_segments += u32::from(record.l1.is_some());
        self.total_n1_segments += u32::from(record.n1.is_some());
    }

    /// Fills in the totals derived from the others: the "all segments" SSN
    /// and DOB counts, and the block count (base records plus header and
    /// trailer).
    pub fn finalize(&mut self) {
        self.total_ssns_all_segments = self.total_ssns_base_segments
            + self.total_ssns_j1_segments
            + self.total_ssns_j2_segments;
        self.total_dobs_all_segments = self.total_dobs_base_segments
            + self.total_dobs_j1_segments
            + self.total_dobs_j2_segments;
        self.block_count = self.total_base_records + 2;
    }

    fn count_contact(&mut self, telephone_number: u64, ecoa_code: EcoaCode) {
        if is_valid_phone(telephone_number) {
            self.total_telephone_numbers += 1;
        }
        if ecoa_code == EcoaCode::Delete {
            self.total_ecoa_code_z += 1;
        }
    }

    /// The counter for an account status.
    pub fn status_counter(&mut self, status: AccountStatus) -> &mut u32 {
        match status {
            AccountStatus::Transferred => &mut self.total_status_code_05,
            AccountStatus::Current => &mut self.total_status_code_11,
            AccountStatus::PaidOrClosed => &mut self.total_status_code_13,
            AccountStatus::PaidVoluntarySurrender => &mut self.total_status_code_61,
            AccountStatus::PaidCollection => &mut self.total_status_code_62,
            AccountStatus::PaidRepossession => &mut self.total_status_code_63,
            AccountStatus::PaidChargeOff => &mut self.total_status_code_64,
            AccountStatus::PaidForeclosureStarted => &mut self.total_status_code_65,
            AccountStatus::Dpd30 => &mut self.total_status_code_71,
            AccountStatus::Dpd60 => &mut self.total_status_code_78,
            AccountStatus::Dpd90 => &mut self.total_status_code_80,
            AccountStatus::Dpd120 => &mut self.total_status_code_82,
            AccountStatus::Dpd150 => &mut self.total_status_code_83,
            AccountStatus::Dpd180 => &mut self.total_status_code_84,
            AccountStatus::GovernmentClaimFiled => &mut self.total_status_code_88,
            AccountStatus::DeedReceived => &mut self.total_status_code_89,
            AccountStatus::Collections => &mut self.total_status_code_93,
            AccountStatus::ForeclosureCompleted => &mut self.total_status_code_94,
            AccountStatus::VoluntarySurrender => &mut self.total_status_code_95,
            AccountStatus::Repossession => &mut self.total_status_code_96,
            AccountStatus::ChargeOff => &mut self.total_status_code_97,
            AccountStatus::DeleteAccount => &mut self.total_status_code_da,
            AccountStatus::DeleteAccountFraud => &mut self.total_status_code_df,
        }
    }
}
