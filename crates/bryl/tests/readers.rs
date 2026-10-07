//! Readers.
//!
//! | `test_bryl.py`          | Here |
//! |-------------------------|------|
//! | `TestLineReader::*`     | `line::*` |
//! | `TestBlockReader::*`    | `block::*` |
//! | `TestMalformedError::*` | `error::*` |
//!
//! Python's `next_record(expected_type, default)` maps to `Reader::expect`
//! (raise on EOF or wrong type) and `Reader::next_if` (`default=None`).

mod common;

use std::io::{self, BufReader, Read};

use bryl::read::{BlockSource, Dispatch, LineSource, Location, ReadError, ReadErrorKind, Reader};
use bryl::{Const, Record};
use common::SampleRecord;

#[derive(Record, Debug, Clone, PartialEq)]
struct TaggedA {
    #[bryl(alpha(1), constant = "A")]
    tag: Const,
    #[bryl(alpha(9))]
    value: String,
}

#[derive(Record, Debug, Clone, PartialEq)]
struct TaggedB {
    #[bryl(alpha(1), constant = "B")]
    tag: Const,
    #[bryl(numeric(9))]
    value: u32,
}

/// Port of `_as_record_type_for_line`: dispatch on the first character.
#[derive(Debug, Clone, PartialEq)]
enum Tagged {
    A(TaggedA),
    B(TaggedB),
}

impl Dispatch for Tagged {
    fn dispatch(raw: &[u8]) -> Result<Self, ReadErrorKind> {
        match raw.first() {
            Some(b'A') => Ok(Self::A(TaggedA::decode(raw)?)),
            Some(b'B') => Ok(Self::B(TaggedB::decode(raw)?)),
            other => Err(ReadErrorKind::UnknownRecord(format!(
                "{:?}",
                other.map(|&b| char::from(b))
            ))),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::A(_) => TaggedA::NAME,
            Self::B(_) => TaggedB::NAME,
        }
    }
}

fn a(value: &str) -> TaggedA {
    TaggedA {
        tag: Const,
        value: value.into(),
    }
}

fn b(value: u32) -> TaggedB {
    TaggedB { tag: Const, value }
}

fn lines(records: &[String]) -> String {
    records.iter().map(|r| r.clone() + "\n").collect()
}

fn line_reader(input: &str) -> Reader<LineSource<&[u8]>, Tagged> {
    Reader::new(LineSource::new(input.as_bytes()))
}

fn expect_a(reader: &mut Reader<LineSource<&[u8]>, Tagged>) -> Result<TaggedA, ReadError> {
    reader.expect("TaggedA", |r| match r {
        Tagged::A(a) => Ok(a),
        other @ Tagged::B(_) => Err(other),
    })
}

mod line {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn iterate_lines() {
        let input = lines(&[a("foobar").encode().unwrap(), b(123).encode().unwrap()]);
        let records: Vec<_> = line_reader(&input).collect::<Result<_, _>>().unwrap();
        assert_eq!(records, [Tagged::A(a("foobar")), Tagged::B(b(123))]);
    }

    #[test]
    fn eof_stops_iteration() {
        assert_eq!(line_reader("").count(), 0);
    }

    #[test]
    fn expect_returns_record() {
        let input = lines(&[a("test").encode().unwrap()]);
        assert_eq!(expect_a(&mut line_reader(&input)).unwrap(), a("test"));
    }

    #[test]
    fn next_if_returns_none_on_eof() {
        let mut reader = line_reader("");
        assert_eq!(reader.next_if(Ok).unwrap(), None);
        assert_eq!(reader.next_record().unwrap(), None);
    }

