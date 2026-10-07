//! Metro 2 code tables, generated from `metro2.py`'s dictionaries. Variant
//! names are the Python keys in `PascalCase`, except the generation codes
//! (`II`–`IX` become `Second`–`Ninth`) and the account statuses, whose Python
//! names were wrong for 61–65, 88, 94–96 and DF (see DEVIATIONS.md, M16).

use bryl::Code;

/// Portfolio type.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortfolioType {
    /// `C`
    #[code("C")]
    LineOfCredit,
    /// `I`
    #[code("I")]
    Installment,
    /// `M`
    #[code("M")]
    Mortgage,
    /// `O`
    #[code("O")]
    Open,
    /// `R`
    #[code("R")]
    Revolving,
}

/// Account type.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountType {
    /// `00`
    #[code("00")]
    Auto,
    /// `01`
    #[code("01")]
    Unsecured,
    /// `02`
    #[code("02")]
    Secured,
    /// `03`
    #[code("03")]
    PartiallySecured,
    /// `04`
    #[code("04")]
    HomeImprovement,
    /// `05`
    #[code("05")]
    FhaHomeImprovement,
    /// `06`
    #[code("06")]
    InstallmentSalesContract,
    /// `07`
    #[code("07")]
    ChargeAccount,
    /// `08`
    #[code("08")]
    RealEstateSpecific,
    /// `0A`
    #[code("0A")]
    AutoLease,
    /// `0C`
    #[code("0C")]
    RealEstateJuniorLiens,
    /// `0F`
    #[code("0F")]
    FlexibleSpendingCreditCard,
    /// `0G`
    #[code("0G")]
    ManufacturedHousing,
    /// `10`
    #[code("10")]
    Business,
    /// `11`
    #[code("11")]
    RecreationalMerchandise,
    /// `12`
    #[code("12")]
    Education,
    /// `13`
    #[code("13")]
    Lease,
    /// `15`
    #[code("15")]
    CheckCredit,
    /// `17`
    #[code("17")]
    MobileHome,
    /// `18`
    #[code("18")]
    CreditCard,
    /// `19`
    #[code("19")]
    FhaRealEstateMortgage,
    /// `20`
    #[code("20")]
    NoteLoan,
    /// `22`
    #[code("22")]
    SecuredByHouseholdGoods,
    /// `23`
    #[code("23")]
    SecuredByHhgAndOther,
    /// `25`
    #[code("25")]
    VaRealEstateMortgage,
    /// `26`
    #[code("26")]
    ConventionalRealEstate,
    /// `29`
    #[code("29")]
    RentalAgreement,
    /// `2A`
    #[code("2A")]
    SecuredCreditCard,
    /// `2C`
    #[code("2C")]
    CommercialLineOfCredit,
    /// `37`
    #[code("37")]
    CombinedCreditPlan,
    /// `3A`
    #[code("3A")]
    Agricultural,
    /// `43`
    #[code("43")]
    DebitCard,
    /// `47`
    #[code("47")]
    CreditLineSecured,
    /// `48`
    #[code("48")]
    CollectionAgency,
    /// `4D`
    #[code("4D")]
    CommercialInstallment,
    /// `50`
    #[code("50")]
    FamilySupport,
    /// `5A`
    #[code("5A")]
    GovernmentEmployeeAdvance,
    /// `5B`
    #[code("5B")]
    GovernmentFeeForServices,
    /// `65`
    #[code("65")]
    GovtUnsecuredGuaranteed,
    /// `66`
    #[code("66")]
    GovtSecuredGuaranteed,
    /// `67`
    #[code("67")]
    GovtUnsecuredDirect,
    /// `68`
    #[code("68")]
    GovtSecuredDirect,
    /// `69`
    #[code("69")]
    GovernmentGrant,
    /// `6A`
    #[code("6A")]
    GovernmentOverpayment,
    /// `6B`
    #[code("6B")]
    GovernmentFine,
    /// `6D`
    #[code("6D")]
    GovernmentMiscellaneous,
    /// `70`
    #[code("70")]
    GovernmentPurchaseProgram70,
    /// `71`
    #[code("71")]
    GovernmentPurchaseProgram71,
    /// `72`
    #[code("72")]
    GovernmentPurchaseProgram72,
    /// `73`
    #[code("73")]
    GovernmentPurchaseProgram73,
    /// `74`
    #[code("74")]
    GovernmentPurchaseProgram74,
    /// `75`
    #[code("75")]
    GovernmentPurchaseProgram75,
    /// `77`
    #[code("77")]
    ReturnedCheck,
    /// `7A`
    #[code("7A")]
    HomeEquityLineOfCredit7a,
    /// `7B`
    #[code("7B")]
    HomeEquityLineOfCredit7b,
    /// `89`
    #[code("89")]
    HomeEquityInstallment,
    /// `8A`
    #[code("8A")]
    HomeEquityLineOfCredit8a,
    /// `8B`
    #[code("8B")]
    HomeEquityLineOfCredit8b,
    /// `90`
    #[code("90")]
    MedicalDebt,
    /// `91`
    #[code("91")]
    DebtConsolidation,
    /// `92`
    #[code("92")]
    UtilityCompany,
    /// `93`
    #[code("93")]
    ChildSupport,
    /// `95`
    #[code("95")]
    TelecomCellular,
    /// `9A`
    #[code("9A")]
    CommercialMortgage,
    /// `9B`
    #[code("9B")]
    CommercialSecured,
    /// `78`
    #[code("78")]
    InstallmentLoan,
}

