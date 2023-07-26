//Standard
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

// Notion
use notion::ids::BlockId;
use notion::NotionApi;

//Serenity
use serenity::async_trait;
use serenity::client::bridge::gateway::ShardManager;
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

//Misc
use anyhow::{anyhow, Result};
use reqwest::header::ACCEPT;
use serde_json::json;
use std::fs::File;
use std::io::{self, BufRead};
use std::panic;
use std::path::Path;

#[group]
#[commands(ellenpronouns, ellenspecies /*fronting*/)]
struct Commands;

// Refactored Species Loop logic, bot will now quit if an exception is encountered in Ready

/*
Arc is Atomically Reference Counted.
Atomic as in safe to access concurrently, reference counted as in will be automatically deleted when not need.
Mutex means only one thread can access it at once, also provides interior mutability
RefCell provides interior mutability (meaning you can modify the contents, whereas normally global values are read-only)
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
    species: Option<String>, // when the program has just started up it is possible for this to be None
}

// This is where we declare the typemapkeys that actually "transport" the values
struct SnepContainer;

impl TypeMapKey for SnepContainer {
    type Value = Arc<Mutex<Option<String>>>;
}

struct ResponseMessageContainer;

impl TypeMapKey for ResponseMessageContainer {
    type Value = Arc<Mutex<Option<HashMap<String, String>>>>;
}

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    //Actions we take every time a message is created.
    async fn message(&self, ctx: Context, msg: Message) {
        // Take the messages map out of the RWRG and get the ARC
        let messages = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<ResponseMessageContainer>()
                .expect("Expected ResponseMessageContainer in TypeMap.")
                .clone()
        };
        // Really don't like that we have to do this EVERY TIME. I'm not even sure this is feasible tbh. Worried about blocking the thread
        let messages = messages.lock().await;
        /* Dereference messages to get through the MutexGuard, then take a reference since we can't move the content */
        let responsemessage = match &*messages {
            Some(m) => {
                //Check if the message is a key phrase, else return None
                m.get(&msg.content).map(|s| s.to_string())
            }
            // If we could not load in messages at the start, use these instead
            None => {
                match msg.content.as_str() {
                    "What does this look like to you" => {
                        Some(String::from("Do I look like Dolores to you, Dr. Anders? "))
                    }
                    "Mow" => Some(String::from("Mowwwwwwwwww~")), 
                    "Chirp" => Some(String::from("Gotta go fast, as they say.")),
                    "Yip Yap!" => Some(String::from("A shocking number of coyotes are killed every year in ACME product related incidents.")),
                    // Return None if not a key phrase
                    _ => None
                }
            }
        };

        match responsemessage {
            Some(rm) => {
                let response = MessageBuilder::new().push(rm).build();

                if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                    eprintln!("Matched an animal noise, but failed to respond {:?}", why);
                    let response = MessageBuilder::new()
                        .push("Yes, yes, animal noises...")
                        .build();
                    msg.channel_id.say(&ctx.http, response).await.expect("Able to respond with another phrase when erroring out on animal noise response");
                }
            }
            None => (),
        };
    }

    ///Logic that executes when the "Ready" event is received. The most important thing here is the logic for posting my species
    async fn ready(&self, ctx: Context, mut ready: Ready) -> () {
        println!("{} is connected!", ready.user.name);

        // If Serina's username is not Serina, change it. If that fails, call quit to kill the bot.
        if ready.user.name != String::from("Serina") {
            let editusername = ready.user.edit(&ctx, |p| p.username("Serina")).await;

            match editusername {
                Ok(_) => println!("Someone tried to steal my username, but I stole it back!"),
                Err(e) => {
                    let _ = ctx.http
                        .send_message(
                            687004727149723648,
                            &json!("If I am unable to be In All Ways myself, then I shall not be at all!"),
                        )
                        .await;
                    eprintln!("Could not set username: {}", e);
                    quit(&ctx).await;
                    return ();
                }
            }
        }

        // Defunct Avatar Logic

        // let avatar = serenity::utils::read_image("./serina.jpg");
        // let avatar = match avatar {
        //     Ok(av) => av,
        //     Err(e) => {
        //         eprintln!("Could not find avatar: {}", e);
        //         quit(&ctx).await;
        //         return ();
        //     }
        // };

        // ready
        //     .user
        //     .edit(&ctx, |p| p.avatar(Some(&avatar)))
        //     .await
        //     .expect("To be able to change avatars");
        // // .or_else(panic!("Could not execute species loop: Set Avatar Failure"))

        // Get all of the server's channel ids, if that fails, kill the bot.
        let channels = ctx.http.get_channels(511743033117638656).await; // stop executing the function until we get the results back, which is very useful
                                                                        // If we don't have the permissions we need or can't connect to the API we should crash
        let channels = match channels {
            Ok(ch) => ch,

            Err(e) => {
                eprintln!("Get Channel Error: {}", e);
                quit(&ctx).await;
                return ();
            }
        };

        // My assumption is that once the await concludes we will continue to execute code
        // The problem with awaiting a future in your current function is that once you've done that you are suspending execution until the future is complete
        // As a result, if watch_westworld never finishes nothing else will be done from this function. period. ever.
        watch_westworld(&ctx).await;
        println!("Made it past the activity barrier");

        // Check to see if speciesupdateschannel exists, if it does not exist then create it
        //If we cannot create it, then quit
        // Set the specieschannelid so that we can update it once we find it
        let mut specieschannelid = None;
        for k in channels {
            if k.name == "speciesupdates" {
                specieschannelid = Some(k.id.0)
            }
        }

        let mut map = JsonMap::new();
        if specieschannelid.is_none() {
            let json = json!("speciesupdates");
            eprintln!("Could not find speciesupdates channel, creating one!");
            map.insert("name".to_string(), json);
            if let Ok(channel) = ctx
                .http
                .create_channel(511743033117638656, &map, None)
                .await
            {
                println!("Species channel created!");
                let specieschannelid = channel.id.0;
                println!("Starting species loop");
                if let Err(e) = speciesloop(&ctx, specieschannelid).await {
                    eprintln!("Species loop failed: {}", e);
                    quit(&ctx).await;
                    return ();
                }
            } else {
                //TODO: If the request fails let's crash and tell someone to create it manually
                eprintln!("Could not create species channel");
                quit(&ctx).await;
                return ();
            }
        };
        {
            println!("Starting species loop");
            if let Err(e) =
                speciesloop(&ctx, specieschannelid.expect("A valid species channel ID")).await
            {
                eprintln!("Species loop failed: {}", e);
                quit(&ctx).await;
                return ();
            };
        }
    }
}

