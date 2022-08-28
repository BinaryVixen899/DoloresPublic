use std::env;

use reqwest::header::ACCEPT;

use serenity::prelude::*;

use serenity::async_trait;

use serenity::model::prelude::ChannelId;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::prelude::Member;
use serenity::model::prelude::Reaction;
use serenity::model::prelude::UserId;
use serenity::model::gateway::Activity;

use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};

use serenity::utils::MessageBuilder;



#[group]
#[commands(EllenSpecies)]
#[commands(EllenGender)]

// I do not understand why we have this random struct
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        // You cannot just match on msg.content with a string because it's a struct field of type String
        // This is why converting it to a String works, as well as why assigning its value to a variable works

        let responsemessage = match msg.content.as_str() {
            "What does this look like to you" => {
                String::from("Doesn't look like much of anything to me.")
            }
            "Mow" => String::from("Father, I think we're having Kieran for dinner tonight"),
            "Skunk Noises" => String::from("No, she is too precious to eat"),
            "Chirp" => String::from("Father, I think we're having Kauko for dinner!"),
            "Yip Yap!" => String::from("Father, I think I've found dinner. And it's coyote."),
            _ => String::from("Pass"),
        };

        if responsemessage != String::from("Pass") {
            let response = MessageBuilder::new().push(responsemessage).build();

            if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                println!(
                    "The damn pony express failed to deliver the message! Goshdarnit!: {:?}", why
                )
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        watch_westworld(&ctx).await;
        println!("{} is connected!", ready.user.name);
    }

    async fn reaction_add(&self, _ctx: Context, _add_reaction: Reaction) {
        // When someone new joins, if an admin adds a check reaction bring them in and give them roles 
        // If an admin does an X reaction, kick them out 
        todo!()
    }

    async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
        let content = MessageBuilder::new()
            .push("Welcome to Westworld!")
            .mention(&new_member.mention())
            .build();
        let message = ChannelId(2341324123123)
            .send_message(&ctx, |m| m.content(content))
            .await;

        if let Err(why) = message {
            eprintln!("Boss, there's a guest who's not supposed to be in Westworld, looks like they're wanted for: {:?}", why);
        };
    }
}

// Is Prelude supposed to handle tokio?

#[tokio::main]
async fn main() {
    // Let's use an env file for the token and server to connect to
    dotenv::dotenv().expect("Teddy, have you seen the .env file?");

    //TODO: Disabled commands
    // Maybe give the CLI user a list of commands to enable and then dynamically pass in disabled ones
    // let disabled = vec![""].into_iter().map(|x| x.to_string()).collect();
    
    // TODO: Refactor this, there has got to be a better way to do it.
    let botuseridstring = env::var("BOT_USER_ID").expect("But no user ID existed!");
    let botu64 = botuseridstring.parse::<u64>().unwrap();
    let botuid = Some(UserId(botu64));

    // Create framework
    let framework = StandardFramework::new().configure(|c| {
        c.allow_dm(true)
            .case_insensitivity(true)
            .on_mention(botuid)
            .prefix("!")
    });
    // TODO: Only allow commands to work in certain channels
    let token = env::var("DISCORD_TOKEN").expect("But there was no token!");


    // Declare my intents
    // TODO: Edit these, they determine what events the bot will be notifed about
    let intents = GatewayIntents::non_privileged();

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("And it all started with, Wyatt");

    if let Err(why) = client.start().await {
        println!(
            "We failed, we failed to start listening for events: {:?}",
            why
        )
    }

}

#[command]
async fn EllenSpecies(ctx: &Context, msg: &Message) -> CommandResult {
    let client = reqwest::Client::new();
    let result = client
        .get("https://www.api.kitsune.gay/Fronting")
        .header(ACCEPT, "application/json")
        .send()
        .await?;
    
    let species = result.text().await?;
    let cntnt = format!("Ellen is a {}", species);
    let response = MessageBuilder::new().push(cntnt).build();
    msg.reply(ctx, response).await?
;
    

    Ok(())
}

#[command]
async fn EllenGender() -> CommandResult {
    struct Alter {
        alter: String,
    }

    todo!()
}

async fn watch_westworld(ctx: &Context) {
    ctx.set_activity(Activity::watching("Westworld")).await;
    // TODO: Make what Dolores activity is change every day

}
