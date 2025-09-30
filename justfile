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
  cargo run -- --port {{port}} --secrets secrets.toml | lines | each {|line| $line | try { from json } catch { $line }}

command:
  http post --full --allow-errors --content-type application/json http://localhost:{{port}}/command { channel: debug user: test command_input: "echo heck" }

echo:
  http post --content-type application/json http://localhost:{{port}}/echo { channel: debug user: test message: "echo hello" }