/// Account status.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccountStatus {
    /// `05`: account transferred to another office.
    #[code("05")]
    Transferred,
    /// `11`: current account (0–29 days past the due date).
    #[code("11")]
    Current,
    /// `13`: paid or closed account, zero balance.
    #[code("13")]
    PaidOrClosed,
    /// `61`: account paid in full, was a voluntary surrender.
    #[code("61")]
    PaidVoluntarySurrender,
    /// `62`: account paid in full, was a collection account.
    #[code("62")]
    PaidCollection,
    /// `63`: account paid in full, was a repossession.
    #[code("63")]
    PaidRepossession,
    /// `64`: account paid in full, was a charge-off.
    #[code("64")]
    PaidChargeOff,
    /// `65`: account paid in full; a foreclosure was started.
    #[code("65")]
    PaidForeclosureStarted,
    /// `71`: 30–59 days past the due date.
    #[code("71")]
    Dpd30,
    /// `78`: 60–89 days past the due date.
    #[code("78")]
    Dpd60,
    /// `80`: 90–119 days past the due date.
    #[code("80")]
    Dpd90,
    /// `82`: 120–149 days past the due date.
    #[code("82")]
    Dpd120,
    /// `83`: 150–179 days past the due date.
    #[code("83")]
    Dpd150,
    /// `84`: 180 or more days past the due date.
    #[code("84")]
    Dpd180,
    /// `88`: claim filed with the government for the insured portion of the
    /// balance on a defaulted loan.
    #[code("88")]
    GovernmentClaimFiled,
    /// `89`: deed received in lieu of foreclosure on a defaulted mortgage;
    /// there may be a balance due.
    #[code("89")]
    DeedReceived,
    /// `93`: account assigned to internal or external collections.
    #[code("93")]
    Collections,
    /// `94`: foreclosure completed; there may be a balance due.
    #[code("94")]
    ForeclosureCompleted,
    /// `95`: voluntary surrender; there may be a balance due.
    #[code("95")]
    VoluntarySurrender,
    /// `96`: merchandise was repossessed; there may be a balance due.
    #[code("96")]
    Repossession,
    /// `97`: unpaid balance reported as a loss (charge-off).
    #[code("97")]
    ChargeOff,
    /// `DA`: delete the entire account, for reasons other than fraud.
    #[code("DA")]
    DeleteAccount,
    /// `DF`: delete the entire account due to confirmed fraud (fraud
    /// investigation completed).
    #[code("DF")]
    DeleteAccountFraud,
}

