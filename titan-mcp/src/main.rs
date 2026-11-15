//! CLI entry point for `titan-mcp-server`.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use titan_trit16_mcp::{serve_stdio, serve_tcp, serve_unix_socket};

fn print_usage() {
    eprintln!(
        "titan-mcp-server v{} (ABI 1.0.0)\n\
         Usage:\n\
         \x20 titan-mcp-server [--stdio]\n\
         \x20 titan-mcp-server --socket <unix_socket_path> [--max-connections <N>]\n\
         \x20 titan-mcp-server --tcp <127.0.0.1:port> [--max-connections <N>]",
        env!("CARGO_PKG_VERSION")
    );
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut socket_path: Option<PathBuf> = None;
    let mut tcp_addr: Option<String> = None;
    let mut max_connections: Option<usize> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--stdio" => {
                i += 1;
            }
            "--socket" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --socket requires a path argument");
                    print_usage();
                    return std::process::ExitCode::from(2);
                }
                socket_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--tcp" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --tcp requires an address argument (e.g. 127.0.0.1:4242)");
                    print_usage();
                    return std::process::ExitCode::from(2);
                }
                tcp_addr = Some(args[i + 1].clone());
                i += 2;
            }
            "--max-connections" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --max-connections requires an integer argument");
                    return std::process::ExitCode::from(2);
                }
                match args[i + 1].parse::<usize>() {
                    Ok(n) if n > 0 => max_connections = Some(n),
                    _ => {
                        eprintln!("Error: --max-connections must be a positive integer");
                        return std::process::ExitCode::from(2);
                    }
                }
                i += 2;
            }
            "--help" | "-h" => {
                print_usage();
                return std::process::ExitCode::SUCCESS;
            }
            "--version" | "-V" => {
                println!("titan-mcp-server {}", env!("CARGO_PKG_VERSION"));
                return std::process::ExitCode::SUCCESS;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                print_usage();
                return std::process::ExitCode::from(2);
            }
        }
    }

    let res = if let Some(ref path) = socket_path {
        serve_unix_socket(path, max_connections)
    } else if let Some(ref addr) = tcp_addr {
        serve_tcp(addr, max_connections)
    } else {
        serve_stdio()
    };

    match res {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("titan-mcp-server error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
