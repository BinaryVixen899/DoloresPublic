use std::cell::RefCell;
use std::env;
use std::string;
//Standard
use std::sync::Arc;
use std::time::Duration;


use anyhow::anyhow;
use notion::NotionApi;
use notion::ids::BlockId;
use serde_json::json;
use serenity::framework::standard::CommandError;
use serenity::framework::standard::macros::hook;
use serenity::model::channel;
use serenity::model::guild;
use serenity::model::prelude::ChannelId;
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
    species: Option<String>,
    lastspecies: Option<String>
}



// This is where we declare the typemapkeys that actually "transport" the values
struct SnepContainer;

impl TypeMapKey for SnepContainer {
    type Value = Arc<Mutex<Option<String>>>;
}

struct LastSnepContainer;

impl TypeMapKey for LastSnepContainer {
    type Value = Arc<Mutex<Option<String>>>;
}


struct Handler;

#[async_trait]
///This command responds to certain phrases that might be said in the server
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

///Logic that executes when the "Ready" event is received. The most important thing here is the logic for posting my species
    async fn ready(&self, ctx: Context, ready: Ready) {
        // Hopefully doing this before the await will result in this always coming first  
        println!("{} is connected!", ready.user.name);
        // watch_westworld(&ctx).await;
        // TODO: handle this later 

    
        // Check to see if speciesupdateschannel exists, if it does not exist then create it 
        // In order to do this we would first need to get all of the guild's channel ids 
        let channels = ctx.http.get_channels(511743033117638656).await.expect("we were able to get the channels");
        
        let mut specieschannelid = None;
        for k in channels {
            if k.name == "speciesupdates" {
                specieschannelid = Some(k.id.0)

            }

        }
        if specieschannelid.is_none() {
            panic!("There needs to be a species channel!")
        }

        let mut specieschannelid = specieschannelid.expect("A valid u64");


        
        // ctx.http.create_channel(511743033117638656, map, audit_log_reason)

        println!("Starting species loop");
        
        loop { 
            // read in the data from the typemaps 
            let species = {
                let data_read = ctx.data.read().await;
                data_read.get::<SnepContainer>().expect("Expected SnepContainer in TypeMap.").clone()
            };
            let lastspecies = {
                let data_read = ctx.data.read().await;
                data_read.get::<LastSnepContainer>().expect("Expected LastSnepContainer in TypeMap.").clone()
            };
           
            // Compare the current values for the shared Ellen instance
            // If the current species is the same as the last species skip to the next genereation of the loop
            // Otherwise, print a message to the console with the new species
            // This is extremely inefficient but we just want a working version 
            if species.lock().await.as_ref().expect("a value") == lastspecies.lock().await.as_ref().expect("as value") {
                continue
            }
            // If we get this far we need to update lastspecies so that we don't get stuck in a loop 
            {
                // Extract a mutable reference from lastspecies 
                let mut lastspecies = lastspecies.lock().await;
                let lastspecies = lastspecies.as_mut().expect("A valid value");

                // Copy the value from species and put it into lastspecies 
                let species = species.lock().await.clone();
                let species = species.expect("This is a thing");
                *lastspecies = species;
            }
           
            // we might honesetly want to include a switch we can flick here and then flick back in the other logic, instead of relying on lastspecies to be that switch 
            
            {
            let species =  species.lock().await.clone().expect("This is a thing");
            let species = format!("{} {}", "Ellen is an", species);
            
            
            let map = json!({
                "content": species,
                "tts": false,
              });
            ctx.http.send_message(specieschannelid, &map).await;
            }
            // For some reason this slept. Not exactly sure why.  
            // tokio::time::sleep(Duration::from_secs(3)).await;
        }
       

        // Logic to send the message

        // let content = format!("{} is a", ready.user.name);
        // let response = MessageBuilder::new().push(content).build();
        // ctx.http.send_message(21, &serde_json::to_value(response).expect("valid")).await.expect("It worked!");
        
        
        
    }

    // Unused logic for automatically handling members joining 
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
    // Initializing the global Ellen object 
    let mut ellen = Ellen {species: {None}, lastspecies: {None}};
    // Introducing the stream and pinning it, this is necessary 
    let stream = poll_notion();
    pin_mut!(stream);
   
    // Create the Arc<Mutex> for ellen's species and seed it with initial data
    let species = Arc::new(Mutex::new(ellen.species));
    let lastspecies = Arc::new(Mutex::new(ellen.lastspecies));
   
    
    
    

    let futureb =   async {
        // While the stream is still giving us values 
        while let Some(item) = stream.next().await {
            // Lock the species value so its safe
            let mut species = species.lock().await;
            // Lock lastspecies so it's also safe
            let mut lastspecies = lastspecies.lock().await;
            
            match item {
                
                Ok(animal) => {
                    // we need to just store  
                    // This is bad but it is late and i am tired 
                    let animaltwo = animal.clone();
                    // If this is the first time we are doing this species will be None
                    // In this case, set the species and lastspecies to the same animal
                    if species.is_none() {
                        println!("Hi, I should only run once!");
                        let animaltwo = animal.clone();
                        *species = Some(animal);
                        *lastspecies = Some(animaltwo);
                        
                    };
                    
                    // However, all subsequent times we go around 
                    if species.is_some() {
                        // Clone these two for Reasons 
                        let animalthree = animaltwo.clone();
                        let animalfour = animaltwo.clone();
                        /* If the new item from the stream is the same as the current lastspecies,
                           Then we know nothing has changed and we should continue the loop
                         */ 
                        
                        if *lastspecies == Some(animaltwo) {
                            continue;
                        }
                        // However, if it is different, then update current species 

                        *species = Some(animalfour);
                        
                    }
                    
                    
                    
                    
                }
                Err(_) => {
                    println!("Could not set species");
                }

            };
            
            
            //TODO: Send to channel! 
        }

    };
    
    // Clone a pointer 
    let arcclone = species.clone();
    let iamalsoanarcclone = lastspecies.clone();

    // Pass the cloned pointers to Discord 
    let futurea = discord(arcclone, iamalsoanarcclone);

    let run = join!(futurea, futureb);
    


}

