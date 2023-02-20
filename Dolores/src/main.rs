//Standard
use std::env;
use std::sync::Arc;
use std::time::Duration;

// Notion
use notion::ids::BlockId;
use notion::NotionApi;


//Serenity
use serenity::async_trait;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::json::JsonMap;
use serenity::model::channel::Message;
use serenity::model::gateway::Activity;
use serenity::model::gateway::Ready;
use serenity::model::prelude::UserId;
use serenity::prelude::*;
use serenity::utils::MessageBuilder;

//AsyncandConcurrency
use async_stream::try_stream;
use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use tokio::join;

//Misc
use anyhow::Result;
use serde_json::json;
use reqwest::header::ACCEPT;

#[group]
#[commands(ellengender, ellenspecies, /*fronting*/)]
struct Commands;


/* 
Arc is Atomically Reference Counted. 
Atomic as in safe to access concurrently, reference counted as in will be automatically deleted when not need. 
Mutex means only one thread can access it at once. 
RefCell provides interior mutability (meaning you can modify the contents, whereas normally global values are read-only)
Mutex also provides interior mutability.
 */ 

// Although no longer relevant, the below notes are kept for sentimental reasons
/* 
What is my problem?
I need to be able to access the value of Ellen.species from anywhere in discord
I cannot just pass a reference because I also need to continuously modify it
I need a mutex
 */
// Because this is my first extremely complex application that I'm able to show publicly, I decided to annotate all await statements, arcs, etc. 
#[derive(Clone)]
struct Ellen {
    species: Option<String>, // when the program has just started up it is possible for both of these values to be None 
}

// This is where we declare the typemapkeys that actually "transport" the values
struct SnepContainer;

