use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use std::env;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(err) = msg.channel_id.say(&ctx.http, "hello").await {
                println!("error sending message: {:?}", err);
            }
        } else if msg.content == "!about" {
            if let Err(err) = msg
                .channel_id
                .say(&ctx.http, "https://github.com/covercash2/ultron")
                .await
            {
                println!("error sending message: {:?}", err);
            }
        } else if msg.content.contains("ultron") {
            if let Err(err) = msg
		.channel_id.say(&ctx.http, "I am always listening")
		.await
	    {
                println!("error sending message: {:?}", err);
            }
        } else {
            println!("unexpected message: {:?}", msg);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("unable load env discord token");

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("unable to create client");

    if let Err(err) = client.start().await {
        println!("client error: {:?}", err);
    }
}
