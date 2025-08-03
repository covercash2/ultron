# come at me
set shell := ["nu", "-c"]
port := "9092"

default:
  just --list

format_check:
  cargo fmt --all --check
  typos
  taplo format --check

lint:
  cargo clippy --all
  cargo fmt --all --check

# run the tests in all modules
test:
  cargo test --all

# run the bot with info logs and parse the output as json
run:
  cargo run -- --port {{port}} --rust-log "info,ultron=debug,ultron_core=debug,ultron_discord=debug" --secrets secrets.toml | lines | each {|line| $line | try { from json } catch { $line }}
