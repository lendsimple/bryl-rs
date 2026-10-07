"""Generate golden files from lms-python's nacha.py and metro2.py.

The Rust tests compare their output with these files, after applying the
intentional differences listed in DEVIATIONS.md.

Usage (from the bryl-rs repo root):

    ../lms-python/backend/.venv/bin/python tools/gen_golden.py ../lms-python/backend
"""

from __future__ import annotations

import datetime
import io
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent


def nacha_scenarios(nacha):
    created_at = datetime.datetime(2023, 6, 15, 14, 30)
    effective = datetime.date(2023, 6, 15)

    def file(writer):
        return writer.begin_file(
            immediate_destination="091000019",
            immediate_destination_name="DEST BANK",
            immediate_origin="9876543210",
            immediate_origin_name="ORIGIN BANK",
            created_at=created_at,
        )

    def batch(writer, service_class_code, description="PAYROLL"):
        return writer.begin_company_batch(
            service_class_code=service_class_code,
            company_name="ACME CORP",
            company_id="1234567890",
            standard_entry_class="PPD",
            company_entry_description=description,
            originating_dfi_id=12345678,
            effective_entry_date=effective,
        )

    def entry(writer, transaction_code, amount, name="JANE SMITH", addenda=None):
        writer.entry(
            transaction_code=transaction_code,
            receiving_dfi_routing_number="091000019",
            receiving_dfi_account_number="123456789",
            amount=amount,
            individual_id="EMP001",
            individual_name=name,
            addenda=addenda,
        )

    def single_entry(fo):
        writer = nacha.Writer(fo)
        with file(writer), batch(writer, 200):
            entry(writer, 22, 10000)

    def entries_with_addenda(fo):
        writer = nacha.Writer(fo)
        with file(writer), batch(writer, 200):
            entry(writer, 22, 3000, addenda=["MEMO LINE"])
            entry(writer, 27, 1500, name="JOHN DOE", addenda=["SECOND MEMO"])

    def two_batches(fo):
        writer = nacha.Writer(fo)
        with file(writer):
            with batch(writer, 220):
                for amount in (1000, 2000, 3000):
                    entry(writer, 22, amount)
            with batch(writer, 225, description="BILLING"):
                for amount in (500, 700):
                    entry(writer, 27, amount)

    return {
        "single_entry": single_entry,
        "entries_with_addenda": entries_with_addenda,
        "two_batches": two_batches,
    }


