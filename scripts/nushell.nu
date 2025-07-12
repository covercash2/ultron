# scripts written in nu for
# - calling the API
# - running the bot with specific parameters

# the XDG data directory for ultron
export const ULTRON_DATA_DIR = "~/.local/state/ultron"
const LOCAL_URL = "http://localhost:8080"
const GREEN_URL = "https://ultron.green.chrash.net"

def state_file [] {
  $"($ULTRON_DATA_DIR)/nu_state.toml" | path expand
}

# default state
export const DEFAULT_STATE = {
  url: $LOCAL_URL,
}

# check if the input matches the state
# and save the state to the data directory if so
export def "ultron state" [
  --url: string@urls
] {
  let state_file = (state_file)

  if (not ($state_file | path exists)) {
    mkdir ($ULTRON_DATA_DIR | path expand)
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
  --url: string@urls
  --endpoint: string = "bot"
] {
  let state = (ultron state --url $url)

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
  --url: string@urls
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

def urls [] {
  [
    $LOCAL_URL
    $GREEN_URL
  ]
}
