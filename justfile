# come at me
set shell := ["nu", "-c"]

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
  with-env { RUST_LOG: "info" } { cargo run } | lines | each {|line| $line | from json }

hit:
  http post --content-type application/json http://localhost:8080/bot { channel: debug message: "echo heck" }
