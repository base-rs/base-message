// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use base_message::combination::{List, Separate, Singleton};
use base_message::{
    FieldErrorKind, FieldName, FieldRef, FieldSection, FieldType, FieldValueError, InvalidValue,
    OwnedFields, StaticTableEntry, ascii_text, static_table_entries,
};
use base_web_media_type::{MediaRange, MediaType};

// Field types: each says how instances combine and how one value parses.

#[derive(Debug, PartialEq)]
struct ContentType(MediaType);

impl FieldType for ContentType {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        MediaType::parse(ascii_text(value)?)
            .map(Self)
            .map_err(|_| FieldValueError::new("invalid media type"))
    }
}

#[derive(Debug, PartialEq)]
struct ContentDisposition {
    disposition: String,
    name: Option<String>,
}

impl FieldType for ContentDisposition {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        // A deliberately small parser for the prototype: `type; name="..."`.
        let text = ascii_text(value)?;
        let mut parts = text.split(';').map(str::trim);
        let disposition = parts
            .next()
            .filter(|part| !part.is_empty())
            .ok_or(FieldValueError::new("missing disposition type"))?
            .to_ascii_lowercase();
        let name = parts
            .find_map(|part| part.strip_prefix("name="))
            .map(|name| name.trim_matches('"').to_owned());
        Ok(Self { disposition, name })
    }
}

/// A field type for any text value, standing in for the many simple ones a protocol defines.
#[derive(Debug, PartialEq)]
struct Text(String);

impl FieldType for Text {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

#[derive(Debug, PartialEq)]
struct ContentLength(u64);

impl FieldType for ContentLength {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        let text = ascii_text(value)?;
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(FieldValueError::new("not a decimal length"));
        }
        text.parse()
            .map(Self)
            .map_err(|_| FieldValueError::new("length too large"))
    }
}

#[derive(Debug, PartialEq)]
struct Accept(MediaRange);

impl FieldType for Accept {
    type Combination = List;
    // Accept is advisory: an element that does not parse is skipped rather than failing the request.
    const INVALID: InvalidValue = InvalidValue::Ignore;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        MediaRange::parse(ascii_text(value)?)
            .map(Self)
            .map_err(|_| FieldValueError::new("invalid media range"))
    }
}

#[derive(Debug, PartialEq)]
struct SetCookie(String);

impl FieldType for SetCookie {
    type Combination = Separate;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

// Sections: the declarations a reviewer reads.

/// Multipart part headers: two fields declared, and an encoding this parser does not support forbidden.
#[derive(Debug, FieldSection)]
#[fields(forbid("content-transfer-encoding"))]
struct PartHeaders {
    /// The part's media type; other attributes, such as this documentation, are left alone.
    #[field("content-type")]
    content_type: Option<ContentType>,
    #[field("content-disposition")]
    content_disposition: ContentDisposition,
}

/// What an HTTP/1.1 transport acts on itself, strictly: framing and the target host.
#[derive(Debug, FieldSection)]
#[fields(forbid("transfer-encoding"))]
struct TransportHeaders {
    #[field("host")]
    host: Text,
    #[field("content-length")]
    content_length: Option<ContentLength>,
}

/// What an application declares for itself.
#[derive(Debug, FieldSection)]
struct ApplicationHeaders {
    #[field("accept")]
    accept: Vec<Accept>,
    #[field("set-cookie")]
    set_cookie: Vec<SetCookie>,
    #[field("x-request-id")]
    request_id: Option<Text>,
}

/// One section composed from another, as an application might declare everything it reads in one place.
#[derive(Debug, FieldSection)]
struct AllHeaders {
    #[field("x-request-id")]
    request_id: Option<Text>,
    #[fields(flatten)]
    transport: TransportHeaders,
    #[field("accept")]
    accept: Vec<Accept>,
}

#[derive(Debug, FieldSection)]
#[fields(unknown = "reject")]
struct OnlyHost {
    #[field("host")]
    host: Option<Text>,
}

fn pairs<'a>(fields: &'a [(&'a str, &'a str)]) -> impl Iterator<Item = (&'a str, &'a [u8])> {
    fields.iter().map(|(name, value)| (*name, value.as_bytes()))
}