impl TypeMapKey for SnepContainer {
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
                    "The damn pony express failed to deliver the message! Goshdarnit!: {:?}",
                    why
                )
            }
        }
    }

    ///Logic that executes when the "Ready" event is received. The most important thing here is the logic for posting my species
    /// Because
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        // My assumption is that once the await concludes we will continue to execute code 
        // The problem with awaiting a future in your current function is that once you've done that you are suspending execution until the future is complete
        // As a result, if watch_westworld never finishes nothing else will be done from this function. period. ever. 
        watch_westworld(&ctx).await;

        // Check to see if speciesupdateschannel exists, if it does not exist then create it
        // Get all of the guilds channel ids 
        let channels = ctx
            .http
            .get_channels(511743033117638656)
            .await // stop executing the function until we get the results back, which is very useful
            // If we don't have the permissions we need or can't connect to the API we should crash 
            .expect("we were able to get the channels");

        // Set the specieschannelid so that we can update it once we find it
        let mut specieschannelid = None;
        for k in channels {
            if k.name == "speciesupdates" {
                specieschannelid = Some(k.id.0)
            }
        }
         
        
        let mut map =  JsonMap::new();
        if specieschannelid.is_none() {
            let json = json!(
                "speciesupdates"
            );
            map.insert("name".to_string(), json);
            ctx.http.create_channel(511743033117638656, &map, None).await; //stop executing the function until the channel create finishes again useful
            //TODO: If the request fails let's crash and tell someone to create it manually 
        };
        
        

        let specieschannelid = specieschannelid.expect("A valid u64");
        // TODO: RECOVER PRE TOMASH VERSION. MIGHT  NOT BE POSSIBLE DUE TO THE WAY DISCORD DOES THIS 
        println!("Starting species loop");

        // read in the data from the typemaps
        let species = {
            let data_read = ctx.data.read().await; 
            //nobody can update the species while i'm reading it
            // hypothetically, this could deadlock 
            data_read
                .get::<SnepContainer>()
                .expect("Expected SnepContainer in TypeMap.")
                .clone()
        };

        // Create this so we can update lastspecies
        let mut lastspecies:Option<String> = None;
        loop { 
                {
                    // Get the current value of species so we can stick it in lastspecies
                    let species = species.lock().await;
                    lastspecies = species.clone();
                }
            // may have to add a sleep in here to give time for the value to change inbetween acquiring the second lock
            loop { 
                let species = species.lock().await;                
                if lastspecies.as_ref() != species.as_ref() {
                    // First we wait to get a lock to the Option<String>
                    // After we have a lock to the option string, we get a reference to its value 
                    // This allows us to use that reference when we do formatting 
                    // The reason why this works but the above has to clone is because as_ref() takes it out of the option WHILE KEEPING THE ORIGINAL 
                    // as_mut DESTROYS THE ORIGINAL 
                    println!("species is {:?}", species);
                    let species = species.as_ref().expect("Species contained a value");
                    let species = format!("{} {}", "Ellen is an", species);

                    let map = json!({
                    "content": species,
                    "tts": false,
                    });
                    
                    match ctx.http.send_message(specieschannelid, &map).await
                    {
                        Ok(_) => {
                            ();
                            break
                        }

                        Err(_) => { 
                            panic!("We couldn't create a specieschannel");
                        }

                    } 
                    
                }
                

            }

            }

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
    let ellen = Ellen {
        species: { None },
    };
    /*  Introducing the stream and pinning it mutably, 
     this is necessary in order to use next below because next takes a mutual reference NOT ownership
    */
    let stream = poll_notion();
    pin_mut!(stream);

    // Create the Arc<Mutex> for ellen's species
    /* It's important to note that we are using Tokio mutexes here. 
    Tokio mutexes have a lock that is asynchronous, yay! 
    the lock guard produced by lock is also designed to be held across await points so that's cool too
    */

    /* 
    The reason we use Arc here is a bit different. We use Arc because it's thread safe and we are ultimately do go across thread boundaries 
    */
    let species = Arc::new(Mutex::new(ellen.species));

    // assigning the future to a variable allows us to basically make a function 
    let futureb = async {
        // While the stream is still giving us values
        while let Some(item) = stream.next().await {
            // Lock species and lastspecies so its safe to modify them
            let mut species = species.lock().await;

            match item {
                Ok(transformation) => {
                    // If this is the first time we are doing this species will be None
                    // In this case, set the species and lastspecies to the same animal
                    // We then have to continue the loop, probably because Rust doesn't know that NOTHING will be modifying 
                    // the value of species inbetween is_none and is_some
                    if species.is_none() {
                        println!("Hi, I should only run once!");
                        *species = Some(transformation);
                        continue
                    };

                    if species.is_some() {
                        /* If the new item from the stream is the same as the current lastspecies,
                          Then we know nothing has changed and we should continue the loop
                        */

                        if species.as_ref().expect("Species has a value") == &transformation {
                            continue
                        }
                        else { 
                            *species = Some(transformation);
                        }
                        // However, if it is different, then update current species

                        
                    }
                }
                Err(_) => {
                    println!("Could not set species");
                }
            };

        }
    };

    // Clone an Arc pointer and increase our strong reference counts!
    let arcclone = species.clone();

    // Pass the cloned pointers to Discord
    let futurea = discord(arcclone);

    join!(futurea, futureb);
}

async fn discord(species: Arc<Mutex<Option<String>>>) {
    dotenv::dotenv().expect("Teddy, have you seen the .env file?");
    let botuserid = env::var("BOT_USER_ID").expect("An existing User ID");

    if let Ok(buid) = botuserid.parse::<u64>() {
        let framework = StandardFramework::new()
            .configure(|c| {
                c.allow_dm(true)
                    .case_insensitivity(true)
                    .on_mention(Some(UserId(buid)))
                    .prefix("!")
            })
            .group(&COMMANDS_GROUP);

        // Get the token
        let token = env::var("DISCORD_TOKEN").expect("A valid token");
        // Declare intents (these determine what events the bot will receive )
        let intents = GatewayIntents::non_privileged()
            | GatewayIntents::MESSAGE_CONTENT
            | GatewayIntents::GUILDS;

        // Build the client
        let mut client = Client::builder(token, intents)
            .event_handler(Handler)
            .framework(framework)
            .await
            .expect("And it all started with, Wyatt");

        // Get a RW lock for a short period of time so that we can shove the arc pointers into their proper containers
        {
            // Transfer the arc pointer so that it is accessible across all functions and callbacks
            let mut data = client.data.write().await;
            data.insert::<SnepContainer>(species);

        }

        if let Err(why) = client.start().await {
            println!(
                "We failed, we failed to start listening for events: {:?}",
                why
            )
        }
    }
}

