//! NACHA code tables.

use bryl::Code;

/// Batch service class code: which kinds of entries a batch may contain.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceClassCode {
    /// `200`: debits and credits.
    #[code(200)]
    MixedDebitsAndCredits,
    /// `220`: credits only.
    #[code(220)]
    CreditsOnly,
    /// `225`: debits only.
    #[code(225)]
    DebitsOnly,
}

impl ServiceClassCode {
    /// True if a batch with this code may contain the transaction.
    pub const fn allows(self, code: TransactionCode) -> bool {
        match self {
            Self::MixedDebitsAndCredits => true,
            Self::CreditsOnly => code.is_credit(),
            Self::DebitsOnly => code.is_debit(),
        }
    }
}

/// Standard entry class (SEC) code.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardEntryClass {
    /// Accounts receivable entry.
    #[code("ARC")]
    Arc,
    /// Back office conversion entry.
    #[code("BOC")]
    Boc,
    /// Customer initiated entry.
    #[code("CIE")]
    Cie,
    /// Machine transfer entry.
    #[code("MTE")]
    Mte,
    /// Consumer cross-border payment.
    #[code("PBR")]
    Pbr,
    /// Point-of-purchase entry.
    #[code("POP")]
    Pop,
    /// Prearranged payment and deposit.
    #[code("PPD")]
    Ppd,
    /// Point-of-sale entry.
    #[code("POS")]
    Pos,
    /// Shared network transaction.
    #[code("SHR")]
    Shr,
    /// Re-presented check entry.
    #[code("RCK")]
    Rck,
    /// Telephone-initiated entry.
    #[code("TEL")]
    Tel,
    /// Internet-initiated entry.
    #[code("WEB")]
    Web,
    /// Corporate cross-border payment.
    #[code("CBR")]
    Cbr,
    /// Cash concentration or disbursement.
    #[code("CCD")]
    Ccd,
    /// Corporate trade exchange.
    #[code("CTX")]
    Ctx,
    /// Acknowledgment entry (CCD).
    #[code("ACK")]
    Ack,
    /// Acknowledgment entry (CTX).
    #[code("ATX")]
    Atx,
    /// Automated accounting advice.
    #[code("ADV")]
    Adv,
    /// Notification of change or refused notification of change.
    #[code("COR")]
    Cor,
    /// Death notification entry.
    #[code("DNE")]
    Dne,
    /// Automated enrollment entry.
    #[code("ENR")]
    Enr,
    /// Truncated entry.
    #[code("TRC")]
    Trc,
    /// Truncated entries exchange.
    #[code("TRX")]
    Trx,
    /// Destroyed check entry.
    #[code("XCK")]
    Xck,
}

impl StandardEntryClass {
    /// Most type-05 addenda records an entry of this class may carry.
    ///
    /// - At most one for PPD, CCD and WEB, none for TEL, and up to 9,999 for
    ///   CTX (NACHA's developer guide and bank specifications).
    /// - None for the other check conversion classes (ARC, BOC, POP, RCK,
    ///   TRC, XCK), and up to 9,999 for ATX and TRX (moov-io/ach).
    /// - None for ADV, which has no addenda, and for COR (type 98) and MTE,
    ///   POS and SHR (type 02): this library writes only type-05 and return
    ///   (type-99) addenda, so entries of these classes cannot carry their
    ///   required addenda. Return entries carry no type-05 addenda at all.
    /// - One for everything else (CIE, ACK, DNE, ENR; PBR and CBR, replaced by
    ///   IAT, are not checked by moov-io).
    pub const fn max_addenda(self) -> u16 {
        match self {
            Self::Arc
            | Self::Boc
            | Self::Pop
            | Self::Rck
            | Self::Trc
            | Self::Xck
            | Self::Tel
            | Self::Adv
            | Self::Cor
            | Self::Mte
            | Self::Pos
            | Self::Shr => 0,
            Self::Ctx | Self::Atx | Self::Trx => 9999,
            _ => 1,
        }
    }

