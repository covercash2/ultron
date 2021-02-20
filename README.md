# ultron

ultron represents a first look at the approaching AI singularity by responding to commands in our Discord channel.

## Begin the age of ultron (Installation)

This project makes use of rust and its package manager cargo. https://www.rust-lang.org/tools/install will show you how to install rust on any platform. Once you have run - 

### quick run

``` sh
export DISCORD_TOKEN=<discord_token_here>
export PROD_DB_URL=<production database url>
cargo run --release
```

### systemd user service

```sh
# Find your discord token using https://discordpy.readthedocs.io/en/latest/discord.html
echo DISCORD_TOKEN=<discord_token_here> > env
echo PROD_DB_URL=<production database url> >> env

cp ultron.service ~/.config/systemd/user/

# configurable in ultron.service
mkdir ~/bin ~/etc/ultron
cp env ~/etc/ultron

cargo build
cp target/debug/ultron ~/bin
# or
cargo build --release
cp target/release/ultron ~/bin
```
