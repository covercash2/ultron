# come at me
set shell := ["nu", "-c"]
port := "9091"

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

command:
  http post --content-type application/json http://localhost:{{port}}/command { channel: debug command_input: "echo heck" }

echo:
  http post --content-type application/json http://localhost:{{port}}/echo { channel: debug message: "echo hello" }
