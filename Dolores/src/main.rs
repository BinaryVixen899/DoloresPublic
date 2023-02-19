use std::cell::RefCell;
use std::env;
use std::string;
//Standard
use std::sync::Arc;
use std::time::Duration;


use serenity::framework::standard::macros::hook;
//Serenity
use serenity::prelude::*;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
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
struct Commands;

// struct SnepContainer;

// impl TypeMapKey for SnepContainer {
//     type Value = Arc<Mutex<SnepContainer>>;
// }
// Arc is Atomically Reference Counted. Atomic as in safe to access concurrently, reference counted as in will be automatically deleted when not need. Mutex means only one thread can access it at once. RefCell provides interior mutability (meaning you can modify the contents, whereas normally global values are read-only)
// What is my problem? 
// I need to be able to access the value of Ellen.species from anywhere in discord
// I cannot just pass a reference because I also need to continuously modify it 
// I need a mutex

#[derive(Clone)]
struct Ellen {
    species: String,
}


impl Ellen {
    fn SetSpecies(&mut self, species: String) {
        self.species = species

    }
    
}

struct SnepContainer;

impl TypeMapKey for SnepContainer {
    type Value = Arc<Mutex<String>>;
}


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
        println!("Starting species loop");
        // Okay so the idea is that we can 
        // loop {
        //     tokio::time::sleep(Duration::from_secs(60)).await;
        //     ellen
        // }
        // put this in its own method 
        // call with join 
        // retrieve from context_clone_data 
        watch_westworld(&ctx).await;
        let species = {
            let data_read = ctx.data.read().await;
            data_read.get::<SnepContainer>().expect("Expected SnepContainer in TypeMap.").clone()
        };
        println!("Ellen is a {:?}", species.clone().as_ref())
        // let content = format!("{} is a", ready.user.name);
        // let response = MessageBuilder::new().push(content).build();
        // ctx.http.send_message(21, &serde_json::to_value(response).expect("valid")).await.expect("It worked!");
        
        
        // Put it in a loop 
        // TODO: Here
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
    let mut ellen = Ellen {species: "Kitsune".to_string()};
    let stream = poll_notion();
    pin_mut!(stream);
   
    let species = Arc::new(Mutex::new(ellen.species));
    {
        let arcclone = species.clone();
        let mut species = species.lock().await;
        *species = "staerw".to_string();
    }
    
    

    let futureb =   async {
        let arcclone = species.clone();
        while let Some(item) = stream.next().await {
            let mut species = species.lock().await;
            
            match item {
                Ok(animal) => {
                    *species = animal;
                    
                }
                Err(_) => {
                    println!("Could not set species");
                }

            }
            
            
            //TODO: Send to channel! 
        }

    };
    let futurea = discord(species.clone());

    let run = join!(futurea, futureb);
    


}

async fn discord(species: Arc<Mutex<String>>) {
    dotenv::dotenv().expect("Teddy, have you seen the .env file?");
    // println!("Ellen is a {:?}", species);
    let botuserid = env::var("BOT_USER_ID").expect("An existing User ID");
    if let Ok(buid) = botuserid.parse::<u64>() {
        let framework = StandardFramework::new().configure(|c| {
        c.allow_dm(true)
            .case_insensitivity(true)
            .on_mention(Some(UserId(buid)))
            .prefix("!")
    })
    .group(&COMMANDS_GROUP);

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
        // we need to continuously do this 
        
        {
            let mut data = client.data.write().await;
            
            data.insert::<SnepContainer>(species);

            // so what I need to construct here is a mutex 
            // Hopefully I'll be able to read Ellen on a continually updated basis
            // If not I'll have to rethink the structure of Ellen 

            // data.insert::<SnepContainer>(Arc::new(AtomicUsize::new(0))).expect();;
        }
        

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

#[hook]
async fn before (ctx: &Context, msg: &Message, anything: &str) -> bool {
    // pass Ellen in and then 
    true
    
    
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
    println!("I have identified the {}", species);
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
