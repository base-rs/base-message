# Getting Started

This guide explains how to define field types and declare field sections with `base-message`.

## Messages and field sections

HTTP (RFC 9110 section 6), email (RFC 5322), and MIME parts (RFC 2045) share one message structure: a header section and, for HTTP, a trailer section of named fields, around the content. A section's fields arrive in wire order as name and value pairs. Interpreting them is where implementations go wrong: taking the first or last of a repeated `Content-Length`, splitting `Accept` on a comma inside quotes, joining `Set-Cookie` values with commas, or silently ignoring a `Transfer-Encoding` that should have been refused.

`base-message` makes that interpretation a declaration. A field type says how one field combines and parses; a section says which fields it takes.

## Field types

A field type implements `FieldType`: its combination, how one value parses, and what an invalid value means.

```rust
use base_message::{FieldType, FieldValueError, InvalidValue, ascii_text, combination::{List, Separate, Singleton}};

/// A singleton: a second instance is an error, even with the same value.
pub struct ContentLength(pub u64);

impl FieldType<'_> for ContentLength {
    type Combination = Singleton;

    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        let text = ascii_text(value)?;
        if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(FieldValueError::new("not a decimal length"));
        }
        text.parse().map(Self).map_err(|_| FieldValueError::new("length too large"))
    }
}

/// A list: instances combine into one comma-separated list, and `parse` sees one element at a time.
pub struct Accept(pub String);

impl FieldType<'_> for Accept {
    type Combination = List;
    // An advisory field: an element that does not parse is skipped.
    const INVALID: InvalidValue = InvalidValue::Ignore;

    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

/// Separate values: never combined, because a value can contain a comma.
pub struct SetCookie(pub String);

impl FieldType<'_> for SetCookie {
    type Combination = Separate;

    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}
```

`parse` receives the value with surrounding spaces and tabs removed. For a list, it receives each element: commas inside quoted strings do not separate elements, and empty elements are skipped, as RFC 9110 section 5.6.1 describes. Comments, the parenthesized text some fields such as `Via` allow, are not recognized, so a field whose values can contain commas inside comments should be `Separate` and split its values itself. Each element becomes one value, so the memory a list field takes is bounded by the size of its section, which the protocol's parser limits. `split_list`, `trim_whitespace`, and `ascii_text` are available for field types with their own structure.

An invalid value is an error by default. A field type whose meaning matters for safety, such as `Content-Length`, should keep that default; an advisory one, such as `Accept`, can ignore invalid values.

The combinations are fixed: `Singleton`, `List`, and `Separate` are the only ones, and a field type cannot be both a singleton and a list.

### Borrowing values

The lifetime in `FieldType<'a>` is that of the field values. A field type that owns its value implements `FieldType<'_>`, as above. One that borrows from the value, such as a host name kept as a slice of the parse buffer, implements `FieldType<'a>` for its own lifetime, and a section that holds it has that lifetime too:

```rust
use base_message::{FieldSection, FieldType, FieldValueError, ascii_text, combination::Singleton};

struct Host<'a>(&'a str);

impl<'a> FieldType<'a> for Host<'a> {
    type Combination = Singleton;

    fn parse(value: &'a [u8]) -> Result<Self, FieldValueError> {
        ascii_text(value).map(Self)
    }
}

#[derive(FieldSection)]
struct Request<'a> {
    #[field("host")]
    host: Host<'a>,
}

let buffer = b"example.com".to_vec();
let request = Request::interpret([("Host", &buffer[..])]).unwrap();
assert_eq!(request.host.0, "example.com");
```

A section has at most one lifetime, and no other generic parameters. Borrowing avoids a copy and an allocation for each value; the section then cannot outlive the buffer the fields were parsed from.

## Sections

A section is a struct deriving `FieldSection`. Each field names the message field it takes, and its type says how instances combine:

| Declaration | Meaning |
| --- | --- |
| `Option<T>` | A singleton that may be absent. |
| `T` | A required singleton: absence is an error, and so is an invalid value, whatever the field type's policy. |
| `Vec<T>` | A list or separate field, every value in wire order. |

```rust
use base_message::{FieldErrorKind, FieldSection, FieldType, FieldValueError, ascii_text, combination::{List, Singleton}};

struct Host(String);

impl FieldType<'_> for Host {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

struct Accept(String);

impl FieldType<'_> for Accept {
    type Combination = List;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

#[derive(FieldSection)]
#[fields(forbid("transfer-encoding"))]
struct Request {
    #[field("host")]
    host: Host,
    #[field("accept")]
    accept: Vec<Accept>,
}

let fields = [("Host", &b"example.com"[..]), ("Accept", b"text/html, application/json;q=\"0,9\""), ("Accept", b"image/*"), ("User-Agent", b"curl/8")];
let request = Request::interpret(fields).unwrap();
assert_eq!(request.host.0, "example.com");
let accept: Vec<&str> = request.accept.iter().map(|accept| accept.0.as_str()).collect();
assert_eq!(accept, ["text/html", "application/json;q=\"0,9\"", "image/*"]);

let error = Request::interpret([("Host", &b"a"[..]), ("Transfer-Encoding", b"chunked")]).err().unwrap();
assert_eq!(error.kind(), FieldErrorKind::Forbidden);
assert_eq!(error.to_string(), "forbidden transfer-encoding field at position 1");
```

Names are declared once, in lowercase, and matched case-insensitively. Declaring a field the wrong way is a compile-time error that says what to write instead: a `Vec` of a singleton, an `Option` of a list, a name that is not a lowercase token, or two declarations on one member.

### Fields a section does not declare

`interpret` ignores fields a section does not declare, without parsing or storing them. On the struct:

- `#[fields(forbid("transfer-encoding", ...))]` makes those fields errors, even though the section does not declare them. Forbid fields that would be dangerous to ignore, such as framing fields a transport does not support, or fields not allowed in trailers.
- `#[fields(unknown = "reject")]` makes every field the section neither declares nor forbids an error, for sections where nothing else may appear. A section that flattens a rejecting section rejects unknown fields too, judged against all of its declarations, and cannot say `unknown = "ignore"`.

To keep undeclared fields for another layer, see the [Layering](layering.md) guide.

### Overriding a field type's policy

A declaration can tighten or relax its field type's invalid-value policy: `#[field("accept", invalid = "error")]`. A required declaration always treats an invalid value as an error, because ignoring the value of a field the section requires would leave nothing to report but a misleading absence; declaring one with `invalid = "ignore"` is a compile-time error. A section that needs a different policy for a field than another section uses declares the field itself, with its own declaration, rather than sharing one.

## Errors

Every error names the field, in the section's own lowercase spelling, and its position in the original section, except that a missing field has no position. Errors never include values, or the names of unknown fields, which may be large or untrusted:

```rust
use base_message::{FieldErrorKind, FieldSection, FieldType, FieldValueError, ascii_text, combination::Singleton};

struct ContentLength(u64);

impl FieldType<'_> for ContentLength {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        ascii_text(value)?.parse().map(Self).map_err(|_| FieldValueError::new("not a decimal length"))
    }
}

#[derive(FieldSection)]
struct Framing {
    #[field("content-length")]
    content_length: Option<ContentLength>,
}

let error = Framing::interpret([("Content-Length", &b"5"[..]), ("content-length", b"5")]).err().unwrap();
assert_eq!(error.kind(), FieldErrorKind::Duplicate { first: 0 });
assert_eq!(error.to_string(), "duplicate content-length field at position 1, first at 0");
```

A repeated singleton is an error even when the values are identical: deciding that two copies agree is exactly the kind of judgement that differs between implementations.
