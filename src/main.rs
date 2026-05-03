mod account;

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "list" | "ls" => cmd_list(),
        "current" | "cur" => cmd_current(),
        "use" | "switch" => {
            if args.len() < 3 {
                eprintln!("usage: codex-switch use <name>");
                std::process::exit(1);
            }
            cmd_use(&args[2]);
        }
        "import" | "add" => {
            if args.len() < 4 {
                eprintln!("usage: codex-switch import <name> <path>");
                std::process::exit(1);
            }
            cmd_import(&args[2], &args[3]);
        }
        "sync" => {
            let extra: Vec<PathBuf> = args[2..].iter().map(PathBuf::from).collect();
            cmd_sync(&extra);
        }
        "-h" | "--help" | "help" => print_usage(),
        _ => {
            eprintln!("unknown command: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    eprintln!(
        "codex-switch — manage multiple Codex CLI accounts

USAGE:
  codex-switch list               List all accounts
  codex-switch current            Show the active account
  codex-switch use <name>         Switch to a specific account
  codex-switch import <name> <path>   Import an existing CODEX_HOME as an account
  codex-switch sync [paths...]    Merge sessions into shared pool"
    );
}

fn cmd_list() {
    let accounts = account::discover();

    if accounts.is_empty() {
        println!("No accounts found.");
        return;
    }

    for acc in &accounts {
        let marker = if acc.active { "\u{2192}" } else { " " };
        let id_short = &acc.account_id[..acc.account_id.len().min(8)];
        println!(
            "{} {:<16} {:<30} {}",
            marker, acc.alias, acc.email, id_short
        );
    }
}

fn cmd_current() {
    match account::current() {
        Some(acc) => println!("{} ({})", acc.email, acc.alias),
        None => println!("No active account."),
    }
}

fn cmd_sync(extra: &[PathBuf]) {
    println!("Merging sessions into shared pool...");
    match account::sync_sessions(extra) {
        Ok((added, skipped, merged)) => {
            println!(
                "Done: {} added, {} skipped, {} merged (kept larger).",
                added, skipped, merged
            );
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_use(name: &str) {
    match account::switch_to(name) {
        Ok(acc) => println!("Switched to {} ({})", acc.alias, acc.email),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_import(name: &str, path: &str) {
    let src = PathBuf::from(path);
    match account::import_account(name, &src) {
        Ok(acc) => println!(
            "Imported '{}' ({}). Use `codex-switch use {}` to activate.",
            acc.alias, acc.email, acc.alias
        ),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