async fn discord(species: Arc<Mutex<Option<String>>>, lastspecies: Arc<Mutex<Option<String>>>) {
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
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS ;
    
    // Build the client 
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("And it all started with, Wyatt");
        // we need to continuously do this 
        
        {
            // Transer the arc pointer so that it is accessible across all functions and callbacks
            let mut data = client.data.write().await;
            data.insert::<SnepContainer>(species);
            data.insert::<LastSnepContainer>(lastspecies);

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
    let response = client
        .get("https://api.kitsune.gay/Species")
        .header(ACCEPT, "application/json")
        .send()
        .await;
    let species = match response {
        Ok(r) => {
            let species = match r.error_for_status() {
                Ok(r) => r.text().await.expect("A valid species string"),
                
                Err(e) => {
                    eprintln!("We encountered an error, so we used Notion as a backup {}", e);
                    get_ellen_species(Source::Notion).await.expect("We got a species")
                    
                }
            

            };
           species


        }
        Err(e) => {
            return Err(e.into())
        }

        
    };
    
    
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

enum Source {
    Notion,
    ApiKitsuneGay,    
}

async fn get_ellen_species(src: Source) -> Result<String> {
    match src { 
        Notion => {
            let api_token = "NotionApiToken";
            let api_token = dotenv::var(api_token).unwrap();
            let notion  = NotionApi::new(api_token).expect("We were able to authenticate to Notion");
            let speciesblockid = <BlockId as std::str::FromStr>::from_str("3aa4b832776a4e76bb23cf7dcc80df38").expect("We got a valid BlockID!");
            
            let speciesblock = notion.get_block_children(speciesblockid).await.expect("We were able to get the block children");
            let test = speciesblock.results;

            let species = match test[1].clone() {
                notion::models::Block::Heading1 {heading_1, common} => {
                  let text = heading_1.rich_text[0].clone();
                  text.plain_text().to_string()
                },
                _ => {
                    "Kitsune".to_string()
                }
                };
            
            Ok(species)

            

        },

        ApiKitsuneGay => {
            let client = reqwest::Client::new();
            let species = client
            .get("https://api.kitsune.gay/Species")
            .header(ACCEPT, "application/json")
            .send()
            .await?
            .text()
            .await?;
        println!("I have identified the {}", species);
        return Ok(species)
        }
    }
    
    
    
}


// 
 fn poll_notion() -> impl Stream <Item = Result<String>> {
    let sleep_time = Duration::from_secs(30);
    //TODO: Find a way to return an error here 
    try_stream! {
        // TODO: Test if this will stop, if yes make it a loop
        loop {
            let species = get_ellen_species(Source::Notion).await?;
            //TODO: check for valid species
            if species.len() != 0 {
                yield species;
            }
            tokio::time::sleep(sleep_time).await;
        }
    }
}