def metro2_scenarios(metro2):
    day = datetime.date(2020, 8, 20)

    def base(**overrides):
        values = dict(
            record_descriptor_word=426,
            identification_number="FURNISHER123",
            consumer_account_number="ACCT-000001",
            portfolio_type="I",
            account_type="01",
            date_opened=datetime.date(2019, 6, 15),
            account_status="11",
            payment_rating="0",
            date_of_account_information=day,
            surname="SMITH",
            first_name="JOHN",
            ecoa_code="1",
            first_line_of_address="123 MAIN ST",
            city="ANYTOWN",
            state="CA",
            zip_code="90210",
        )
        values.update(overrides)
        return metro2.BaseSegment(**values)

    def write(fo, records, newline=False):
        writer = metro2.Writer(fo, newline=newline)
        with writer.begin_file(
            activity_date=day,
            reporter_name="TEST REPORTER",
            reporter_address="123 MAIN ST ANYTOWN US 12345",
            reporter_telephone_number=5551234567,
            program_date=datetime.date(2019, 5, 10),
            software_vendor_name="BRYL",
        ):
            for record in records:
                writer.write_data_record(record)

    def base_only(fo):
        write(fo, [metro2.DataRecord(base=base())])

    def all_segments(fo):
        record = metro2.DataRecord(
            base=base(
                time_stamp=datetime.datetime(2020, 8, 20, 14, 30, 45),
                credit_limit=50000,
                highest_credit=42000,
                terms_duration="036",
                terms_frequency="M",
                scheduled_monthly_payment=350,
                actual_payment_amount=350,
                payment_history_profile="000000000000BBBBBBBBBBBB",
                current_balance=12345,
                date_of_last_payment=datetime.date(2020, 8, 1),
                middle_name="Q",
                generation_code="J",
                social_security_number=123456789,
                date_of_birth=datetime.date(1990, 5, 15),
                telephone_number=5559876543,
                country_code="US",
                address_indicator="C",
                residence_code="O",
            ),
            j1_segments=[
                metro2.J1Segment(
                    surname="SMITH",
                    first_name="JANE",
                    ecoa_code="2",
                    social_security_number=987654321,
                    date_of_birth=datetime.date(1985, 3, 20),
                )
            ],
            j2_segments=[
                metro2.J2Segment(
                    surname="JONES",
                    first_name="BOB",
                    ecoa_code="Z",
                    first_line_of_address="456 OAK AVE",
                    city="OTHERTOWN",
                    state="NY",
                    zip_code="10001",
                    telephone_number=5550001111,
                )
            ],
            k1_segment=metro2.K1Segment(original_creditor_name="FIRST NATIONAL BANK", creditor_classification=8),
            k2_segment=metro2.K2Segment(purchased_indicator=2, purchased_from_sold_to_name="ACME COLLECTIONS"),
            k3_segment=metro2.K3Segment(
                agency_identifier=1,
                account_number="FNM-12345678",
                mortgage_identification_number="MIN-99887766",
            ),
            k4_segment=metro2.K4Segment(
                specialized_payment_indicator=1,
                balloon_payment_due_date=datetime.date(2025, 12, 31),
                balloon_payment_amount=50000,
            ),
            l1_segment=metro2.L1Segment(
                change_indicator=3,
                new_consumer_account_number="NEW-ACCT-999",
                new_identification_number="NEW-ID-888",
            ),
            n1_segment=metro2.N1Segment(employer_name="ACME CORPORATION", occupation="SOFTWARE ENGINEER"),
        )
        write(fo, [record])

    def statuses(newline):
        def scenario(fo):
            write(
                fo,
                [
                    metro2.DataRecord(base=base(consumer_account_number="ACCT-001")),
                    metro2.DataRecord(
                        base=base(
                            consumer_account_number="ACCT-002",
                            account_status="97",
                            payment_rating="L",
                            amount_past_due=1500,
                            original_charge_off_amount=1500,
                            date_of_first_delinquency=datetime.date(2020, 1, 15),
                            special_comment="AU",
                        )
                    ),
                    metro2.DataRecord(
                        base=base(
                            consumer_account_number="ACCT-003",
                            account_status="DA",
                            payment_rating="",
                            ecoa_code="Z",
                            compliance_condition_code="XB",
                            date_closed=datetime.date(2020, 7, 31),
                        )
                    ),
                ],
                newline=newline,
            )

        return scenario

    return {
        "base_only": base_only,
        "all_segments": all_segments,
        "statuses": statuses(False),
        "statuses_newline": statuses(True),
    }


def main() -> None:
    backend = Path(sys.argv[1]).resolve()
    sys.path.insert(0, str(backend))
    from common import metro2, nacha

    out_dir = REPO / "crates" / "bryl-nacha" / "tests" / "fixtures" / "golden"
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, scenario in nacha_scenarios(nacha).items():
        fo = io.StringIO()
        scenario(fo)
        path = out_dir / f"{name}.ach"
        path.write_text(fo.getvalue())
        print(f"wrote {path.relative_to(REPO)}")

    out_dir = REPO / "crates" / "bryl-metro2" / "tests" / "fixtures" / "golden"
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, scenario in metro2_scenarios(metro2).items():
        fo = io.StringIO()
        scenario(fo)
        path = out_dir / f"{name}.dat"
        path.write_text(fo.getvalue())
        print(f"wrote {path.relative_to(REPO)}")


if __name__ == "__main__":
    main()
