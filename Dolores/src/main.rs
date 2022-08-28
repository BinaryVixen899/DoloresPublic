use std::env;
use std::result;

use reqwest::header::ACCEPT;
use reqwest::header::ACCEPT_ENCODING;
use serenity::async_trait;
use serenity::http::error;
use serenity::model::prelude::Channel;
use serenity::model::prelude::ChannelId;
use serenity::model::prelude::Guild;
use serenity::model::prelude::GuildChannel;
use serenity::model::prelude::GuildWelcomeChannel;
use serenity::prelude::*;

use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::prelude::Member;
use serenity::model::prelude::Reaction;
use serenity::model::prelude::UserId;

use serenity::model::gateway::Activity;

use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::Args;
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::http::Http;
use serenity::utils::GuildChannelParseError;
use serenity::utils::MessageBuilder;
use tokio::sync::watch;

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
                    "The damn pony express failed to deliver the message! Goshdarnit!: {:?}",
                    why
                )
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        watchWestworld(&ctx);
        println!("{} is connected!", ready.user.name);
    }

    async fn reaction_add(&self, ctx: Context, add_reaction: Reaction) {
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

        // do the error here
        // new_member.user.mention()

        // let response = MessageBuilder::new()
        // .push(content)
        // .build();
    }
}

// Is Prelude supposed to handle tokio?

#[tokio::main]
async fn main() {
    // Let's use an env file for the token and server to connect to
    dotenv::dotenv().expect("Teddy, have you seen the .env file?");

    // Again, what declares dotenv?

    //TODO: Disabled commands
    // let disabled = vec![""].into_iter().map(|x| x.to_string()).collect();

    let botuid = Some(UserId(234234234));

    // Create framework
    let framework = StandardFramework::new().configure(|c| {
        c.allow_dm(true)
            .case_insensitivity(true)
            .disabled_commands(disabled)
            .on_mention(botuid)
            .prefix("!")
    });
    // TODO: Only allow commands to work in certain channels
    let token = env::var("DISCORD_TOKEN").expect("But there was no token!");

    let http = Http::new(&token);

    let bot_id = match http.get_current_user().await {
        Ok(bot_id) => (bot_id.id),
        Err(why) => panic!("Couldn't access the bot id: {why:?}"),
    };

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

    // TODO: At some point we need to tell it what server to connect to
}

#[command]
async fn EllenSpecies(ctx: &Context, msg: &Message) -> CommandResult {
    struct Species {
        species: String,
    }

    let client = reqwest::Client::new();
    let result = client
        .get("https://www.api.kitsune.gay/Fronting")
        .header(ACCEPT, "application/json")
        .send()
        .await?;
    if let species = result.text().await? {
        msg.reply(ctx, species).await?;
    } else {
        msg.reply(
            ctx,
            "I can't figure it out, that fox changes species every five minutes!",
        );
    }

    Ok(())
}

#[command]
async fn EllenGender() -> CommandResult {
    struct Alter {
        alter: String,
    }

    todo!()
}

async fn watchWestworld(ctx: &Context) -> CommandResult {
    ctx.set_activity(Activity::watching("Westworld")).await;
    // TODO: Make what Dolores activity is change every day

    Ok(())
}
