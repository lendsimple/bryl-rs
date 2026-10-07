"""Generate golden files from lms-python's nacha.py (and later metro2.py).

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


def main() -> None:
    backend = Path(sys.argv[1]).resolve()
    sys.path.insert(0, str(backend))
    from common import nacha

    out_dir = REPO / "crates" / "bryl-nacha" / "tests" / "fixtures" / "golden"
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, scenario in nacha_scenarios(nacha).items():
        fo = io.StringIO()
        scenario(fo)
        path = out_dir / f"{name}.ach"
        path.write_text(fo.getvalue())
        print(f"wrote {path.relative_to(REPO)}")


if __name__ == "__main__":
    main()