    /// Fewest type-05 addenda records an entry of this class must carry: one
    /// for DNE and ENR (moov-io/ach), otherwise none.
    pub const fn min_addenda(self) -> u16 {
        match self {
            Self::Dne | Self::Enr => 1,
            _ => 0,
        }
    }

    /// True for classes whose entries must be debits: telephone-initiated
    /// (TEL) and check conversion (ARC, BOC, POP, RCK, TRC) entries debit the
    /// consumer. Credits are allowed only in a reversal batch (company entry
    /// description `REVERSAL`). Follows moov-io/ach and NACHA's developer
    /// guide.
    pub const fn debits_only(self) -> bool {
        matches!(
            self,
            Self::Tel | Self::Arc | Self::Boc | Self::Pop | Self::Rck | Self::Trc
        )
    }

    /// True for classes whose entries must be credits: customer-initiated
    /// entries (CIE) move money to the biller (moov-io/ach).
    pub const fn credits_only(self) -> bool {
        matches!(self, Self::Cie)
    }

    /// The company entry description batches of this class must use:
    /// `AUTOENROLL` for ENR and `REDEPCHECK` for RCK (moov-io/ach).
    pub const fn required_entry_description(self) -> Option<&'static str> {
        match self {
            Self::Enr => Some("AUTOENROLL"),
            Self::Rck => Some("REDEPCHECK"),
            _ => None,
        }
    }
}

/// Company entry description that marks a reversal batch.
pub const REVERSAL: &str = "REVERSAL";

/// Entry detail transaction code.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionCode {
    /// `21`: return or notification of change for a checking credit.
    #[code(21)]
    CheckingReturnedCredit,
    /// `22`: checking credit.
    #[code(22)]
    CheckingCredit,
    /// `23`: checking credit prenote.
    #[code(23)]
    CheckingPrenoteCredit,
    /// `26`: return or notification of change for a checking debit.
    #[code(26)]
    CheckingReturnedDebit,
    /// `27`: checking debit.
    #[code(27)]
    CheckingDebit,
    /// `28`: checking debit prenote.
    #[code(28)]
    CheckingPrenoteDebit,
    /// `31`: return or notification of change for a savings credit.
    #[code(31)]
    SavingsReturnedCredit,
    /// `32`: savings credit.
    #[code(32)]
    SavingsCredit,
    /// `33`: savings credit prenote.
    #[code(33)]
    SavingsPrenoteCredit,
    /// `36`: return or notification of change for a savings debit.
    #[code(36)]
    SavingsReturnedDebit,
    /// `37`: savings debit.
    #[code(37)]
    SavingsDebit,
    /// `38`: savings debit prenote.
    #[code(38)]
    SavingsPrenoteDebit,
}

