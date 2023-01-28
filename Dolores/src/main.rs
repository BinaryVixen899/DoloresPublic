//Standard
use std::env;
use std::time::Duration;


//Serenity
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

//AsyncandConcurrency 
use tokio::join;
use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use async_stream::try_stream;

//Misc
use anyhow::Result;
// use anyhow::Ok;

use reqwest::header::ACCEPT;


#[group]
#[commands(ellengender, ellenspecies, fronting)]
struct Ellen;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
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

        // Pass is used in order to not do anything if Dolores doesn't recognize anything
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
        // Hopefully doing this before the await will result in this always coming first  
        println!("{} is connected!", ready.user.name);
        watch_westworld(&ctx).await;
    }

    // async fn reaction_add(&self, _ctx: Context, _add_reaction: Reaction) {
    //     // When someone new joins, if an admin adds a check reaction bring them in and give them roles 
    //     // If an admin does an X reaction, kick them out 
    //     todo!()
    // }

    // async fn guild_member_addition(&self, ctx: Context, new_member: Member) {
    //     let content = MessageBuilder::new()
    //         .push("Welcome to Westworld!")
    //         .mention(&new_member.mention())
    //         .build();
    //     let message = ChannelId(2341324123123)
    //         .send_message(&ctx, |m| m.content(content))
    //         .await;

    //     if let Err(why) = message {
    //         eprintln!("Boss, there's a guest who's not supposed to be in Westworld, looks like they're wanted for: {:?}", why);
    //     };
    // }
}

#[tokio::main]
async fn main() {
    let stream = poll_notion();
    pin_mut!(stream);
    let mut futurea = discord();
    let futureb =   async {
        while let Some(item) = stream.next().await {
            print!("Ellen is a {:?} ", item);
            //TODO: Send to channel! 
        }

    };
    
    

    let run = join!(futurea, futureb);
    


}
async fn discord() {
    dotenv::dotenv().expect("Teddy, have you seen the .env file?");

    let botuserid = env::var("BOT_USER_ID").expect("An existing User ID");
    if let Ok(buid) = botuserid.parse::<u64>() {
        let framework = StandardFramework::new().configure(|c| {
        c.allow_dm(true)
            .case_insensitivity(true)
            .on_mention(Some(UserId(buid)))
            .prefix("!")
    })
    .group(&ELLEN_GROUP);

    // Get the token 
    let token = env::var("DISCORD_TOKEN").expect("A valid token");
    // Declare intents (these determine what events the bot will receive )
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    // Build the client 
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
}

#[command]
async fn ellenspecies(ctx: &Context, msg: &Message) -> CommandResult {
    let client = reqwest::Client::new();
    let species = client
        .get("https://api.kitsune.gay/Species")
        .header(ACCEPT, "application/json")
        .send()
        .await?
        .text()
        .await?;
    let content = format!("Ellen is a {}", species);
    let response = MessageBuilder::new().push(content).build();
    msg.reply(ctx, response).await?;

    Ok(())
    
}

#[command]
async fn ellengender(ctx: &Context, msg: &Message) -> CommandResult {
    let client = reqwest::Client::new();
    let gender = client
    .get("https:///api.kitsune.gay/Gender")
    .header(ACCEPT, "application/json")
    .send()
    .await?
    .text()
    .await?;
    let content = format!("Our pronouns are {}", gender);
    let response = MessageBuilder::new().push(content).build();
    msg.reply(ctx, response).await?;
    
    Ok(())
}

#[command]
async fn fronting(ctx: &Context, msg: &Message) -> CommandResult {
    println!("I have received the command.");
    let client = reqwest::Client::new();
    let result = client
        .get("https://api.kitsune.gay/Fronting")
        .header(ACCEPT, "application/json")
        .send()
        .await?;

    let alter = result.text().await?;
    println!("I have gotten the {}", alter);
    let content = format!("{} is in front", alter);
    let response = MessageBuilder::new().push(content).build();
    msg.reply(ctx, response).await?;

    Ok(())
}

async fn watch_westworld(ctx: &Context) {
    
    ctx.set_activity(Activity::watching("Westworld")).await;
    // TODO: Make what Dolores activity is change every day

}

async fn get_ellen_species() -> Result<String> {
    let client = reqwest::Client::new();
    let species = client
        .get("https://api.kitsune.gay/Species")
        .header(ACCEPT, "application/json")
        .send()
        .await?
        .text()
        .await?;
    Ok(species) 
}

// 
 fn poll_notion() -> impl Stream <Item = Result<String>> {
    let sleep_time = Duration::from_secs(50);
    //TODO: Find a way to return an error here 
    try_stream! {
        // TODO: Test if this will stop, if yes make it a loop
        for _ in 0..5u64 {
            let species = get_ellen_species().await?;
            //TODO: check for valid species
            if species.len() != 0 {
                yield species;
            }
            tokio::time::sleep(sleep_time).await;
        }
    }
}
