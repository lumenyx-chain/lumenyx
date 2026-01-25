#![warn(missing_docs)]

mod chain_spec;
mod cli;
mod command;
mod pool;
mod pool_mode_handle;
mod rpc;
mod rx_lx;
mod service;

fn main() -> sc_cli::Result<()> {
    command::run()
}
