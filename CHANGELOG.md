# Changelog

## 0.3.0
> new features, and a RustBridge! The work done for this release will result in
> Rust Belt Rust 2017's RustBridge being taught with `simple-server`!

### Features

- An environment variable can be used to configure the number of threads in the pool. ([@gsquire]/[pull/79])
- The response body type is now `Cow<'a, [u8]>`, allowing you to return owned as well as borrowed data. ([@kardeiz]/[pull/74])

[@gsquire]: https://github.com/gsquire
[@kardeiz]: https://github.com/kardeiz
[pull/79]: https://github.com/steveklabnik/simple-server/pull/79
[pull/74]: https://github.com/steveklabnik/simple-server/pull/74

## 0.2.0
> the release that inevitably comes immediately after a first release 😅😅😅

### Features

- `ResponseBuilder` should have been public. Now it is! ([@steveklabnik]/[`9908f62`])
- Allow graceful termination of TCPConnection. ([@binarybana]/[pull/66])

[@steveklabnik]: https://github.com/steveklabnik
[@binarybana]: https://github.com/binarybana
[pull/66]: https://github.com/steveklabnik/simple-server/pull/66
[`9908f62`]: https://github.com/steveklabnik/simple-server/commit/9908f62529fa99ae5459f622a870ff35e3c9bb92

### Documentation

- Use `master` as links' blob for examples, fixing 404s. ([@nbigaouette]/[pull/62])

[@nbigaouette]: https://github.com/nbigaouette
[pull/62]: https://github.com/steveklabnik/simple-server/pull/62
