# come at me
set shell := ["nu", "-c"]

default:
  just --list

test:
  cargo test --all