/// Return reason code, carried by a [`ReturnAddendum`](crate::ReturnAddendum).
///
/// Covers the return codes R01–R53 (titles as in NACHA's current return code
/// table) and R90. Dishonored and contested dishonored returns (R61–R77) use
/// a different addendum layout and international returns (R80–R85) need IAT,
/// so neither is supported; reading a file that contains them fails.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnReasonCode {
    /// `R01`: Insufficient funds.
    #[code("R01")]
    InsufficientFunds,
    /// `R02`: Account closed.
    #[code("R02")]
    AccountClosed,
    /// `R03`: No account, or unable to locate the account.
    #[code("R03")]
    NoAccount,
    /// `R04`: Invalid account number structure.
    #[code("R04")]
    InvalidAccountNumber,
    /// `R05`: Unauthorized debit to a consumer account using a corporate SEC code.
    #[code("R05")]
    UnauthorizedCorporateDebit,
    /// `R06`: Returned at the ODFI's request.
    #[code("R06")]
    ReturnedPerOdfiRequest,
    /// `R07`: Authorization revoked by the customer.
    #[code("R07")]
    AuthorizationRevoked,
    /// `R08`: Payment stopped.
    #[code("R08")]
    PaymentStopped,
    /// `R09`: Uncollected funds.
    #[code("R09")]
    UncollectedFunds,
    /// `R10`: Customer advises the originator is not known or not authorized.
    #[code("R10")]
    NotAuthorized,
    /// `R11`: Customer advises the entry is not in accordance with the terms of the authorization.
    #[code("R11")]
    NotPerAuthorization,
    /// `R12`: Account sold to another DFI.
    #[code("R12")]
    AccountSold,
    /// `R13`: Invalid ACH routing number.
    #[code("R13")]
    InvalidRoutingNumber,
    /// `R14`: Representative payee deceased or unable to continue in that capacity.
    #[code("R14")]
    RepresentativePayeeDeceased,
    /// `R15`: Beneficiary or account holder deceased.
    #[code("R15")]
    AccountHolderDeceased,
    /// `R16`: Account frozen, or the entry returned per OFAC instruction.
    #[code("R16")]
    AccountFrozen,
    /// `R17`: File record edit criteria, or an entry initiated under questionable circumstances.
    #[code("R17")]
    FileRecordEditCriteria,
    /// `R18`: Improper effective entry date.
    #[code("R18")]
    ImproperEffectiveEntryDate,
    /// `R19`: Amount field error.
    #[code("R19")]
    AmountFieldError,
    /// `R20`: Non-transaction account.
    #[code("R20")]
    NonTransactionAccount,
    /// `R21`: Invalid company identification.
    #[code("R21")]
    InvalidCompanyId,
    /// `R22`: Invalid individual ID number.
    #[code("R22")]
    InvalidIndividualId,
    /// `R23`: Credit entry refused by the receiver.
    #[code("R23")]
    CreditRefused,
    /// `R24`: Duplicate entry.
    #[code("R24")]
    DuplicateEntry,
    /// `R25`: Addenda error.
    #[code("R25")]
    AddendaError,
    /// `R26`: Mandatory field error.
    #[code("R26")]
    MandatoryFieldError,
    /// `R27`: Trace number error.
    #[code("R27")]
    TraceNumberError,
    /// `R28`: Routing number check digit error.
    #[code("R28")]
    RoutingCheckDigitError,
    /// `R29`: Corporate customer advises not authorized.
    #[code("R29")]
    CorporateNotAuthorized,
    /// `R30`: RDFI not a participant in the check truncation program.
    #[code("R30")]
    NotInCheckTruncationProgram,
    /// `R31`: Permissible return entry (CCD and CTX only).
    #[code("R31")]
    PermissibleReturn,
    /// `R32`: RDFI non-settlement.
    #[code("R32")]
    RdfiNonSettlement,
    /// `R33`: Return of an XCK entry.
    #[code("R33")]
    XckReturn,
    /// `R34`: Limited participation DFI.
    #[code("R34")]
    LimitedParticipationDfi,
    /// `R35`: Return of an improper debit entry.
    #[code("R35")]
    ImproperDebit,
    /// `R36`: Return of an improper credit entry.
    #[code("R36")]
    ImproperCredit,
    /// `R37`: Source document presented for payment.
    #[code("R37")]
    SourceDocumentPresented,
    /// `R38`: Stop payment on the source document.
    #[code("R38")]
    SourceDocumentStopped,
    /// `R39`: Improper source document.
    #[code("R39")]
    ImproperSourceDocument,
    /// `R40`: Return of an ENR entry by a federal government agency.
    #[code("R40")]
    EnrReturnedByAgency,
    /// `R41`: Invalid transaction code (ENR only).
    #[code("R41")]
    EnrInvalidTransactionCode,
    /// `R42`: Routing number or check digit error (ENR only).
    #[code("R42")]
    EnrRoutingNumberError,
    /// `R43`: Invalid DFI account number (ENR only).
    #[code("R43")]
    EnrInvalidAccountNumber,
    /// `R44`: Invalid individual ID number (ENR only).
    #[code("R44")]
    EnrInvalidIndividualId,
    /// `R45`: Invalid individual or company name (ENR only).
    #[code("R45")]
    EnrInvalidName,
    /// `R46`: Invalid representative payee indicator (ENR only).
    #[code("R46")]
    EnrInvalidRepresentativePayee,
    /// `R47`: Duplicate enrollment (ENR only).
    #[code("R47")]
    EnrDuplicate,
    /// `R50`: State law affecting RCK acceptance.
    #[code("R50")]
    RckStateLaw,
    /// `R51`: Item related to an RCK entry is ineligible, or the RCK entry is improper.
    #[code("R51")]
    RckIneligible,
    /// `R52`: Stop payment on the item related to an RCK entry.
    #[code("R52")]
    RckStopPayment,
    /// `R53`: Item and RCK entry presented for payment.
    #[code("R53")]
    RckPresented,
    /// `R90`: Returned to meet the RDFI's sanctions compliance obligations (moov-io/ach).
    #[code("R90")]
    SanctionsCompliance,
}