    #[test]
    fn expect_raises_on_eof() {
        let err = expect_a(&mut line_reader("")).unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::UnexpectedEof { .. }));
        assert!(err.to_string().contains("unexpected EOF"), "{err}");
    }

    #[test]
    fn wrong_type_stays_queued() {
        let input = lines(&[b(1).encode().unwrap(), a("x").encode().unwrap()]);
        let mut reader = line_reader(&input);

        let err = expect_a(&mut reader).unwrap_err();
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedRecord {
                found: "TaggedB",
                ..
            }
        ));
        assert_eq!(err.location, Location::Line(1));

        let not_a = reader
            .next_if(|r| match r {
                Tagged::A(a) => Ok(a),
                other @ Tagged::B(_) => Err(other),
            })
            .unwrap();
        assert_eq!(not_a, None);

        // The B record is still there, then the A.
        assert_eq!(reader.next_record().unwrap(), Some(Tagged::B(b(1))));
        assert_eq!(expect_a(&mut reader).unwrap(), a("x"));
    }

    #[test]
    fn peek_does_not_consume() {
        let input = lines(&[a("x").encode().unwrap()]);
        let mut reader = line_reader(&input);
        assert_eq!(reader.peek().unwrap(), Some(&Tagged::A(a("x"))));
        assert_eq!(reader.peek().unwrap(), Some(&Tagged::A(a("x"))));
        assert_eq!(reader.next_record().unwrap(), Some(Tagged::A(a("x"))));
        assert_eq!(reader.peek().unwrap(), None);
    }

    #[test]
    fn unknown_record_is_located_and_consumed() {
        let input = lines(&[
            a("x").encode().unwrap(),
            "XINVALID  ".into(),
            b(2).encode().unwrap(),
        ]);
        let results: Vec<_> = line_reader(&input).collect();
        assert_eq!(results.len(), 3);
        let err = results[1].as_ref().unwrap_err();
        assert_eq!(err.location, Location::Line(2));
        assert!(matches!(err.kind, ReadErrorKind::UnknownRecord(_)));
        assert_eq!(results[2].as_ref().unwrap(), &Tagged::B(b(2)));
    }

    #[test]
    fn invalid_field_is_record_error() {
        let input = "B00000000x\n";
        let err = line_reader(input).next().unwrap().unwrap_err();
        let ReadErrorKind::Record(err) = err.kind else {
            panic!("expected a record error, got {:?}", err.kind);
        };
        assert!(matches!(*err, bryl::Error::Field { field: "value", .. }));
    }

    #[test]
    fn crlf_and_missing_final_terminator() {
        let input = format!("{}\r\n{}", a("x").encode().unwrap(), b(7).encode().unwrap());
        let records: Vec<_> = line_reader(&input).collect::<Result<_, _>>().unwrap();
        assert_eq!(records, [Tagged::A(a("x")), Tagged::B(b(7))]);
    }

    #[test]
    fn expected_terminator_enforced() {
        let input = format!(
            "{}\r\n{}\n",
            a("x").encode().unwrap(),
            a("y").encode().unwrap()
        );
        let mut reader: Reader<_, Tagged> =
            Reader::new(LineSource::new(input.as_bytes()).expect_terminator(*b"\r\n"));
        assert_eq!(reader.next_record().unwrap(), Some(Tagged::A(a("x"))));
        let err = reader.next_record().unwrap_err();
        assert_eq!(err.location, Location::Line(2));
        assert!(matches!(
            err.kind,
            ReadErrorKind::UnexpectedTerminator { ref found, .. } if found == "\n"
        ));
    }

    #[test]
    fn expected_terminator_allows_unterminated_last_line() {
        let input = a("x").encode().unwrap();
        let mut reader: Reader<_, Tagged> =
            Reader::new(LineSource::new(input.as_bytes()).expect_terminator(*b"\n"));
        assert_eq!(reader.next_record().unwrap(), Some(Tagged::A(a("x"))));
    }

    #[test]
    fn blank_lines() {
        let input = format!("\n{}\n\n", a("x").encode().unwrap());
        // Blank lines are records by default (and fail to dispatch)...
        assert!(line_reader(&input).next().unwrap().is_err());
        // ...unless skipped.
        let reader: Reader<_, Tagged> =
            Reader::new(LineSource::new(input.as_bytes()).skip_blank(true));
        let records: Vec<_> = reader.collect::<Result<_, _>>().unwrap();
        assert_eq!(records, [Tagged::A(a("x"))]);
    }

    #[test]
    fn line_numbers_count_skipped_lines() {
        let input = "\n\nZ\n";
        let mut reader: Reader<_, Tagged> =
            Reader::new(LineSource::new(input.as_bytes()).skip_blank(true));
        assert_eq!(
            reader.next().unwrap().unwrap_err().location,
            Location::Line(3)
        );
    }

    #[test]
    fn single_record_type_needs_no_dispatch_impl() {
        let input = lines(&[SampleRecord::new("x", 1).encode().unwrap()]);
        let reader: Reader<_, SampleRecord> = Reader::new(LineSource::new(input.as_bytes()));
        let records: Vec<_> = reader.collect::<Result<_, _>>().unwrap();
        assert_eq!(records, [SampleRecord::new("x", 1)]);
    }
}

