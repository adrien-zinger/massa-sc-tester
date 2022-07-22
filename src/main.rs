use anyhow::{bail, Result};
use clap::Parser;
use massa_sc_runtime::{run_function, run_main};
use std::{collections::HashMap, env, fs, path::Path};
mod interface_impl;
mod ledger_interface;
mod types;

use ledger_interface::{CallItem, InterfaceImpl};

#[derive(Parser)]
#[clap(name = "Massa SC Tester")]
#[clap(author = "massalabs team")]
#[clap(version = "1.0")]
#[clap(about = "Test massa smart contracts", long_about = None)]
struct Arguments {
    #[clap(value_parser)]
    filename: Option<String>,
    #[clap(short, long, value_parser)]
    addr: Option<String>,
    #[clap(short, long, value_parser)]
    function: Option<String>,
    #[clap(short, long, value_parser)]
    coins: Option<u64>,
    #[clap(short, long, value_parser)]
    arg: Option<String>,
    #[clap(short, long, value_parser)]
    sender: Option<String>,
}

struct Inputs {
    /// Copy of the user command line arguments
    args: Arguments,
    /// module we need to start to execute
    module: Vec<u8>,
    /// Filename if user choose to call a smart contract he just build
    filename: Option<String>,
    /// Caller information like address and coins to give to the call
    caller: Option<CallItem>,
    /// (Optional function name (call main otherwise), and optional parameter
    function: Option<(String, String)>,
}

/// If argument list contains the filename, return a pair with a path and
/// the parsed module. Otherwise return None.
///
/// # Result
/// File IO result can fail. Return an error if the given path isn't readable
/// for any reason.
fn parse_file(args: &Arguments) -> Result<Option<(String, Vec<u8>)>> {
    if args.filename.is_none() {
        return Ok(None);
    }
    let path_str = args.filename.clone().unwrap();
    let path = Path::new(&path_str);
    if path.extension().unwrap_or_default() != "wasm" {
        bail!("{} should be .wasm", path.to_string_lossy())
    }
    Ok(Some((path.to_string_lossy().to_string(), fs::read(path)?)))
}

fn get_module(
    args: &Arguments,
    opt: Option<(String, Vec<u8>)>,
    interface: &InterfaceImpl,
) -> Result<Vec<u8>> {
    if args.addr.is_some() && args.filename.is_some() {
        bail!("choose between calling an address in the ledger or a file")
    }
    if let Some((_, module)) = opt {
        return Ok(module);
    }
    let addr = match args.addr.clone() {
        Some(addr) => addr,
        None => bail!("command require an address or a smart contract file"),
    };
    let bytecode = match interface.get_entry(&addr) {
        Ok(entry) => entry.bytecode,
        Err(err) => bail!("no bytecode found{}", err),
    };
    match bytecode {
        Some(module) => Ok(module),
        None => bail!("no module found at address"),
    }
}

fn get_inputs(interface: &InterfaceImpl) -> Result<Inputs> {
    // collect the arguments
    let args = Arguments::parse();
    let file = parse_file(&args)?;
    let module = get_module(&args, file.clone(), interface)?;
    let function = match args.function.clone() {
        Some(func) => Some((func, args.arg.clone().unwrap_or_default())),
        _ => None,
    };
    let caller = match args.sender.clone() {
        Some(address) => Some(CallItem {
            address,
            coins: args.coins.unwrap_or_default(),
        }),
        _ => None,
    };
    // return parsed arguments
    Ok(Inputs {
        filename: file.map(|(path, _)| path),
        module,
        function,
        caller,
        args,
    })
}

fn main() -> Result<()> {
    let ledger_context = InterfaceImpl::new()?;
    let inputs: Inputs = get_inputs(&ledger_context)?;

    ledger_context.reset_addresses()?;
    if let Some(caller) = inputs.caller {
        ledger_context.call_stack_push(caller)?;
    }
    if let Some(filename) = inputs.filename {
        println!("run file {}", filename);
    }
    if let Some(addr) = inputs.args.addr {
        println!("run addr {}", addr);
    }
    println!(
        "remaining points: {}",
        if let Some((name, param)) = inputs.function {
            run_function(
                &inputs.module,
                1_000_000_000_000,
                &name,
                &param,
                &ledger_context,
            )?
        } else {
            run_main(&inputs.module, 1_000_000_000_000, &ledger_context)?
        }
    );
    ledger_context.save()?;
    Ok(())
}
