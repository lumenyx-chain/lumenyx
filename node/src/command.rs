use crate::{
    chain_spec,
    cli::{Cli, Subcommand},
    service,
};
use lumenyx_runtime::opaque::Block;
use sc_cli::SubstrateCli;
use sc_service::PartialComponents;

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Lumenyx Node".into()
    }

    fn impl_version() -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn description() -> String {
        "Lumenyx Blockchain Node".into()
    }

    fn author() -> String {
        "Anonymous".into()
    }

    fn support_url() -> String {
        "https://github.com/lumenyx-chain/lumenyx".into()
    }

    fn copyright_start_year() -> i32 {
        2025
    }

    fn load_spec(&self, id: &str) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config()?),
            "local" => Box::new(chain_spec::local_testnet_config()?),
            "" | "mainnet" => Box::new(chain_spec::mainnet_config()?),
            path => Box::new(chain_spec::ChainSpec::from_json_file(
                std::path::PathBuf::from(path),
            )?),
        })
    }
}

pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.chain_spec, config.network))
        }
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = service::new_partial(&config)?;
                Ok((cmd.run(client, backend, None), task_manager))
            })
        }
        Some(Subcommand::ChainInfo(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run::<Block>(&config))
        }
        None => {
            // Auto-create network key BEFORE create_runner
            let base_path = match cli.run.shared_params.base_path() {
                Ok(Some(bp)) => bp,
                _ => sc_service::BasePath::from_project("", "", "lumenyx-node"),
            };
            let chain_id = cli
                .run
                .shared_params
                .chain_id(cli.run.shared_params.is_dev());
            let chain_folder = match chain_id.as_str() {
                "dev" => "lumenyx_dev",
                "local" => "lumenyx_local_testnet",
                _ => "lumenyx_mainnet",
            };
            let network_path = base_path
                .path()
                .join("chains")
                .join(chain_folder)
                .join("network");
            let _ = std::fs::create_dir_all(&network_path);
            let secret_key_path = network_path.join("secret_ed25519");
            if !secret_key_path.exists() {
                use sp_core::Pair;
                let keypair = sp_core::ed25519::Pair::generate().0;
                let _ = std::fs::write(&secret_key_path, keypair.to_raw_vec());
            }

            // Capture pool_mode flag - use persisted value if exists
            use crate::pool_mode_handle::read_persisted_pool_mode;
            let pool_mode = match read_persisted_pool_mode() {
                Ok(Some(v)) => v,         // override CLI if file exists
                Ok(None) => cli.pool_mode, // use CLI flag
                Err(e) => {
                    eprintln!("WARN: cannot read persisted pool mode: {e}");
                    cli.pool_mode
                }
            };

            let runner = cli.create_runner(&cli.run)?;
            runner.run_node_until_exit(|config| async move {
                service::new_full(config, pool_mode).map_err(sc_cli::Error::Service)
            })
        }
    }
}
