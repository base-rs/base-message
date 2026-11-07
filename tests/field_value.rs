// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_message::{FieldValueError, ascii_text, split_list, trim_whitespace};

#[test]
fn splits_lists_respecting_quoted_strings() {
    let cases: [(&[u8], &[&[u8]]); 6] = [
        (b"a, b,c", &[b"a", b"b", b"c"]),
        (b" , a , , ", &[b"a"]),
        (b"a;q=\"1,2\", b", &[b"a;q=\"1,2\"", b"b"]),
        (b"\"a\\\",b\", c", &[b"\"a\\\",b\"", b"c"]),
        (b"", &[]),
        (b"x\\,y", &[b"x\\", b"y"]),
    ];
    for (value, elements) in cases {
        assert_eq!(
            split_list(value).collect::<Result<Vec<_>, _>>().unwrap(),
            elements,
            "{:?}",
            String::from_utf8_lossy(value)
        );
    }
    let elements: Vec<_> = split_list(b"a, \"b").collect();
    assert_eq!(
        elements,
        [
            Ok(&b"a"[..]),
            Err(FieldValueError::new("unterminated quoted string"))
        ],
        "elements before the error are produced, then the error, then nothing"
    );
    let mut elements = split_list(b"\"");
    assert!(
        elements.next().unwrap().is_err() && elements.next().is_none() && elements.next().is_none()
    );
    assert_eq!(split_list(b"a,b").clone().count(), 2);
    assert!(format!("{:?}", split_list(b"a")).contains("ListElements"));
}

#[test]
fn trims_whitespace_and_reads_ascii() {
    assert_eq!(trim_whitespace(b" \t a b \t"), b"a b");
    assert_eq!(trim_whitespace(b" \t "), b"");
    assert_eq!(ascii_text(b"text/html; q=1\t"), Ok("text/html; q=1\t"));
    for invalid in [&b"caf\xc3\xa9"[..], b"a\nb", b"\x7f", b"\xff"] {
        assert_eq!(
            ascii_text(invalid),
            Err(FieldValueError::new("value is not visible ASCII"))
        );
    }
    let error = FieldValueError::new("reason");
    assert_eq!(
        (error.reason(), error.to_string()),
        ("reason", "reason".to_owned())
    );
    let error: Box<dyn std::error::Error> = Box::new(error);
    assert!(error.source().is_none());
}
