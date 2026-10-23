# Hearthsound TheAudioDB plugin

A [Hearthsound](https://github.com/hearthsound) metadata plugin backed by
[TheAudioDB](https://www.theaudiodb.com/)'s public v1 JSON API. It compiles to
a WebAssembly component that implements the `metadata` provider interface, so a
Hearthsound server can show a biography and reference links for an artist.

The plugin needs no account and no paid key. The public test key `123` is enough
to evaluate it; a premium key raises the request limit and the size of a search
result page.

## Requirements

- Rust 1.99 or newer, with the `wasm32-wasip2` target. The included
  `rust-toolchain.toml` pins both; `rustup toolchain install` fetches them.

## Build

```console
cargo build --release --locked --target wasm32-wasip2
```

The component lands at
`target/wasm32-wasip2/release/hearthsound_plugin_theaudiodb.wasm`.

## Check and pack

Install the plugin CLI from the SDK repository:

```console
cargo install --locked --git https://github.com/hearthsound/plugin-sdk --tag v0.3.0 hearthsound-plugin-cli
```

Then validate the manifest and the component:

```console
hsplugin check
```

It prints `ok: app.hearthsound.theaudiodb 0.1.0 (provides metadata)` on
success.

Finally, build the archive you install on a server:

```console
hsplugin pack --out-dir dist
```

`dist/app.hearthsound.theaudiodb-0.1.0.hsplugin` contains the manifest and the
component.

## Tests

The response mapping and the lookup policy run on the host target, so their
unit tests need no WebAssembly toolchain and no network. They run against JSON
fixtures captured from the public API:

```console
cargo test
```

## Configuration

| Instance setting | Default | Meaning |
|---|---|---|
| `apiKey` | `123` | TheAudioDB API key, sent as a path segment. The public test key works for evaluation; a premium key raises the rate limit and returns more search results. |
| `baseUrl` | `https://www.theaudiodb.com/api/v1/json/` | Base URL of the v1 JSON API. Point it at a mirror or a test double to keep traffic off theaudiodb.com. |

The instance requests a `host-http` rate limit of one request per second with a
burst of one, which stays inside the 30 requests per minute the free key
allows.

## Notes

- The API key travels in the URL path, not in a header. The plugin encodes it
  and every query value before building a request.
- `get-artist-info` reads the artist the server's `mb-artist` id names; when
  that lookup finds nothing, or the server holds no id, it searches the artist
  name instead. A search hit counts only when its primary or alternate name
  spells the requested name the same way, ignoring case and whitespace, so a
  namesake or a tribute act is never shown as the artist.
- The biography is the English column. The links come from the row's reference
  columns: `strWebsite` as `homepage`, `strWikipediaID` as `wikipedia`,
  `strWikidataID` as `wikidata`, `strLastFMChart` as `lastfm`,
  `strDiscogsID` as `discogs` and `strAllMusicID` as `allmusic`. Facebook and
  Twitter have no matching link kind and are not reported.
- TheAudioDB is not a recording catalog: `resolve-recording` answers no matches
  and `get-release` answers not-found, so the server moves on to the next
  metadata instance in the layer order.
- An unknown artist is an HTTP 200 response whose collection is `null`, not an
  error. The plugin answers "no artist" for it, and translates a 404 (an
  unrecognized key or an endpoint the shared service does not serve), a 429
  (rate limited, with the `Retry-After` delay when present) and a 503 (shared
  service unavailable) into the provider errors that describe them.

## Layout

| Path | Contents |
|---|---|
| `Cargo.toml` | Crate metadata and the SDK dependency. |
| `src/lib.rs` | The plugin: configuration, lifecycle and the metadata interface. |
| `src/api.rs` | The HTTP transport, request paths and TheAudioDB's error statuses. |
| `src/audiodb.rs` | Serde types for TheAudioDB's v1 JSON responses. |
| `src/map.rs` | TheAudioDB rows to provider records: biography, identifiers and links. |
| `src/metadata.rs` | The metadata operations: artist lookup with the name-search fallback. |
| `src/url.rs` | Base URL validation, endpoint composition and query encoding. |
| `manifest.json` | The plugin manifest the server reads. |

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
