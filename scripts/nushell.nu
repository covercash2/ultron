# scripts written in nu for
# - calling the API
# - running the bot with specific parameters

# the XDG data directory for ultron
export const ULTRON_DATA_DIR = "~/.local/state/ultron"
const LOCAL_URL = "http://localhost:9092"
const GREEN_URL = "https://ultron.green.chrash.net"

export const ENDPOINTS = {
  api_doc: "/api_doc"
  command: "/command"
  echo: "/echo"
  events: "/events"
  health: "/healthcheck"
  index: "/"
  mcp: "/mcp"
}

const HOSTS = {
  local: $LOCAL_URL
  green: $GREEN_URL
}

def state_file [] {
  $"($ULTRON_DATA_DIR)/nu_state.toml" | path expand
}

# default state
export const DEFAULT_STATE = {
  url: $LOCAL_URL,
}

# manage ultron state.
# this command will create a state file if it doesn't exist.
# if a url is given, it will update the state file
# to use the new url.
# if no url is given, it will return the current state.
export def "ultron state" [
  --host: string@hosts
] {
  let state_file = (state_file)

  if (not ($state_file | path exists)) {
    mkdir ($ULTRON_DATA_DIR | path expand)
    $DEFAULT_STATE | save $state_file
  }

  let state = open $state_file

  let url = if $host != null { $HOSTS | get $host } else { null }

  if $url == null {
    $state
  } else if $url == $state.url {
    print "no state change"
  } else {
    let new_state = {
      url: $url,
    }

    let file = (state_file)

    print $"updating state file at ($file) to use url: ($url)"

    $new_state | save --force $file

    $new_state
  }
}

# a wrapper around POST operations
export def ultron [
  channel: string@channels # the channel to send the message to: e.g. `debug`
  event_input: string # the input to the bot including the command: e.g. `echo hello`
  --host: string@hosts # the host to send the message to
  --endpoint: string@endpoints = "command"
] {
  let route = ultron route --host $host --endpoint $endpoint

  print "POSTing to" $route

  (http post
    --content-type application/json
    $route {
      channel: $channel
      user: "nushell"
      event_input: $event_input
      event_type: "command"
    })
}

# a wrapper around GET operations
export def "ultron get" [
  --host: string@hosts # the host to send the message to
  --endpoint: string@endpoints = "command"
] {
  let state = (ultron state --host $host)

  let url = $state.url

  let route = $"($url)/($endpoint)"

  print "GETting from" $route

  http get --full --allow-errors $route
}

export def "ultron say" [
  channel: string@channels # the channel to send the message to: e.g. ``
  message: string # what ultron should say
  --url: string@urls
] {
  let message = $"echo ($message)"
  ultron $channel $message
}

# run the server
export def "ultron run" [
  --port: int # the port to run the bot on
  --log_level: string = "info" # the log level to run the bot with
] {
  with-env { RUST_LOG: $log_level } { cargo run -- --port $port }
  | lines
  | each {|line| $line | from json }
}

# construct the full route to an endpoint
# uses the saved state if no url is given
export def "ultron route" [
  --host: string@hosts
  --endpoint: string@endpoints = "command"
] {
  let state = (ultron state --host $host)

  let url = $state.url

  $"($url)/($endpoint)"
}

# returns a list of predefined channels for autocomplete
def channels [] {
  echo [
    debug
    dnd
    psa
  ]
}

# returns a list of endpoint names for autocomplete
def endpoints [] {
  $ENDPOINTS | columns
}

def hosts [] {
  $HOSTS | columns
}
