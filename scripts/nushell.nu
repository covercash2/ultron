# scripts written in nu for
# - calling the API
# - running the bot with specific parameters

# the XDG data directory for ultron
const DATA_DIR = "~/.local/state/ultron"
const DEFAULT_URL = "http://localhost:8080"

def state_file [] {
  $"($DATA_DIR)/nu_state.toml" | path expand
}

# default state
const DEFAULT_STATE = {
  url: $DEFAULT_URL,
}

# check if the input matches the state
# and save the state to the data directory if so
def check_state [
  --url: string
] {
  let state_file = (state_file)

  if (not ($state_file | path exists)) {
    mkdir ($DATA_DIR | path expand)
    $DEFAULT_STATE | save $state_file
  }

  let state = open $state_file

  if $url == null {
    $state
  } else if $url == $state.url {
    print "no state change"
  } else {
    let new_state = {
      url: $url,
    }

    print "new state" $new_state

    $new_state | save --force (state_file)
  }

  $state
}

# the API call script
export def ultron [
  channel: string@channels # the channel to send the message to: e.g. `debug`
  message: string # the input to the bot including the command: e.g. `echo hello`
  --url: string
  --endpoint: string = "bot"
] {
  let state = (check_state --url $url)

  let url = $state.url

  let route = $"($url)/($endpoint)"
  http post --content-type application/json $route {
    channel: $channel
    message: $message
  }
}

export def "ultron say" [
  channel: string@channels # the channel to send the message to: e.g. ``
  message: string # what ultron should say
  --url: string = "http://localhost:8080"
] {
  let message = $"echo ($message)"
  ultron $channel $message
}

export def "ultron run" [
  --port: int # the port to run the bot on
  --log_level: string = "info" # the log level to run the bot with
] {
  with-env { RUST_LOG: $log_level } { cargo run -- --port $port }
  | lines
  | each {|line| $line | from json }
}

# returns a list of predefined channels for autocomplete
def channels [] {
  echo [
    debug
    dnd
    psa
  ]
}