// Discord Commands Section 

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
                    eprintln!(
                        "We encountered an error, so we used Notion as a backup {}",
                        e
                    );
                    get_ellen_species(Source::Notion)
                        .await
                        .expect("We got a species")
                }
            };
            species
        }
        Err(e) => return Err(e.into()),
    };

    let content = format!("Ellen is a {}", species);
    let response = MessageBuilder::new().push(content).build();
    msg.reply(ctx, response).await?;

    Ok(())
}

#[command]
async fn ellengender(ctx: &Context, msg: &Message) -> CommandResult {
            let api_token = "NotionApiToken";
            let api_token = dotenv::var(api_token).unwrap();
            let notion = NotionApi::new(api_token).expect("We were able to authenticate to Notion");
            let genderblockid =
                <BlockId as std::str::FromStr>::from_str("3aa4b832776a4e76bb23cf7dcc80df38")
                    .expect("We got a valid BlockID!");

            let genderblock = notion
                .get_block_children(genderblockid)
                .await
                .expect("We were able to get the block children");
            let genderblock = genderblock.results;

            let gender = match genderblock[3].clone() {
                notion::models::Block::Heading1 { heading_1, common:_ } => {
                    let text = heading_1.rich_text[0].clone();
                    text.plain_text().to_string()
                }
                _ => "She/Her".to_string(),
            };

            
            let content = format!("The current pronouns are {}", gender);
            let response = MessageBuilder::new().push(content).build();
            msg.reply(ctx, response).await?;

    Ok(())
}
    


// #[command]
// async fn fronting(ctx: &Context, msg: &Message) -> CommandResult {
//     println!("I have received the command.");
//     let client = reqwest::Client::new();
//     let result = client
//         .get("https://api.kitsune.gay/Fronting")
//         .header(ACCEPT, "application/json")
//         .send()
//         .await?;

//     let alter = result.text().await?;
//     println!("I have gotten the {}", alter);
//     let content = format!("{} is in front", alter);
//     let response = MessageBuilder::new().push(content).build();
//     msg.reply(ctx, response).await?;

//     Ok(())
// }

// Non Discord Functions 

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
        Source::Notion => {
            let api_token = "NotionApiToken";
            let api_token = dotenv::var(api_token).unwrap();
            let notion = NotionApi::new(api_token).expect("We were able to authenticate to Notion");
            let speciesblockid =
                <BlockId as std::str::FromStr>::from_str("3aa4b832776a4e76bb23cf7dcc80df38")
                    .expect("We got a valid BlockID!");

            let speciesblock = notion
                .get_block_children(speciesblockid)
                .await
                .expect("We were able to get the block children");
            let test = speciesblock.results;

            let species = match test[1].clone() {
                notion::models::Block::Heading1 { heading_1, common:_ } => {
                    let text = heading_1.rich_text[0].clone();
                    text.plain_text().to_string()
                }
                _ => "Kitsune".to_string(),
            };

            Ok(species)
        }

        Source::ApiKitsuneGay => {
            let client = reqwest::Client::new();
            let species = client
                .get("https://api.kitsune.gay/Species")
                .header(ACCEPT, "application/json")
                .send()
                .await?
                .text()
                .await?;
            println!("I have identified the {}", species);
            return Ok(species);
        }
    }
}

/// Poll notion every 30 seconds and get the result 
/// 
fn poll_notion() -> impl Stream<Item = Result<String>> {
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