mod block {
    use super::*;
    use pretty_assertions::assert_eq;

    fn block_reader(input: &str) -> Reader<BlockSource<&[u8]>, SampleRecord> {
        Reader::new(BlockSource::new(input.as_bytes(), SampleRecord::LENGTH))
    }

    #[test]
    fn iterate_blocks() {
        let data = SampleRecord::new("hello", 42).encode().unwrap().repeat(3);
        let records: Vec<_> = block_reader(&data).collect::<Result<_, _>>().unwrap();
        assert_eq!(records, vec![SampleRecord::new("hello", 42); 3]);
    }

    #[test]
    fn eof_stops_iteration() {
        assert_eq!(block_reader("").count(), 0);
    }

    #[test]
    fn next_record() {
        let data = SampleRecord::new("test", 1).encode().unwrap();
        let record = block_reader(&data).next_record().unwrap().unwrap();
        assert_eq!(record.alpha, "test");
    }

    #[test]
    fn next_record_none_on_eof() {
        assert_eq!(block_reader("").next_record().unwrap(), None);
    }

    #[test]
    fn locations_are_byte_offsets() {
        let good = SampleRecord::new("ok", 1).encode().unwrap();
        let data = format!("{good}{}{good}", "x".repeat(20));
        let results: Vec<_> = block_reader(&data).collect();
        assert_eq!(
            results[1].as_ref().unwrap_err().location,
            Location::Offset(20)
        );
        assert!(results[2].is_ok());
    }

    #[test]
    fn truncated_final_block() {
        let data = format!("{}short", SampleRecord::new("ok", 1).encode().unwrap());
        let mut reader = block_reader(&data);
        assert!(reader.next_record().unwrap().is_some());
        let err = reader.next_record().unwrap_err();
        assert_eq!(err.location, Location::Offset(20));
        assert!(matches!(
            err.kind,
            ReadErrorKind::Truncated {
                expected: 20,
                actual: 5
            }
        ));
    }

    /// Returns at most 3 bytes per read, to exercise partial reads.
    struct Trickle<'a>(&'a [u8]);

    impl Read for Trickle<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = buf.len().min(3).min(self.0.len());
            buf[..n].copy_from_slice(&self.0[..n]);
            self.0 = &self.0[n..];
            Ok(n)
        }
    }

    #[test]
    fn partial_reads_are_assembled() {
        let data = SampleRecord::new("hello", 42).encode().unwrap().repeat(2);
        let reader: Reader<_, SampleRecord> = Reader::new(BlockSource::new(
            Trickle(data.as_bytes()),
            SampleRecord::LENGTH,
        ));
        assert_eq!(reader.count(), 2);
    }
}

mod error {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn attributes_and_message() {
        let err = ReadError::new(
            "test.dat",
            Location::Offset(42),
            ReadErrorKind::Other("bad data".into()),
        );
        assert_eq!(err.source_name, "test.dat");
        assert_eq!(err.location, Location::Offset(42));
        assert_eq!(err.to_string(), "test.dat @ offset 42 - bad data");
    }

    #[test]
    fn reader_name_appears_in_errors() {
        let mut reader = line_reader("Z\n").with_name("ach.nacha");
        assert_eq!(reader.name(), "ach.nacha");
        let err = reader.next_record().unwrap_err();
        assert_eq!(
            err.to_string(),
            "ach.nacha @ line 1 - unknown record type Some('Z')"
        );
    }

    #[test]
    fn default_name_is_memory() {
        assert_eq!(line_reader("").name(), "<memory>");
    }

    /// Fails on the first read.
    struct Broken;

    impl Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("disk on fire"))
        }
    }

    #[test]
    fn io_errors_surface() {
        let mut reader: Reader<_, Tagged> = Reader::new(LineSource::new(BufReader::new(Broken)));
        let err = reader.next_record().unwrap_err();
        assert!(matches!(err.kind, ReadErrorKind::Io(_)));
        assert!(err.to_string().contains("disk on fire"));

        let mut reader: Reader<_, SampleRecord> = Reader::new(BlockSource::new(Broken, 20));
        assert!(matches!(
            reader.next_record().unwrap_err().kind,
            ReadErrorKind::Io(_)
        ));
    }

    #[test]
    fn is_std_error() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
        assert_error::<ReadError>();
    }
}