#[test]
fn interprets_part_headers() {
    let headers = PartHeaders::interpret(pairs(&[
        ("Content-Disposition", "form-data; name=\"avatar\""),
        ("X-Trace", "1"),
        ("CONTENT-TYPE", " image/png "),
    ]))
    .unwrap();
    assert_eq!(
        headers.content_type,
        Some(ContentType(MediaType::IMAGE_PNG))
    );
    assert_eq!(
        headers.content_disposition,
        ContentDisposition {
            disposition: "form-data".into(),
            name: Some("avatar".into())
        }
    );
}

#[test]
fn rejects_duplicate_singletons_with_both_positions() {
    let error = PartHeaders::interpret(pairs(&[
        ("Content-Type", "text/plain"),
        ("Content-Disposition", "form-data"),
        ("content-type", "text/html"),
    ]))
    .unwrap_err();
    assert_eq!(
        (error.name(), error.position(), error.kind()),
        (
            Some("content-type"),
            2,
            FieldErrorKind::Duplicate { first: 0 }
        )
    );
    assert_eq!(
        error.to_string(),
        "duplicate content-type field at position 2, first at 0"
    );
    let error = TransportHeaders::interpret(pairs(&[
        ("Host", "h"),
        ("Content-Length", "1"),
        ("Content-Length", "1"),
    ]))
    .unwrap_err();
    assert_eq!(
        error.kind(),
        FieldErrorKind::Duplicate { first: 1 },
        "even identical duplicates are rejected"
    );
}

#[test]
fn reports_missing_and_invalid_fields_without_their_values() {
    let error = PartHeaders::interpret(pairs(&[("Content-Type", "text/plain")])).unwrap_err();
    assert_eq!(
        (error.name(), error.position(), error.kind()),
        (Some("content-disposition"), 1, FieldErrorKind::Missing)
    );
    let error = PartHeaders::interpret(pairs(&[
        ("Content-Disposition", "form-data"),
        ("Content-Type", "not a type"),
    ]))
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid content-type field at position 1: invalid media type"
    );
    let error = TransportHeaders::interpret(pairs(&[("Host", "h"), ("Content-Length", "12, 12")]))
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid content-length field at position 1: not a decimal length"
    );
}

#[test]
fn combines_lists_but_keeps_separate_values_apart() {
    let headers = ApplicationHeaders::interpret(pairs(&[
        ("Accept", "text/html, , application/json;q=\"0,9\""),
        ("Set-Cookie", "a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT"),
        ("Accept", "image/*, not a range"),
        ("Set-Cookie", "b=2"),
    ]))
    .unwrap();
    let accept: Vec<String> = headers
        .accept
        .iter()
        .map(|accept| accept.0.essence().to_owned())
        .collect();
    assert_eq!(
        accept,
        ["text/html", "application/json", "image/*"],
        "a comma inside quotes does not split, empty elements are skipped, and Accept ignores invalid elements"
    );
    assert_eq!(headers.accept[1].0.parameter("q"), Some("0,9"));
    assert_eq!(
        headers.set_cookie,
        [
            SetCookie("a=1; Expires=Wed, 21 Oct 2026 07:28:00 GMT".into()),
            SetCookie("b=2".into())
        ],
        "commas in a separate value are data"
    );
}

#[test]
fn ignores_undeclared_fields_by_default_and_rejects_them_when_declared_to() {
    let headers =
        ApplicationHeaders::interpret(pairs(&[("X-Anything", "x"), ("X-Request-Id", "abc")]))
            .unwrap();
    assert_eq!(headers.request_id, Some(Text("abc".into())));
    assert_eq!(
        OnlyHost::interpret(pairs(&[("Host", "h")])).unwrap().host,
        Some(Text("h".into()))
    );
    let error = OnlyHost::interpret(pairs(&[("Host", "h"), ("Evil", "x")])).unwrap_err();
    assert_eq!(
        (error.kind(), error.position(), error.name()),
        (FieldErrorKind::Unknown, 1, None),
        "an unknown name is untrusted, so it is not repeated"
    );
}