/// Payment rating.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentRating {
    /// `0`
    #[code("0")]
    Current,
    /// `1`
    #[code("1")]
    Past30,
    /// `2`
    #[code("2")]
    Past60,
    /// `3`
    #[code("3")]
    Past90,
    /// `4`
    #[code("4")]
    Past120,
    /// `5`
    #[code("5")]
    Past150,
    /// `6`
    #[code("6")]
    Past180,
    /// `G`
    #[code("G")]
    Collection,
    /// `L`
    #[code("L")]
    ChargeOff,
}

/// One month of the 24-month payment history profile.
///
/// The blank code (no history reported, `" "` in Python) is not a variant:
/// it is a space in [`crate::PaymentHistoryProfile`], because codes cannot be
/// blank.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaymentHistoryCode {
    /// `0`
    #[code("0")]
    Current,
    /// `1`
    #[code("1")]
    Past30,
    /// `2`
    #[code("2")]
    Past60,
    /// `3`
    #[code("3")]
    Past90,
    /// `4`
    #[code("4")]
    Past120,
    /// `5`
    #[code("5")]
    Past150,
    /// `6`
    #[code("6")]
    Past180,
    /// `B`
    #[code("B")]
    NoHistoryPrior,
    /// `D`
    #[code("D")]
    NoDataThisMonth,
    /// `E`
    #[code("E")]
    ZeroBalanceCurrent,
    /// `G`
    #[code("G")]
    Collection,
    /// `H`
    #[code("H")]
    ForeclosureCompleted,
    /// `J`
    #[code("J")]
    VoluntarySurrender,
    /// `K`
    #[code("K")]
    Repossession,
    /// `L`
    #[code("L")]
    ChargeOff,
    /// `Z`
    #[code("Z")]
    TooNewToRate,
}

/// Equal Credit Opportunity Act (ECOA) code.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EcoaCode {
    /// `1`
    #[code("1")]
    Individual,
    /// `2`
    #[code("2")]
    Joint,
    /// `3`
    #[code("3")]
    AuthorizedUser,
    /// `5`
    #[code("5")]
    CoMaker,
    /// `7`
    #[code("7")]
    Maker,
    /// `T`
    #[code("T")]
    Terminated,
    /// `W`
    #[code("W")]
    Business,
    /// `X`
    #[code("X")]
    Deceased,
    /// `Z`
    #[code("Z")]
    Delete,
}

/// Consumer information indicator (bankruptcy and similar).
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConsumerInformationIndicator {
    /// `A`
    #[code("A")]
    BankruptcyCh7Petition,
    /// `B`
    #[code("B")]
    BankruptcyCh11Petition,
    /// `C`
    #[code("C")]
    BankruptcyCh12Petition,
    /// `D`
    #[code("D")]
    BankruptcyCh13Petition,
    /// `E`
    #[code("E")]
    BankruptcyCh7Discharged,
    /// `F`
    #[code("F")]
    BankruptcyCh11Discharged,
    /// `G`
    #[code("G")]
    BankruptcyCh12Discharged,
    /// `H`
    #[code("H")]
    BankruptcyCh13Discharged,
    /// `I`
    #[code("I")]
    BankruptcyCh7Dismissed,
    /// `J`
    #[code("J")]
    BankruptcyCh11Dismissed,
    /// `K`
    #[code("K")]
    BankruptcyCh12Dismissed,
    /// `L`
    #[code("L")]
    BankruptcyCh13Dismissed,
    /// `M`
    #[code("M")]
    BankruptcyCh7Withdrawn,
    /// `N`
    #[code("N")]
    BankruptcyCh11Withdrawn,
    /// `O`
    #[code("O")]
    BankruptcyCh12Withdrawn,
    /// `P`
    #[code("P")]
    BankruptcyCh13Withdrawn,
    /// `Q`
    #[code("Q")]
    RemovePriorBankruptcy,
    /// `R`
    #[code("R")]
    ReaffirmationDebt,
    /// `S`
    #[code("S")]
    RemoveReaffirmation,
    /// `T`
    #[code("T")]
    CannotLocateConsumer,
    /// `U`
    #[code("U")]
    ConsumerNowLocated,
    /// `V`
    #[code("V")]
    RescissionReaffirmation,
    /// `Z`
    #[code("Z")]
    UndesignatedChapter,
    /// `1A`
    #[code("1A")]
    PersonalReceivership,
    /// `2A`
    #[code("2A")]
    LeaseAssumption,
}

