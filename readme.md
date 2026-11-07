# `base-message`

The header and trailer field sections of protocol messages, interpreted into typed structs for Rust.

HTTP, email, and MIME parts share one message structure: sections of named fields around the content. Each `FieldType` says how its instances combine, as a singleton, a list, or separate values, and how one value parses. `#[derive(FieldSection)]` generates the interpretation of a whole section: duplicate singletons are errors, lists respect quoted commas, forbidden fields are refused, and undeclared fields are ignored or passed on to the next layer. Name dispatch is generated at compile time, and a protocol's static table, such as HPACK's, can identify fields without comparing names at all.

## Usage

```rust
use base_message::{FieldSection, FieldType, FieldValueError, ascii_text, combination::Singleton};

struct Host(String);

impl FieldType for Host {
    type Combination = Singleton;
    fn parse(value: &[u8]) -> Result<Self, FieldValueError> {
        Ok(Self(ascii_text(value)?.to_owned()))
    }
}

#[derive(FieldSection)]
#[fields(forbid("transfer-encoding"))]
struct Request {
    #[field("host")]
    host: Host,
}

let request = Request::interpret([("Host", &b"example.com"[..]), ("User-Agent", b"curl/8")]).unwrap();
assert_eq!(request.host.0, "example.com");
```

See the guides:

- [Getting Started](context/getting-started.md): field types, sections, and errors.
- [Layering](context/layering.md): passing fields between layers, composing sections, and static tables.

## Releases

See [releases.md](releases.md).

## See Also

- [Protocol::Message](https://github.com/socketry/protocol-message), the Ruby gem whose policy-driven metadata this crate follows.
- [RFC 9110 section 6](https://www.rfc-editor.org/rfc/rfc9110#section-6), the HTTP message abstraction, and [RFC 5322](https://www.rfc-editor.org/rfc/rfc5322), the Internet Message Format.

## Contributing

Please open an issue or pull request on [GitHub](https://github.com/base-rs/base-message).

Install the [Bake launcher](https://github.com/base-rs/bake) and run `cargo bake test`. Use `cargo bake test:coverage --all-targets true` for the shared source-region coverage gate.

### Agent Context

Run `cargo bake agent:context:install` to install shared context and skills. Use `cargo bake agent:context:list` and `cargo bake agent:context:show` to discover relevant guides, follow repository guidance in `.agents/`, and apply skills under `.agents/skills/`.

Before planning or changing behavior, discover and read applicable `design/*.md`. Use `base-design-review` to assess plans and changes, and `base-design-author` when user requirements are introduced, changed, or removed. Review design compliance before opening or updating a pull request.
