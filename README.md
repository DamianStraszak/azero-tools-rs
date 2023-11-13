# azero-tools-rs

Disclaimer: this code was written as part of a fun weekend project and it is not production ready.

This project is a simple rust webserver that allows to display basic information about PSP22 tokens on Aleph Zero testnet and mainnet. It uses the following great libraries:
- [subxt](https://github.com/paritytech/subxt) for interacting with Aleph Zero (a Substrate-based chain)
- [ink-wrapper](https://github.com/Cardinal-Cryptography/ink-wrapper) for generating type-safe wrappers around a PSP22 contract
- [askama](https://github.com/djc/askama) for HTML templates using rust
- [axum](https://github.com/tokio-rs/axum) a HTTP server


The application could be stateless, but then it would take too much time to scrape the information from chain on restart, hence the server makes backups in json files from time to time.


## Running instructions

You need cargo installed to run this project. 

`cargo run --release` will run the server at `http://127.0.0.1:3000`.

## Issues and Bugs

This projects uses some heuristics to extract PSP22 holder information out of the contract, which work for standard PSP22 implementations but might fail for some non-standard ones. If you know a contract whose information is displayed incorrectly, please raise an issue in this repository, or contact me on the Aleph Zero discord server (DamianS from the Aleph Zero team).

