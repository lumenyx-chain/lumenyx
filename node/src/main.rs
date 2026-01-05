#![warn(missing_docs)]

mod chain_spec;
mod dag_sync;
mod dag_protocol;
mod orphan_pool;
mod cli;
mod command;
mod rpc;
mod service;
mod ghostdag_select;

fn main() -> sc_cli::Result<()> {
    command::run()
}