/// Compliance condition code (disputes and closures).
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComplianceConditionCode {
    /// `XA`
    #[code("XA")]
    ClosedAtRequest,
    /// `XB`
    #[code("XB")]
    FcraDispute,
    /// `XC`
    #[code("XC")]
    FcraInvestigatedDisagrees,
    /// `XD`
    #[code("XD")]
    ClosedAndFcraDispute,
    /// `XE`
    #[code("XE")]
    ClosedAndFcraInvestigated,
    /// `XF`
    #[code("XF")]
    FcbaDispute,
    /// `XG`
    #[code("XG")]
    FcbaResolvedDisagrees,
    /// `XH`
    #[code("XH")]
    PreviouslyInDispute,
    /// `XJ`
    #[code("XJ")]
    ClosedAndFcbaDispute,
    /// `XR`
    #[code("XR")]
    RemoveComplianceCode,
}

/// Special comment code.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialComment {
    /// `B`
    #[code("B")]
    B,
    /// `C`
    #[code("C")]
    C,
    /// `H`
    #[code("H")]
    H,
    /// `I`
    #[code("I")]
    I,
    /// `M`
    #[code("M")]
    M,
    /// `O`
    #[code("O")]
    O,
    /// `S`
    #[code("S")]
    S,
    /// `V`
    #[code("V")]
    V,
    /// `AB`
    #[code("AB")]
    Ab,
    /// `AC`
    #[code("AC")]
    Ac,
    /// `AH`
    #[code("AH")]
    Ah,
    /// `AI`
    #[code("AI")]
    Ai,
    /// `AM`
    #[code("AM")]
    Am,
    /// `AN`
    #[code("AN")]
    An,
    /// `AO`
    #[code("AO")]
    Ao,
    /// `AP`
    #[code("AP")]
    Ap,
    /// `AS`
    #[code("AS")]
    As,
    /// `AT`
    #[code("AT")]
    At,
    /// `AU`
    #[code("AU")]
    Au,
    /// `AV`
    #[code("AV")]
    Av,
    /// `AW`
    #[code("AW")]
    Aw,
    /// `AX`
    #[code("AX")]
    Ax,
    /// `AZ`
    #[code("AZ")]
    Az,
    /// `BA`
    #[code("BA")]
    Ba,
    /// `BB`
    #[code("BB")]
    Bb,
    /// `BC`
    #[code("BC")]
    Bc,
    /// `BD`
    #[code("BD")]
    Bd,
    /// `BE`
    #[code("BE")]
    Be,
    /// `BF`
    #[code("BF")]
    Bf,
    /// `BG`
    #[code("BG")]
    Bg,
    /// `BH`
    #[code("BH")]
    Bh,
    /// `BI`
    #[code("BI")]
    Bi,
    /// `BJ`
    #[code("BJ")]
    Bj,
    /// `BK`
    #[code("BK")]
    Bk,
    /// `BL`
    #[code("BL")]
    Bl,
    /// `BN`
    #[code("BN")]
    Bn,
    /// `BO`
    #[code("BO")]
    Bo,
    /// `BP`
    #[code("BP")]
    Bp,
    /// `BS`
    #[code("BS")]
    Bs,
    /// `BT`
    #[code("BT")]
    Bt,
    /// `CH`
    #[code("CH")]
    Ch,
    /// `CI`
    #[code("CI")]
    Ci,
    /// `CJ`
    #[code("CJ")]
    Cj,
    /// `CK`
    #[code("CK")]
    Ck,
    /// `CL`
    #[code("CL")]
    Cl,
    /// `CM`
    #[code("CM")]
    Cm,
    /// `CN`
    #[code("CN")]
    Cn,
    /// `CO`
    #[code("CO")]
    Co,
    /// `CP`
    #[code("CP")]
    Cp,
    /// `CS`
    #[code("CS")]
    Cs,
    /// `DE`
    #[code("DE")]
    De,
}

