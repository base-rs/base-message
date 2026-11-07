# Layering

This guide explains how sections at different layers share one message's fields, how sections compose, and how a decoder with a static table interprets fields without comparing names.

## Each layer takes what it understands

A transport acts on a few fields itself, such as an HTTP server's `Content-Length`, `Transfer-Encoding`, and `Host`, and interprets them strictly, because that is where request smuggling happens. An application declares the fields it reads. Neither should see the other's policy.

`partition` interprets a section and returns the fields it does not take, for the next layer:

```rust
use base_message::{FieldRef, FieldSection, FieldType, FieldValueError, ascii_text, combination::{List, Singleton}};

struct Text(String);

impl FieldType for Text {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

struct Accept(String);

impl FieldType for Accept {
    type Combination = List;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

#[derive(FieldSection)]
#[fields(forbid("transfer-encoding"))]
struct Transport {
    #[field("host")]
    host: Text,
}

#[derive(FieldSection)]
struct Application {
    #[field("accept")]
    accept: Vec<Accept>,
    #[field("x-request-id")]
    request_id: Option<Text>,
}

let fields = [("Host", &b"example.com"[..]), ("Accept", b"text/html"), ("X-Request-Id", b"abc"), ("User-Agent", b"curl/8")];
let (transport, rest): (Transport, Vec<FieldRef<'_>>) = Transport::partition(fields).unwrap();
let (application, rest): (Application, Vec<FieldRef<'_>>) = Application::partition(rest).unwrap();
assert_eq!(transport.host.0, "example.com");
assert_eq!(application.request_id.map(|id| id.0).as_deref(), Some("abc"));

// Only what neither layer declares is left, at its original position.
assert_eq!((rest.len(), rest[0].position, rest[0].name.as_str()), (1, 3, "User-Agent"));
```

- The remainder borrows names and values from the input, so a `Vec<FieldRef>` copies nothing. Collect into `OwnedFields` instead to keep the fields beyond the input's lifetime.
- Each remaining field keeps its position in the original section, so a later layer's errors cite the position a person reading the message would see.
- A forbidden field is an error, never part of the remainder, so a later layer is never handed a field an earlier one refused.
- `interpret` is `partition` with the remainder discarded. A section with `unknown = "reject"` has no remainder.

Declare everything a layer reads in its section, and let the rest fall through: a field no section declares costs a failed name check at each layer and is never parsed or stored.

## Composing sections

`#[fields(flatten)]` embeds one section in another, so an application can declare the standard fields of a library's section alongside its own in one struct:

```rust
use base_message::{FieldSection, FieldType, FieldValueError, ascii_text, combination::Singleton};

struct Text(String);

impl FieldType for Text {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

#[derive(FieldSection)]
struct Standard {
    #[field("host")]
    host: Option<Text>,
}

#[derive(FieldSection)]
struct Everything {
    #[fields(flatten)]
    standard: Standard,
    #[field("x-request-id")]
    request_id: Option<Text>,
}

let everything = Everything::interpret([("Host", &b"example.com"[..]), ("X-Request-Id", b"abc")]).unwrap();
assert_eq!(everything.standard.host.map(|host| host.0).as_deref(), Some("example.com"));
```

A name declared or forbidden both by a section and by a section it flattens is a compile-time error, because the two declarations would disagree about the field and one would silently never receive values. Flattening composes sections; it does not override them. To interpret a field differently from a library's section, declare it in a section of your own and interpret the fields with that.

## Trailers

A trailer section is a section of its own. Interpret the trailer fields with their own struct, never by adding them to the header fields: RFC 9110 section 6.5.1 forbids merging a trailer field into the header section unless the field's definition says how, so that a trailer cannot quietly change metadata a recipient has already acted on. A protocol crate supplies the fields its trailers must not carry as a section that declares nothing and only forbids, and an application flattens it into each trailer section it declares:

```rust
use base_message::{FieldErrorKind, FieldSection, FieldType, FieldValueError, ascii_text, combination::Singleton};

struct Text(String);

impl FieldType for Text {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

/// What a protocol crate would provide: the fields that must not appear in trailers.
#[derive(FieldSection)]
#[fields(forbid("content-length", "transfer-encoding", "host", "authorization"))]
struct TrailerRules {}

#[derive(FieldSection)]
struct Headers {
    #[field("etag")]
    etag: Option<Text>,
}

#[derive(FieldSection)]
struct Trailers {
    #[fields(flatten)]
    _rules: TrailerRules,
    #[field("etag")]
    etag: Option<Text>,
    #[field("x-checksum")]
    checksum: Option<Text>,
}

let headers = Headers::interpret([("ETag", &b"\"v1\""[..])]).unwrap();
let trailers = Trailers::interpret([("X-Checksum", &b"abc"[..]), ("ETag", b"\"v2\""), ("X-Unknown", b"1")]).unwrap();
assert_eq!(trailers.checksum.map(|checksum| checksum.0).as_deref(), Some("abc"));

// The sections stay apart, so which ETag to believe is the application's decision.
assert_eq!(headers.etag.map(|etag| etag.0).as_deref(), Some("\"v1\""));
assert_eq!(trailers.etag.map(|etag| etag.0).as_deref(), Some("\"v2\""));

let error = Trailers::interpret([("X-Checksum", &b"abc"[..]), ("Content-Length", b"5")]).err().unwrap();
assert_eq!((error.kind(), error.position()), (FieldErrorKind::Forbidden, 1));
```

- Because nothing is merged, a trailer field no section declares is ignored, as in a header section, and no layer ever acts on it. Forbidding a field makes it an error instead, for fields whose presence in trailers signals a broken or hostile peer.
- Declaring a field the flattened rules forbid, such as `content-length`, is a compile-time error, so an application cannot read one from trailers by mistake.
- Positions count from the start of the trailer section, and the caller knows which section an error came from, because it chose the section to interpret.

## Static tables

HTTP/2's HPACK and HTTP/3's QPACK identify common fields by an index into a static table, and the two tables differ. A decoder can classify each table entry for a section once, at compile time, and then interpret indexed fields by declaration id, without comparing names:

```rust
use base_message::{FieldName, FieldSection, FieldType, FieldValueError, StaticTableEntry, ascii_text, combination::Singleton, static_table_entries};

struct Text(String);

impl FieldType for Text {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

#[derive(FieldSection)]
#[fields(forbid("transfer-encoding"))]
struct Request {
    #[field("host")]
    host: Option<Text>,
}

// A stand-in for a protocol's static table of names.
const NAMES: [&str; 3] = [":authority", "host", "transfer-encoding"];
const TABLE: [StaticTableEntry; 3] = static_table_entries(&Request::NAMES, &NAMES);

// A decoder passes indexed fields with the table's classification, and literal names as text.
let fields = [(TABLE[1].name(NAMES[1]), &b"example.com"[..]), (FieldName::Text("user-agent"), b"curl/8")];
let request = Request::interpret(fields).unwrap();
assert_eq!(request.host.map(|host| host.0).as_deref(), Some("example.com"));

let refused = Request::interpret([(TABLE[2].name(NAMES[2]), &b"chunked"[..])]);
assert!(refused.is_err(), "a forbidden field is refused by index too");
```

The table is a constant for each section type: 62 entries for HPACK and 99 for QPACK. A classification belongs to the section it was made for. A field classified as undeclared reaches a later layer's remainder as text, since that layer may declare it, and a declaration id the section does not have is an error rather than a silent misreading.
