# b2creds

[![CI](https://github.com/schultetwin1/b2creds/actions/workflows/ci.yml/badge.svg)](https://github.com/schultetwin1/b2creds/actions/workflows/ci.yml)
[![Docs](https://docs.rs/b2creds/badge.svg)](https://docs.rs/b2creds/)
[![Crates.io](https://img.shields.io/crates/v/b2creds)](https://crates.io/crates/b2creds)

b2creds is a simple library built to access the credentials for BackBlaze
APIs. It mimics the access patterns of the b2 CLI tool and thus should work
on any machine where a user has logged in with the b2 CLI.

By default, b2creds will search in the following locations:

1. In the B2_APPLICATION_KEY and B2_APPLICATION_KEY_ID environmentals
   variables

2. In the sqlite database pointed to by the environmental variable
   B2_ACCOUNT_INFO

3. In the default sqlite database ~/.b2_account_info

```rust
let creds = b2creds::Credentials::locate().unwrap();
println!("Key ID: {} Key: {}", creds.application_key_id, creds.application_key);
```