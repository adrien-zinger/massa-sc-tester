use anyhow::{bail, Result};
use massa_sc_runtime::{run_function, run_main};
use std::{collections::HashMap, env, fs, path::Path};

mod interface_impl;
mod ledger_interface;
mod types;

use ledger_interface::{CallItem, InterfaceImpl};

pub struct Arguments {
    filename: Option<String>,
    module: Vec<u8>,
    function: Option<(String, String)>,
    caller: Option<CallItem>,
}

fn parse_module(args: &[String]) -> Result<(Option<String>, Option<Vec<u8>>)> {
    // parse the file
    let name = args[1].clone();
    let path = Path::new(&name);
    if !path.is_file() {
        return Ok((None, None));
    }
    if path.extension().unwrap_or_default() != "wasm" {
        bail!("{} should be .wasm", name)
    }
    Ok((
        Some(path.to_string_lossy().to_string()),
        Some(fs::read(path)?),
    ))
}

fn parse_arguments(interface: &InterfaceImpl) -> Result<Arguments> {
    // collect the arguments
    let args: Vec<String> = env::args().collect();
    let len = args.len();
    println!("{}", len);
    if !(2..=5).contains(&len) {
        bail!("invalid number of arguments")
    }

    let (filename, module_opt) = parse_module(&args)?;

    // parse the configuration parameters
    let p_list: [&str; 5] = ["function", "param", "addr", "coins", "sender"];
    let mut p: HashMap<String, String> = HashMap::new();
    for v in args.iter().skip(2) {
        if let Some(index) = v.find('=') {
            let s: (&str, &str) = v.split_at(index);
            if p_list.contains(&s.0) {
                p.insert(s.0.to_string(), s.1[1..].to_string());
            } else {
                bail!("this option does not exist");
            }
        } else {
            bail!("invalid option format");
        }
    }

    let module = match module_opt {
        Some(module) => module,
        None => {
            let addr = match p.get("addr") {
                Some(addr) => addr,
                None => bail!("command reauire an address or a smart contract file"),
            };
            let bytecode = match interface.get_entry(addr) {
                Ok(entry) => entry.bytecode,
                Err(err) => bail!("no bytecode found{}", err),
            };
            match bytecode {
                Some(module) => module,
                None => bail!("no module found at address"),
            }
        }
    };

    // return parsed arguments
    Ok(Arguments {
        filename,
        module,
        addr: p.get("addr"),
        function: match (
            p.get_key_value("function").map(|x| x.1.clone()),
            p.get_key_value("param").map(|x| x.1.clone()),
        ) {
            (Some(function), Some(param)) => Some((function, param)),
            (Some(function), None) => Some((function, "".to_string())),
            _ => None,
        },
        caller: match (
            p.get_key_value("sender").map(|x| x.1.clone()),
            p.get_key_value("coins").map(|x| x.1.clone()),
        ) {
            (Some(address), Some(coins)) => Some(CallItem {
                address,
                coins: if let Ok(coins) = coins.parse::<u64>() {
                    coins
                } else {
                    println!("invalid coins, will be set to 0");
                    0
                },
            }),
            (Some(address), None) => Some(CallItem { address, coins: 0 }),
            _ => None,
        },
    })
}

fn main() -> Result<()> {
    let ledger_context = InterfaceImpl::new()?;
    let args: Arguments = parse_arguments(&ledger_context)?;

    ledger_context.reset_addresses()?;
    if let Some(caller) = args.caller {
        ledger_context.call_stack_push(caller)?;
    }
    if let Some(filename) args.filename {
        println!("run {}", filename);
    }
    if let Some(args.)
    println!(
        "remaining points: {}",
        if let Some((name, param)) = args.function {
            run_function(
                &args.module,
                1_000_000_000_000,
                &name,
                &param,
                &ledger_context,
            )?
        } else {
            run_main(&args.module, 1_000_000_000_000, &ledger_context)?
        }
    );
    ledger_context.save()?;
    Ok(())
}
