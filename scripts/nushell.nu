# scripts written in nu for
# - calling the API
# - running the bot with specific parameters

# the API call script
export def ultron [
  channel: string@channels # the channel to send the message to: e.g. `debug`
  message: string # the input to the bot including the command: e.g. `echo hello`
  url: string = "http://localhost:8080"
  endpoint: string = "bot"
] {
  let route = $"($url)/($endpoint)"
  http post --content-type application/json $route {
    channel: $channel
    message: $message
  }
}

export def "ultron say" [
  channel: string@channels # the channel to send the message to: e.g. ``
  message: string # what ultron should say
] {
  let message = $"echo ($message)"
  ultron $channel $message
}

# returns a list of predefined channels for autocomplete
def channels [] {
  echo [
    debug
    dnd
    psa
  ]
}
