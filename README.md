# azero-tools-rs

## Disclaimer

Please note that this code was developed as part of a fun weekend project. It is not intended for production use and should be treated as experimental. As such, it may lack the robustness and features needed for a production environment.

## Project

This project implements a rust web-server that allows to display basic information about PSP22 tokens on Aleph Zero testnet and mainnet. It makes use of the following great libraries:
- [subxt](https://github.com/paritytech/subxt) for interacting with Aleph Zero (a Substrate-based chain),
- [ink-wrapper](https://github.com/Cardinal-Cryptography/ink-wrapper) for generating type-safe wrappers around a PSP22 contract,
- [askama](https://github.com/djc/askama) for HTML templates using rust,
- [axum](https://github.com/tokio-rs/axum) for HTTP server.

The implementation of the server is in the `azero_webserver_psp22` crate. The remaining crates are libraries useful for interacting with the Aleph Zero chain using rust.


The application could be stateless, but then it would take too much time to scrape the information from chain on restart, hence the server makes backups in json files from time to time.

It is currently deployed on https://azero-tools.com.


## Running instructions

You need cargo installed to run this project. To run the webserver for psp22 tokens run
```
cd azero_webserver_psp22 
cargo run --release
``` 
This will run the server at `http://127.0.0.1:3000`.

## Issues and Bugs

This projects uses some heuristics to extract PSP22 holder information out of the contract, which work for standard PSP22 implementations but might fail for some non-standard ones. If you know a contract whose information is displayed incorrectly, please raise an issue in this repository, or contact me on the Aleph Zero discord server (DamianS from the Aleph Zero team).