/// Terms frequency.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermsFrequency {
    /// `D`
    #[code("D")]
    Deferred,
    /// `P`
    #[code("P")]
    SinglePayment,
    /// `W`
    #[code("W")]
    Weekly,
    /// `B`
    #[code("B")]
    Biweekly,
    /// `E`
    #[code("E")]
    Semimonthly,
    /// `M`
    #[code("M")]
    Monthly,
    /// `L`
    #[code("L")]
    Bimonthly,
    /// `Q`
    #[code("Q")]
    Quarterly,
    /// `T`
    #[code("T")]
    TriAnnually,
    /// `S`
    #[code("S")]
    Semiannually,
    /// `Y`
    #[code("Y")]
    Annually,
}

/// Address indicator.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AddressIndicator {
    /// `C`
    #[code("C")]
    Confirmed,
    /// `Y`
    #[code("Y")]
    Known,
    /// `N`
    #[code("N")]
    NotConfirmed,
    /// `M`
    #[code("M")]
    Military,
    /// `S`
    #[code("S")]
    Secondary,
    /// `B`
    #[code("B")]
    Business,
    /// `U`
    #[code("U")]
    NonDeliverable,
    /// `D`
    #[code("D")]
    DataReporterDefault,
    /// `P`
    #[code("P")]
    BillPayerService,
}

/// Residence code.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResidenceCode {
    /// `O`
    #[code("O")]
    Owns,
    /// `R`
    #[code("R")]
    Rents,
}

/// Generation code (Jr., Sr., II, ...).
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenerationCode {
    /// `J`
    #[code("J")]
    Junior,
    /// `S`
    #[code("S")]
    Senior,
    /// `2`
    #[code("2")]
    Second,
    /// `3`
    #[code("3")]
    Third,
    /// `4`
    #[code("4")]
    Fourth,
    /// `5`
    #[code("5")]
    Fifth,
    /// `6`
    #[code("6")]
    Sixth,
    /// `7`
    #[code("7")]
    Seventh,
    /// `8`
    #[code("8")]
    Eighth,
    /// `9`
    #[code("9")]
    Ninth,
}

/// Interest type indicator.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterestTypeIndicator {
    /// `F`
    #[code("F")]
    Fixed,
    /// `V`
    #[code("V")]
    Variable,
}

/// K1 creditor classification.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreditorClassification {
    /// `1`
    #[code(1)]
    Retail,
    /// `2`
    #[code(2)]
    Medical,
    /// `3`
    #[code(3)]
    OilCompany,
    /// `4`
    #[code(4)]
    Government,
    /// `5`
    #[code(5)]
    PersonalServices,
    /// `6`
    #[code(6)]
    Insurance,
    /// `7`
    #[code(7)]
    Educational,
    /// `8`
    #[code(8)]
    Banking,
    /// `9`
    #[code(9)]
    RentalLeasing,
    /// `10`
    #[code(10)]
    Utilities,
    /// `11`
    #[code(11)]
    CableCellular,
    /// `12`
    #[code(12)]
    Financial,
    /// `13`
    #[code(13)]
    CreditUnion,
    /// `14`
    #[code(14)]
    Automotive,
    /// `15`
    #[code(15)]
    CheckGuarantee,
}

/// L1 change indicator.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeIndicator {
    /// `1`
    #[code(1)]
    AccountNumber,
    /// `2`
    #[code(2)]
    IdentificationNumber,
    /// `3`
    #[code(3)]
    Both,
}

/// K2 purchased/sold indicator.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PurchasedIndicator {
    /// `1`
    #[code(1)]
    PurchasedFrom,
    /// `2`
    #[code(2)]
    SoldTo,
    /// `9`
    #[code(9)]
    Remove,
}

/// K3 agency identifier.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgencyIdentifier {
    /// `0`
    #[code(0)]
    NotApplicable,
    /// `1`
    #[code(1)]
    FannieMae,
    /// `2`
    #[code(2)]
    FreddieMac,
}

/// K4 specialized payment indicator.
#[derive(Code, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecializedPaymentIndicator {
    /// `1`
    #[code(1)]
    Balloon,
    /// `2`
    #[code(2)]
    Deferred,
}