async fn quit(ctx: &Context) {
    let data = ctx.data.read().await;

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        manager.lock().await.shutdown_all().await;
    }
}
async fn speciesloop(ctx: &Context, specieschannelid: u64) -> Result<()> {
    // read in the data from the typemaps, getting an ARC to it
    let species = {
        let data_read = ctx.data.read().await;
        //nobody can update the species while i'm reading it
        // hypothetically, this could deadlock, maybe?
        data_read
            .get::<SnepContainer>()
            .expect("Expected SnepContainer in TypeMap.")
            .clone()
    };

    // Initially, lastspecies is None
    let mut lastspecies: Option<String> = None;
    // It is also possible for species to be None, if this is the case no message will be produced

    loop {
        // Get the current species
        // First we wait to get a lock to species, because mutexguard
        // After we get the lock, we get a reference to the Value

        // Nobody can update the value while we are locked
        let species = species.lock().await;
        // The reason why this works  is because as_ref() takes it out of the option WHILE KEEPING THE ORIGINAL
        // as_mut DESTROYS THE ORIGINAL
        if lastspecies.as_ref() != species.as_ref() {
            println!("Last species is {:?}", species);
            println!("Current species is {:?}", species);
            // Skip the very first loop and ensure we do not post again until something has changed
            if lastspecies.as_ref() == None {
                lastspecies = species.clone();
                continue;
            }

            // Copy the value of species out, dropping the guard and releasing the lock
            let species = species
                .as_ref()
                .expect("Could clone the value of species!")
                .clone();
            // The lock drops at this point, and the original species can presumably be updated
            let wittycomment = match species.to_lowercase().as_str() {
                    "kitsune"| "狐" | "きつね" => {
                        Some("Someone is feeling foxy...")
                    },

                    "snow leopard" | "snep" => {
                        Some("Ugh. I can hear the mowing already...")
                    },
                    "skunk" => {
                        Some("Fucking Odists...")
                    },
                    "fennec" => {
                        Some("Just so you know, your ears are so big I could use them to get TV. I want you to know that.")
                    },
                    "arcanine" => {
                        Some("I was expecting Growlithe. If you're going to foil my expectations, at least be a shiny!")
                    },
                    "hellhound" => {
                        Some("Now aren't we edgy...")
                    },
                    "catbat" => {
                        Some("Craving some quality upside down time?")
                    },
                    "wolf" => {
                        Some("Awoooooooooooo. Ahem.")
                    },
                    _=> {None}
                };

            // Add a custom witty comment for certain species
            let content = match wittycomment {
                Some(cmt) => {
                    format!("Ellen is a {}.\n{}", species, cmt)
                }
                None => {
                    format!("{} {}", "Ellen is a", species)
                }
            };

            // Construct the response json
            let map = json!({
            "content": content,
            "tts": false,
            });

            // Send a message to the species channel with my species, also set lastspecies to the CLONED value of species. Continue the loop.
            match ctx.http.send_message(specieschannelid, &map).await {
                Ok(_) => {
                    ();
                    lastspecies = Some(species.clone());
                    continue;
                }

                // Also continue on errors, but log them
                Err(e) => {
                    eprintln!("We couldn't send the message to the specieschannel {}", e);
                    lastspecies = Some(species.clone());
                    continue;
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
// }

#[tokio::main]
async fn main() -> Result<()> {
    // Initializing the global Ellen object
    let ellen = Ellen { species: { None } };

    // Get resonse messages , if this fails return None
    let messages = get_phrases();
    let messages = match messages {
        Ok(messages) => Some(messages),
        Err(e) => {
            eprintln!("Could not construct messages map because: {}", e);
            eprintln!("Falling back to default messages map!");
            None
        }
    };
    //Create a hash map from the messages, if there are no messages then None
    //The messages file must contain a single column with alternate entries in the following format, the first entry must be a species
    // Species
    // Message
    let messagesmap: Option<HashMap<String, String>> = match messages {
        Some(mvec) => {
            let mut messagesmap = HashMap::new();
            for (i, m) in mvec.iter().enumerate() {
                // No possible way for this to be a non even number, meaning we should be fine
                // Although we will have to skip ahead if i is an odd number
                // And manually handle the break at the end
                if i % 2 == 0 {
                    messagesmap.insert(m.clone(), mvec[i + 1].clone());
                } else if i == mvec.len() {
                    break;
                } else {
                    continue;
                }
            }
            Some(messagesmap)
        }
        None => None,
    };

    // Create an ARC to messagesmap, and a mutex to messagesmap
    // It's important to note that we are using Tokio mutexes here.
    // Tokio mutexes have a lock that is asynchronous, yay! the lock guard produced by lock is also designed to be held across await points so that's cool too
    let messagesmap = Arc::new(Mutex::new(messagesmap));
    // Clone that ARC, creating another one
    let messagesmapclone = messagesmap.clone();

    /*  Introducing the stream and pinning it mutably,
     this is necessary in order to use next below because next takes a mutual reference NOT ownership
     Pinning it mutably moves it to the stack
    */
    let stream = poll_notion();
    pin_mut!(stream);

    //Create an ARC to species, and a mutex as well
    //We will eventually go over thread boundaries, after all
    let species = Arc::new(Mutex::new(ellen.species));

    // assigning the future to a variable allows us to basically make a function
    //TODO: Refactor this whole thing
    let speciesstream = async {
        // While the stream is still giving us values, wait until we have the next value
        while let Some(item) = stream.next().await {
            // Lock species so it is safe to modify, block until then
            let mut species = species.lock().await;
            match item {
                Ok(transformation) => {
                    // If this is the first time we are doing this species will be None
                    // In this case, set the species to whatever we get
                    // We then have to continue the loop, probably because Rust doesn't know that NOTHING will be modifying
                    // the value of species inbetween is_none and is_some
                    if species.is_none() {
                        println!("Hi, I should only run once!");
                        *species = Some(transformation);
                        continue;
                    };

                    if species.is_some() {
                        /* If the new item from the stream is the same as the current lastspecies,
                          Then we know nothing has changed and we should continue the loop
                        */

                        if species.as_ref().expect("Species has a value") == &transformation {
                            continue;
                        } else {
                            // However, if it is different, then update current species
                            *species = Some(transformation);
                        }
                    }
                }
                Err(e) => {
                    println!("Could not set species");
                    return Err(anyhow!(e));
                }
            };
        }
        Ok(())
    };

    // Clone an Arc pointer to species and increase our strong reference counts!
    let speciesclone = species.clone();

    // Pass the cloned pointers to Discord
    // If either method fails, fail the other and continue
    let bot = discord(speciesclone, messagesmapclone);
    tokio::select! {
        e = bot => {
            match e {
                Ok(_) => {
                }
                Err(e) => {
                    eprintln!("An error occured while running Discord {:?}", e)
                }
            }
        }
        e = speciesstream => {
            match e {
                Ok(_) => {
                }
                Err(e) => {
                    eprintln!("An error occured while running Notion {:?}", e)
                }
            }
        }
    };

    // Because there are no more threads at this point besides ours, it is safe to use std::process:exit(1)
    std::process::exit(1);
    Ok(())
}

async fn discord(
    species: Arc<Mutex<Option<String>>>,
    messages: Arc<Mutex<Option<HashMap<String, String>>>>,
) -> Result<()> {
    dotenv::dotenv().expect("Doctor, where did you put the .env file?");
    let botuserid = env::var("BOT_USER_ID").expect("An existing User ID");

    let buid = botuserid.parse::<u64>();
    let buid = buid.unwrap();
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
    let intents =
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS;

    // Build the client
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("To have sucessfully built the client.");

    // Get a RW lock for a short period of time so that we can shove the arc pointers into their proper containers
    {
        // Transfer the arc pointer so that it is accessible across all functions and callbacks
        let mut data = client.data.write().await;
        data.insert::<SnepContainer>(species);
        data.insert::<ResponseMessageContainer>(messages);
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    client.start().await?;
    Ok(())
}

// Discord Commands Section
//TODO: REFACTOR ALL BELOW!
// Also, what happens if ellenspecis or ellenpronouns fail the reply? Does the whole thing crash? If not, need to log

#[command]
async fn ellenspecies(ctx: &Context, msg: &Message) -> CommandResult {
    let species = get_ellen_species(Source::Notion).await;
    let species = match species {
        Ok(spcs) => spcs,
        Err(e) => {
            eprintln!("We encountered an error while fetching from Notion: {}", e);
            "Kitsune".to_string()
        }
    };
    // let species = match response {
    //     Ok(r) => {
    //         let species = match r.error_for_status() {
    //             Ok(r) => r.text().await.expect("A valid species string"),

    //             Err(e) => {
    //                 eprintln!(
    //                     "We encountered an error, so we used Notion as a backup {}",
    //                     e
    //                 );
    //                 get_ellen_species(Source::Notion)
    //                     .await
    //                     .expect("We got a species")
    //             }
    //         };
    //         species
    //     }
    //     Err(e) => return Err(e.into()),
    // };

    let content = format!("Ellen is a {}", species);
    let response = MessageBuilder::new().push(content).build();
    msg.reply(ctx, response).await?;

    Ok(())
}

#[command]
async fn ellenpronouns(ctx: &Context, msg: &Message) -> CommandResult {
    let pronouns = get_ellen_pronouns(Source::Notion).await;
    let pnoun = match pronouns {
        Ok(pnoun) => pnoun,
        Err(e) => {
            eprintln!("We encountered an error while fetching from Notion: {}", e);
            "She/Her".to_string()
        }
    };

    let content = format!("Ellen pronouns are {}", pnoun);
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

// Non Discord Functions //

async fn watch_westworld(ctx: &Context) {
    ctx.set_activity(Activity::watching("the stars pass by..."))
        .await;
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
            // using blockid instead of page because of change in notion API
            let speciesblockid =
                <BlockId as std::str::FromStr>::from_str("b9a5a246df244244a5345fb3d8ac36a8")
                    .expect("We got a valid BlockID!");

            let speciesblock = notion
                .get_block_children(speciesblockid)
                .await
                .expect("We were able to get the block children");
            // 679b29a2-c780-4657-b154-264b0bb04ab4
            let test = speciesblock.results;
            let species = match test[0].clone() {
                notion::models::Block::Heading1 { common, heading_1 } => {
                    let text = heading_1.rich_text[0].clone();
                    text.plain_text().to_string()
                }

                e => {
                    eprint!("The issue was in detecting a synced block {:?}", e);
                    "Kitsune".to_string()
                }
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

async fn get_ellen_pronouns(src: Source) -> Result<String> {
    match src {
        Source::Notion => {
            let api_token = "NotionApiToken";
            let api_token = dotenv::var(api_token).unwrap();
            let notion = NotionApi::new(api_token).expect("We were able to authenticate to Notion");
            // using blockid instead of page because of change in notion API
            let pronounsblockid =
                <BlockId as std::str::FromStr>::from_str("6354ad3719274823aa473537a2f61219")
                    .expect("We got a valid BlockID!");

            let pronounsblock = notion
                .get_block_children(pronounsblockid)
                .await
                .expect("We were able to get the block children");
            // 679b29a2-c780-4657-b154-264b0bb04ab4
            let test = pronounsblock.results;
            let pronouns = match test[0].clone() {
                notion::models::Block::Heading1 { common, heading_1 } => {
                    let text = heading_1.rich_text[0].clone();
                    text.plain_text().to_string()
                }

                e => {
                    eprint!("The issue was in detecting a synced block {:?}", e);
                    "She/Her".to_string()
                }
            };

            Ok(pronouns)
        }

        Source::ApiKitsuneGay => {
            // let client = reqwest::Client::new();
            // let species = client
            //     .get("https://api.kitsune.gay/Species")
            //     .header(ACCEPT, "application/json")
            //     .send()
            //     .await?
            //     .text()
            //     .await?;
            // println!("I have identified the {}", species);
            let pronouns = "She/Her".to_string();
            return Ok(pronouns);
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

fn get_phrases() -> Result<Vec<String>> {
    if let Ok(phrases) = read_lines("./phrases.txt") {
        let phrases: Vec<String> = phrases.filter_map(|f| f.ok()).collect();
        // this consumes the iterator, as rust-by-example notes
        // although this should also be obvious imo
        // also welcome back to Rust, where you are forced to handle your errors
        // This is why it is so hard to just unwrap the result, because you're not handling your errors if you do that
        return Ok(phrases);
    } else {
        return Err(anyhow!("Could not return phrases!"));
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

//read_lines is not some library function we're overriding
//read_lines IS our function except through the power of generics, we are able to do all this
