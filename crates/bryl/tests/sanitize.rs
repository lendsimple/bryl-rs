//! Sanitize tests.
//!
//! | `test_bryl.py`                          | Here |
//! |-----------------------------------------|------|
//! | `TestContext::*` (global ctx stack)     | `resolve_*`: precedence of field, override and record settings (B4) |
//! | `TestAlphanumeric::test_sanitize_*`     | `filter`, `truncate`, `upper`, `combined` |

mod common;

use bryl::{FieldSpec, Sanitize};
use common::pack_with;
use pretty_assertions::assert_eq;

#[test]
fn default_is_none() {
    assert_eq!(Sanitize::default(), Sanitize::NONE);
    assert_eq!(
        Sanitize::NONE,
        Sanitize {
            upper: false,
            filter: false,
            truncate: false
        }
    );
}

#[test]
fn builder_methods() {
    let s = Sanitize::NONE.upper(true).truncate(true);
    assert!(s.upper && s.truncate && !s.filter);
    assert_eq!(Sanitize::UPPER, Sanitize::NONE.upper(true));
}

#[test]
fn resolve_field_wins() {
    let field = Sanitize::NONE.filter(true);
    assert_eq!(
        Sanitize::resolve(Some(field), Some(Sanitize::UPPER), Sanitize::NONE),
        field
    );
}

#[test]
fn resolve_override_beats_record() {
    assert_eq!(
        Sanitize::resolve(None, Some(Sanitize::NONE), Sanitize::UPPER),
        Sanitize::NONE
    );
}

#[test]
fn resolve_falls_back_to_record() {
    assert_eq!(
        Sanitize::resolve(None, None, Sanitize::UPPER),
        Sanitize::UPPER
    );
}

#[test]
fn filter() {
    let s = Sanitize::NONE.filter(true);
    assert_eq!(s.apply("he\0llo\té", 10), "hello");
}

#[test]
fn truncate() {
    let s = Sanitize::NONE.truncate(true);
    assert_eq!(s.apply("toolongstring", 5), "toolo");
    assert_eq!(s.apply("short", 5), "short");
}

#[test]
fn upper() {
    assert_eq!(Sanitize::UPPER.apply("hello", 10), "HELLO");
}

#[test]
fn combined() {
    let s = Sanitize::UPPER.truncate(true);
    assert_eq!(s.apply("longstring", 5), "LONGS");
}

#[test]
fn filter_runs_before_truncate() {
    let s = Sanitize::NONE.filter(true).truncate(true);
    assert_eq!(s.apply("\0\0abcdef", 3), "abc");
}

#[test]
fn unchanged_value_is_borrowed() {
    let s = Sanitize::UPPER.filter(true).truncate(true);
    assert!(matches!(s.apply("ABC", 5), std::borrow::Cow::Borrowed(_)));
}

#[test]
fn applied_when_encoding_a_field() {
    let f = FieldSpec::alpha("a", 0, 5);
    let s = Sanitize::UPPER.truncate(true);
    assert_eq!(pack_with(&f, &"longstring".to_owned(), s).unwrap(), "LONGS");
}

#[test]
fn not_applied_to_constants() {
    let f = FieldSpec::alpha("a", 0, 3).with_constant_str("ab");
    assert_eq!(pack_with(&f, &bryl::Const, Sanitize::UPPER).unwrap(), "ab ");
}
