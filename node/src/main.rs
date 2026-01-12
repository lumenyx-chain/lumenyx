#![warn(missing_docs)]

mod chain_spec;
mod cli;
mod command;
mod rpc;
mod service;
mod rx_lx;
mod pow_import;

fn main() -> sc_cli::Result<()> {
    command::run()
}
