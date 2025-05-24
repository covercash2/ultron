# come at me
set shell := ["nu", "-c"]
port := "9090"

default:
  just --list

lint:
  cargo clippy --all
  cargo fmt --all --check

# run the tests in all modules
test:
  cargo test --all

# run the bot with info logs and parse the output as json
run:
  cargo run -- --port {{port}}  | lines | each {|line| $line | try { from json } catch { $line }}

hit:
  http post --content-type application/json http://localhost:{{port}}/bot { channel: debug message: "echo heck" }