impl ReturnReasonCode {
    /// True for R14 and R15, the only codes whose return addendum may carry a
    /// date of death.
    pub const fn allows_date_of_death(self) -> bool {
        matches!(
            self,
            Self::RepresentativePayeeDeceased | Self::AccountHolderDeceased
        )
    }
}

/// Kind of receiving account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountKind {
    /// Checking (demand deposit) account.
    Checking,
    /// Savings account.
    Savings,
}

impl TransactionCode {
    /// Picks the transaction code for an entry: returns use the "returned"
    /// code, prenotes and zero amounts use the prenote code, negative amounts
    /// are debits and positive amounts credits.
    pub const fn for_entry(
        amount: i64,
        account: AccountKind,
        is_return: bool,
        is_prenote: bool,
    ) -> Self {
        let debit = amount < 0;
        let prenote = !is_return && (is_prenote || amount == 0);
        match (account, debit, is_return, prenote) {
            (AccountKind::Checking, false, true, _) => Self::CheckingReturnedCredit,
            (AccountKind::Checking, false, false, true) => Self::CheckingPrenoteCredit,
            (AccountKind::Checking, false, false, false) => Self::CheckingCredit,
            (AccountKind::Checking, true, true, _) => Self::CheckingReturnedDebit,
            (AccountKind::Checking, true, false, true) => Self::CheckingPrenoteDebit,
            (AccountKind::Checking, true, false, false) => Self::CheckingDebit,
            (AccountKind::Savings, false, true, _) => Self::SavingsReturnedCredit,
            (AccountKind::Savings, false, false, true) => Self::SavingsPrenoteCredit,
            (AccountKind::Savings, false, false, false) => Self::SavingsCredit,
            (AccountKind::Savings, true, true, _) => Self::SavingsReturnedDebit,
            (AccountKind::Savings, true, false, true) => Self::SavingsPrenoteDebit,
            (AccountKind::Savings, true, false, false) => Self::SavingsDebit,
        }
    }

    /// The receiving account kind.
    pub const fn account(self) -> AccountKind {
        if self.as_code() / 10 == 2 {
            AccountKind::Checking
        } else {
            AccountKind::Savings
        }
    }

    /// True for checking account codes (2x).
    pub const fn is_checking(self) -> bool {
        matches!(self.account(), AccountKind::Checking)
    }

    /// True for savings account codes (3x).
    pub const fn is_savings(self) -> bool {
        matches!(self.account(), AccountKind::Savings)
    }

    /// True for credits, including credit returns and prenotes (x1–x3).
    pub const fn is_credit(self) -> bool {
        matches!(self.as_code() % 10, 1..=3)
    }

    /// True for debits, including debit returns and prenotes (x6–x8).
    pub const fn is_debit(self) -> bool {
        matches!(self.as_code() % 10, 6..=8)
    }

    /// True for prenotes (x3, x8), which must have a zero amount.
    pub const fn is_prenote(self) -> bool {
        matches!(self.as_code() % 10, 3 | 8)
    }

    /// True for returns and notifications of change (x1, x6).
    pub const fn is_return(self) -> bool {
        matches!(self.as_code() % 10, 1 | 6)
    }
}
