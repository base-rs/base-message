# Scope

## Requirement

`base-message` must hold the field sections of the message abstraction that HTTP (RFC 9110 section 6), the Internet Message Format (RFC 5322), and MIME entities (RFC 2045) share: header and trailer sections of named fields, and their interpretation into typed structs. Parsing and writing a section's wire syntax belong to each protocol's own crate, as do control data, such as request lines, status codes, and HTTP/2 pseudo-headers, content, framing, and transport. Those crates pass the fields they parse to a section as name and value pairs in wire order, or with names classified by a static table, and must pass only names that are valid for their protocol, such as RFC 9110 tokens: a section matches names exactly, ignoring ASCII case, and treats any other name as undeclared. They also limit the size of each section, which bounds the memory interpretation takes.

The crate must not depend on an HTTP, web, or email crate; `base-http`, `base-web-multipart`, and future email crates depend on it. Protocol-specific rules stay with those crates: which fields are singletons or forbidden for a protocol is expressed in their field types and section declarations, and protocol static tables, such as HPACK's and QPACK's, are defined by the crates that decode them.

Interpretation takes fields as name and value pairs in wire order and never reorders, merges, or stores them beyond what a section declares. The ordered fields remain the source of truth, owned by the caller, so that any number of sections can interpret them. A trailer section is interpreted as a section of its own: the crate has no trailer marker and no operation that merges a trailer section's fields into a header section's. A protocol crate expresses the fields its trailers must not carry as a section that only forbids, for applications to flatten.

## Rationale

The user asked for a shared message layer, after the Ruby `protocol-message` gem, so that HTTP and multipart part headers, and later email, interpret fields the same way. Calling it `base-web-message` would make `base-http`, the transport, depend on the web content family, and putting it in `base-http` would make multipart depend on HTTP, so it is a standalone crate both depend on. The user chose the name `base-message` because RFC 9110 and RFC 5322 both name this structure the message; the scope is recorded here, and in `base-project`'s Package Organization guide, so that the generic name does not draw in bodies, framing, or transport. The user's goal was a standard way to map N fields to M ≤ N typed entries, because interpretation is where implementations go wrong. The user decided that field parsers belong in the protocol crates, because the wire syntaxes differ: HTTP/1.1 rejects folded lines while email unfolds them, MIME part headers are parsed strictly, and HTTP/2 and HTTP/3 decode fields from compressed binary frames. Each of those crates already owns its framing, limits, and static tables, so the shared contract is only the ordered pairs. The security audit before the first release confirmed that a malformed name, such as `Transfer-Encoding` with a trailing space, would be ignored rather than refused as forbidden; the user decided that validating name syntax stays with the parsers, which already examine every name, rather than repeating it for every field here. The user decided that trailers are never merged into the header section, as RFC 9110 section 6.5.1 requires unless a field's definition permits it, so that a trailer cannot quietly change metadata a recipient has already acted on; which of a header and a trailer value to believe is the application's decision.

## Scope

The `base-message` and `base-message-macros` crates as a whole.

## Validation

Inspect `cargo tree -p base-message -e normal`, which must list only `base-message-macros`, and the macro crate's dependencies, which must be only `proc-macro2`, `quote`, and `syn`. Review new public items against this scope.