#[test]
fn layers_pass_on_what_they_do_not_take() {
    let request = [
        ("Host", "example.com"),
        ("Accept", "text/html"),
        ("Content-Length", "5"),
        ("X-Request-Id", "abc"),
        ("Set-Cookie", "a=1"),
        ("User-Agent", "curl/8"),
    ];
    let (transport, rest): (TransportHeaders, Vec<FieldRef<'_>>) =
        TransportHeaders::partition(pairs(&request)).unwrap();
    assert_eq!(
        (transport.host, transport.content_length),
        (Text("example.com".into()), Some(ContentLength(5)))
    );
    let positions: Vec<usize> = rest.iter().map(|field| field.position).collect();
    assert_eq!(
        positions,
        [1, 3, 4, 5],
        "the remainder keeps original positions"
    );
    let (application, rest): (ApplicationHeaders, Vec<FieldRef<'_>>) =
        ApplicationHeaders::partition(rest).unwrap();
    assert_eq!(
        (application.accept.len(), application.set_cookie.len()),
        (1, 1)
    );
    assert_eq!(
        rest,
        [FieldRef {
            position: 5,
            name: FieldName::Text("User-Agent"),
            value: b"curl/8"
        }],
        "only what neither layer declares is left"
    );
    // Errors in a later layer cite the field's position in the original section.
    let request = [("Host", "h"), ("X-Request-Id", "a"), ("X-Request-Id", "b")];
    let (_, rest): (TransportHeaders, Vec<FieldRef<'_>>) =
        TransportHeaders::partition(pairs(&request)).unwrap();
    let error = ApplicationHeaders::interpret(rest).unwrap_err();
    assert_eq!(
        (error.position(), error.kind()),
        (2, FieldErrorKind::Duplicate { first: 1 })
    );
}

#[test]
fn remainders_can_be_owned() {
    let request = [("Host", "h"), ("X-Trace", "1"), ("X-Trace", "2")];
    let (_, rest): (TransportHeaders, OwnedFields) =
        TransportHeaders::partition(pairs(&request)).unwrap();
    assert_eq!(rest.values("x-trace").collect::<Vec<_>>(), [b"1", b"2"]);
    assert_eq!(
        rest.iter()
            .map(|field| field.position())
            .collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn forbidden_fields_are_errors_even_when_unknown_fields_are_ignored() {
    let error = PartHeaders::interpret(pairs(&[
        ("Content-Disposition", "form-data"),
        ("Content-Transfer-Encoding", "base64"),
    ]))
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "forbidden content-transfer-encoding field at position 1"
    );
    // A forbidden field is never passed on to a later layer.
    let result = TransportHeaders::partition::<_, Vec<FieldRef<'_>>>(pairs(&[
        ("Host", "h"),
        ("Transfer-Encoding", "chunked"),
    ]));
    assert_eq!(result.unwrap_err().kind(), FieldErrorKind::Forbidden);
    // Forbidding applies through flattening too.
    let error = AllHeaders::interpret(pairs(&[("Host", "h"), ("TRANSFER-ENCODING", "chunked")]))
        .unwrap_err();
    assert_eq!(
        (error.name(), error.position()),
        (Some("transfer-encoding"), 1)
    );
}

/// A stand-in for a protocol's static table, such as HPACK's, which lists names by index.
const MOCK_TABLE: [&str; 6] = [
    ":authority",
    "accept",
    "content-length",
    "x-request-id",
    "transfer-encoding",
    "user-agent",
];

#[test]
fn numbers_declarations_across_flattened_sections() {
    let names = &AllHeaders::NAMES;
    assert_eq!(names.count(), 4);
    // Its own declarations come first, then the flattened section's, in order.
    assert_eq!(
        (
            names.declaration("x-request-id"),
            names.declaration("accept")
        ),
        (Some(0), Some(1))
    );
    assert_eq!(
        (
            names.declaration("host"),
            names.declaration("content-length")
        ),
        (Some(2), Some(3))
    );
    assert!(names.forbids("transfer-encoding") && names.declaration("transfer-encoding").is_none());
    const TABLE: [StaticTableEntry; 6] = static_table_entries(&AllHeaders::NAMES, &MOCK_TABLE);
    assert_eq!(
        TABLE,
        [
            StaticTableEntry::Undeclared,
            StaticTableEntry::Declaration(1),
            StaticTableEntry::Declaration(3),
            StaticTableEntry::Declaration(0),
            StaticTableEntry::Forbidden,
            StaticTableEntry::Undeclared
        ]
    );
}

#[test]
fn interprets_fields_identified_by_a_static_table() {
    const TABLE: [StaticTableEntry; 6] = static_table_entries(&AllHeaders::NAMES, &MOCK_TABLE);
    // A decoder resolves static indexes through the table, and passes literal names as text.
    let indexed = |index: usize, value: &'static str| {
        (TABLE[index].name(MOCK_TABLE[index]), value.as_bytes())
    };
    let fields = [
        (FieldName::Text("host"), &b"example.com"[..]),
        indexed(1, "text/html, image/*"),
        indexed(3, "abc"),
        indexed(5, "curl/8"),
        indexed(2, "7"),
    ];
    let (headers, rest): (AllHeaders, Vec<FieldRef<'_>>) = AllHeaders::partition(fields).unwrap();
    assert_eq!(
        (
            headers.transport.host.0.as_str(),
            headers
                .transport
                .content_length
                .as_ref()
                .map(|length| length.0)
        ),
        ("example.com", Some(7))
    );
    assert_eq!(
        (
            headers.accept.len(),
            headers.request_id.as_ref().map(|id| id.0.as_str())
        ),
        (2, Some("abc"))
    );
    // An entry the table found undeclared for this section reaches the next one as text, since that section may declare it.
    assert_eq!(
        rest,
        [FieldRef {
            position: 3,
            name: FieldName::Text("user-agent"),
            value: b"curl/8"
        }]
    );
    // Forbidden and duplicate fields are caught by index as they are by name.
    let error =
        AllHeaders::interpret([(FieldName::Text("host"), &b"h"[..]), indexed(4, "chunked")])
            .unwrap_err();
    assert_eq!(
        (error.kind(), error.name(), error.position()),
        (FieldErrorKind::Forbidden, Some("transfer-encoding"), 1)
    );
    let error = AllHeaders::interpret([
        (FieldName::Text("host"), &b"h"[..]),
        indexed(3, "a"),
        indexed(3, "b"),
    ])
    .unwrap_err();
    assert_eq!(error.kind(), FieldErrorKind::Duplicate { first: 1 });
}

#[derive(Debug, PartialEq)]
struct Strict(String);

impl FieldType for Strict {
    type Combination = List;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

/// Declarations that override their field types' invalid-value policies.
#[derive(Debug, FieldSection)]
#[fields(unknown = "ignore", forbid("upgrade", "te"))]
struct Overrides {
    #[field("accept", invalid = "error")]
    accept: Vec<Accept>,
    #[field("x-list", invalid = "ignore")]
    list: Vec<Strict>,
    #[field("content-length", invalid = "ignore")]
    content_length: Option<ContentLength>,
    #[field("set-cookie", invalid = "ignore")]
    set_cookie: Vec<SetCookie>,
}

#[test]
fn declarations_can_override_invalid_value_policies() {
    let error = Overrides::interpret(pairs(&[("Accept", "not a range")])).unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid accept field at position 0: invalid media range"
    );
    let error = Overrides::interpret(pairs(&[("Accept", "\"unterminated")])).unwrap_err();
    assert_eq!(
        error.kind(),
        FieldErrorKind::Invalid(FieldValueError::new("unterminated quoted string"))
    );
    let headers = Overrides::interpret([
        ("X-List", &b"\"unterminated, a"[..]),
        ("X-List", b"b"),
        ("Content-Length", b"x"),
        ("Set-Cookie", b"caf\xc3\xa9"),
        ("Set-Cookie", b"a=1"),
    ])
    .unwrap();
    assert_eq!(
        headers.list,
        [Strict("b".into())],
        "an unterminated list is skipped as a whole"
    );
    assert!(headers.accept.is_empty());
    assert_eq!(headers.content_length, None);
    assert_eq!(headers.set_cookie, [SetCookie("a=1".into())]);
    let error = Overrides::interpret(pairs(&[("Content-Length", "x"), ("Content-Length", "1")]))
        .unwrap_err();
    assert_eq!(
        error.kind(),
        FieldErrorKind::Duplicate { first: 0 },
        "an ignored value still counts as present"
    );
    let error = ApplicationHeaders::interpret([("Set-Cookie", &b"caf\xc3\xa9"[..])]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "invalid set-cookie field at position 0: value is not visible ASCII"
    );
    for forbidden in ["Upgrade", "TE"] {
        assert_eq!(
            Overrides::interpret(pairs(&[(forbidden, "x")]))
                .unwrap_err()
                .kind(),
            FieldErrorKind::Forbidden
        );
    }
}

#[test]
fn rejects_unknown_fields_classified_by_a_static_table() {
    let error =
        OnlyHost::interpret([(FieldName::Undeclared("user-agent"), &b"x"[..])]).unwrap_err();
    assert_eq!(error.kind(), FieldErrorKind::Unknown);
    // An empty name is never declared.
    assert!(OnlyHost::interpret([("", &b"x"[..])]).is_err());
    assert!(ApplicationHeaders::interpret([("", &b"x"[..])]).is_ok());
}

#[test]
fn rejects_classifications_made_for_another_section() {
    let error = AllHeaders::interpret([(FieldName::Declaration(9, "x"), &b"v"[..])]).unwrap_err();
    assert_eq!(
        (error.kind(), error.name(), error.position()),
        (FieldErrorKind::Declaration, None, 0)
    );
    assert_eq!(
        error.to_string(),
        "field at position 0 has a declaration id this section does not have"
    );
    let error = PartHeaders::interpret([(FieldName::Declaration(2, "x"), &b"v"[..])]).unwrap_err();
    assert_eq!(error.kind(), FieldErrorKind::Declaration);
    let error = AllHeaders::interpret([(FieldName::Forbidden("upgrade"), &b"v"[..])]).unwrap_err();
    assert_eq!(
        error.kind(),
        FieldErrorKind::Declaration,
        "the section does not forbid `upgrade`, so the table was not built for it"
    );
}

#[test]
fn flattened_sections_report_their_own_missing_fields() {
    let error = AllHeaders::interpret(pairs(&[("Accept", "*/*")])).unwrap_err();
    assert_eq!(
        (error.kind(), error.name(), error.position()),
        (FieldErrorKind::Missing, Some("host"), 1)
    );
    assert_eq!(error.to_string(), "missing required host field");
}

#[test]
fn errors_describe_every_kind_and_expose_value_errors() {
    use std::error::Error as _;
    let error =
        TransportHeaders::interpret(pairs(&[("Host", "h"), ("Content-Length", "x")])).unwrap_err();
    assert_eq!(
        error.source().map(ToString::to_string),
        Some("not a decimal length".to_owned())
    );
    let error = OnlyHost::interpret(pairs(&[("X", "y")])).unwrap_err();
    assert_eq!(
        (error.to_string(), error.source().is_none()),
        ("unexpected field at position 0".to_owned(), true)
    );
}

#[test]
fn owned_remainders_can_be_interpreted_again() {
    let request = [("Host", "h"), ("Accept", "text/html"), ("X-Trace", "1")];
    let (_, rest): (TransportHeaders, OwnedFields) =
        TransportHeaders::partition(pairs(&request)).unwrap();
    assert_eq!((rest.len(), rest.is_empty()), (2, false));
    let first = rest.iter().next().unwrap();
    assert_eq!(
        (first.position(), first.name(), first.value()),
        (1, "Accept", &b"text/html"[..])
    );
    assert_eq!((&rest).into_iter().count(), 2);
    let application = ApplicationHeaders::interpret(rest.as_field_refs()).unwrap();
    assert_eq!(application.accept.len(), 1);
    assert!(OwnedFields::default().is_empty());
    let names = [
        FieldName::Text("a"),
        FieldName::Declaration(0, "b"),
        FieldName::Forbidden("c"),
        FieldName::Undeclared("d"),
    ];
    assert_eq!(names.map(|name| name.as_str()), ["a", "b", "c", "d"]);
}
