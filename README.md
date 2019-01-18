# Pour

_When it rains it pours._

A tool for a limited kind of load testing.

Not intended to measure total throughput or latency of an application.
Originally used for debugging a race condition triggered by a burst of requests.

I wanted to be able to:
- send requests async at large volume
- have timings for each request, as well as ok/err information logged
- send a "setlist" of specific urls to call.

It's possible to cobble this together using linux commands and utilities. However, I wanted something a little more ergonomic.

## Install

You'll need rust. See [rustup](https://rustup.rs).

You can install with stable.

### from the github

```
cargo install --git https://github.com/hwchen/pour
```
## Usage
`async` flag must be specified for async, currently. May become default in the future.

```
$ pour --async -f script.txt -n 10
```

```
$ pour --async --url https://google.com -n 10
```

## TODO:
- make `url` and `file` options conflict
- use async client
- allow base url prefix to urls in file?
