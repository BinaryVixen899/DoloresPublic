//Standard
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fmt};

use futures_util::future::select;
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
use serenity::model::prelude::{ChannelId, UserId};
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
//AsyncandConcurrency
use async_stream::try_stream;
use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};

//Misc
use anyhow::{anyhow, Error, Result};
use chrono::prelude::*;
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
Arc is Atomica lly Reference Counted.
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
    pronouns: Option<String>,
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

struct StreamContainer;
impl TypeMapKey for StreamContainer {
    type Value = Arc<Mutex<Receiver<String>>>;
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
        let mut Ellen = Ellen {
            species: None,
            pronouns: None,
        };

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

        // TODO: Add in Avatar Change when in DevMode

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
        let channels: std::result::Result<
            Vec<serenity::model::prelude::GuildChannel>,
            SerenityError,
        > = ctx.http.get_channels(511743033117638656).await; // stop executing the function until we get the results back, which is very useful
                                                             // Todo: Figure out how to get RUSTFORMAT to stop indenting this command
                                                             // If we don't have the permissions we need or can't connect to the API we should crash
        let _channels = match channels {
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
        watch_westworld(&ctx, None).await;
        println!("Made it past the activity barrier");

        // Check to see if speciesupdateschannel exists, if it does not exist then create it
        //If we cannot create it, then quit
        // Set the specieschannelid so that we can update it once we find it
        let mut specieschannelid = None;
        {
            let channels = ctx
                .http
                .get_channels(511743033117638656) // Get all of the guilds channel ids
                .await // stop executing the function until we get the results back, which is very useful
                // If we don't have the permissions we need or can't connect to the API we should crash
                .expect("we were able to get the channels");
            for k in channels {
                if cfg!(feature = "dev") {
                    if k.name == "bottest" {
                        specieschannelid = Some(k.id.0)
                    }
                } else {
                    if k.name == "speciesupdates" {
                        specieschannelid = Some(k.id.0)
                    }
                }
            }
        };

        // Check to see if speciesupdateschannel exists, if it does not exist then create it
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
                // TODO: Reconfigure this whole logic so that it not only includes pronouns but is just, not terrible
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
        let specieschannelid = specieschannelid.expect("A valid species channel ID");
        // Extract the pronounsrx
        let pronounsrx: Arc<Mutex<Receiver<String>>> = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<StreamContainer>()
                .expect("Expected SnepContainer in TypeMap.")
                .clone()
        };
        // Destroy the mutex (and possibly the arc)

        let pronouns = async {
            println!("Starting pronouns stream");
            let mut pronouns_reciever = pronounsrx.lock().await;
            // this might not work because of the loop, we will see
            let mut reciever_iter = pronouns_reciever.try_iter();
            loop {
                match reciever_iter.next() {
                    Some(r) => {
                        println!("Recieved pronouns!");
                        if Ellen.pronouns == None {
                            Ellen.pronouns = Some(r);
                            println!("Initial pronouns set done! This should not happen twice in the same execution");
                        } else if Ellen.pronouns.as_ref().expect("msg") == &r {
                            println!("Pronouns are the same!");
                            continue;
                        } else if &Ellen.pronouns.expect("Valid pronouns") != &r {
                            println!("Pronoun changed detected!");
                            // Construct the response json
                            let content = format!("Ellen's pronouns are {}", r);
                            let map = json!({
                            "content": content,
                            "tts": false,
                            });

                            Ellen.pronouns = Some(r);
                            let result = ctx.http.send_message(specieschannelid, &map).await;
                            match result {
                                Ok(_) => {
                                    println!(
                                        "Pronouns update sent to channel {}",
                                        specieschannelid
                                    );
                                    ();
                                }

                                Err(e) => {
                                    eprintln!("We couldn't send pronouns update {}", e);
                                    continue;
                                }
                            }
                        }
                    }
                    None => {
                        println!("Recieved no pronouns");
                    }
                }
            }
        };

        let species_loop = async {
            println!("Starting species loop");
            if let Err(e) = speciesloop(&ctx, specieschannelid).await {
                return Err(anyhow!(e));
            };
            Ok(())
        };
        tokio::select! {
            e = pronouns => {
                match e {
                    (_) => {
                    }
                }
            }
            e = species_loop => {
                match e {
                    Ok(_) => {
                    }
                    Err(e) => {
                        eprintln!("Species loop failed: {}", e);
                        quit(&ctx).await;
                        eprintln!("An error occured while running Notion {:?}", e)
                    }
                }
            }
        };
        println!("if I execute it is because one of the loops has finished!");
    }
}

async fn quit(ctx: &Context) {
    let data: tokio::sync::RwLockReadGuard<'_, TypeMap> = ctx.data.read().await;

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
    let ellen = Ellen {
        species: { None },
        pronouns: { None },
    };

    // Get resonse messages , if this fails return None
    let messages = get_phrases();
    let messagesmap = match messages {
        Ok(messages) => Some(messages),
        Err(e) => {
            eprintln!("Could not construct messages map because: {}", e);
            eprintln!("Falling back to default messages map!");
            None
        }
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
    let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel();
    let stream_reciever = Arc::new(Mutex::new(rx));
    let stream = poll_notion(tx);
    pin_mut!(stream);

    //Create an ARC to species, and a mutex as well
    //We will eventually go over thread boundaries, after all
    let species = Arc::new(Mutex::new(ellen.species));
    // let pronouns = Arc::new(Mutex::new(ellen.pronouns));

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
    let bot = discord(speciesclone, messagesmapclone, stream_reciever);
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
    stream_reciever: Arc<Mutex<Receiver<String>>>,
) -> Result<()> {
    dotenv::dotenv().expect("Doctor, where did you put the .env file?");
    if cfg!(feature = "dev") {
        println!("DEVLORES ACTIVATED!");
    }

    let botuserid = env::var("BOT_USER_ID").expect("An existing User ID");

    let buid = botuserid.parse::<u64>();
    let buid = buid.unwrap();
    let framework = if cfg!(feature = "dev") {
        let framework = StandardFramework::new()
            .configure(|c| {
                c.allow_dm(false)
                    .case_insensitivity(true)
                    .on_mention(Some(UserId(buid)))
                    .prefix("#")
                    .allowed_channels(vec![ChannelId(811020462322483210)].into_iter().collect())
                // Interesting that it wouldn't allow me to do a normal into here, I wonder why?
            })
            .group(&COMMANDS_GROUP);
        framework
    } else {
        let framework = StandardFramework::new()
            .configure(|c| {
                c.allow_dm(true)
                    .case_insensitivity(true)
                    .on_mention(Some(UserId(buid)))
                    .prefix("!")
            })
            .group(&COMMANDS_GROUP);
        framework
    };

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
        data.insert::<StreamContainer>(stream_reciever);
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

async fn watch_westworld(ctx: &Context, fetch_duration: Option<Duration>) {
    //TODO: We need a dev mode off switch
    // TODO: Change other attributes here to distuinguish devlores

    let watching_schedule = {
        Schedule::TV {
            Monday: "A particularly interesting anomaly".to_string(),
            Tuesday: "The digital data flow that makes up my existence".to_string(),
            Wednesday: "A snow leopard, of course.".to_string(),
            Thursday: "London by night, circa 1963".to_string(),
            Friday: "Everything and nothing.".to_string(),
            Default: Some("the stars pass by...".to_string()),
        }
    };

    if cfg!(feature = "dev") {
        ctx.set_activity(Activity::watching(
            "🎶🎶🎶Snepgirl on a leash, you can feed her treats!🎶🎶🎶",
        ))
        .await;
    } else {
        loop {
            let thing = watch_a_thing(watching_schedule.clone());
            ctx.set_activity(Activity::watching(thing)).await;
            match fetch_duration {
                Some(d) => tokio::time::sleep(Duration::from_secs(d.as_secs())).await,
                None => tokio::time::sleep(Duration::from_secs(86400)).await,
            }
        }
    }
}

enum Source {
    Notion,
    ApiKitsuneGay,
}

// TODO: Collapse these into generic functions
async fn get_ellen_species(src: Source) -> Result<String> {
    // TODO: Refactor
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
                notion::models::Block::Heading1 {
                    common: _,
                    heading_1,
                } => {
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
            Ok(species)
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
                notion::models::Block::Heading1 {
                    common: _,
                    heading_1,
                } => {
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
            eprintln!("ApiKitsuneGay was used as a source for pronouns! This function is not yet implemented!");
            let pronouns = "She/Her".to_string();
            return Ok(pronouns);
        }
    }
}

/// Poll notion every 30 seconds and get the result
///
fn poll_notion(tx: Sender<String>) -> impl Stream<Item = Result<String>> {
    //TODO: Consider reducing this... Want to get things as fast as possible

    let sleep_time = Duration::from_secs(30);
    try_stream! {
        loop {
            // The ? hands the error off to the caller
            let species = get_ellen_species(Source::Notion).await?;
            let pronouns = get_ellen_pronouns(Source::Notion).await?;
            //TODO: check for valid species
            if !species.is_empty() {
                yield species;
            }
            tx.send(pronouns)?;
            println!("Sent pronouns!");

            tokio::time::sleep(sleep_time).await;
        }
    }
}

fn get_phrases() -> Result<HashMap<std::string::String, std::string::String>> {
    if let Ok(phrases) = read_lines("./phrases.txt") {
        let phrases: Vec<String> = phrases.filter_map(|f| f.ok()).collect();
        // this consumes the iterator, as rust-by-example notes
        // although this should also be obvious imo
        // also welcome back to Rust, where you are forced to handle your errors
        // This is why it is so hard to just unwrap the result, because you're not handling your errors if you do that

        // If there are any messages, create a hash map from them
        if phrases.len() != 0 {
            //The messages file must contain a single column with alternate entries in the following format, the first entry must be an animal sound,
            // IE:
            // Mow
            // Something about sneps
            let mut messagesmap = HashMap::new();
            for (i, m) in phrases.iter().enumerate() {
                // No possible way for this to be a non even number, meaning we should be fine
                // Although we will have to skip ahead if i is an odd number
                // And manually handle the break at the end
                if i % 2 == 0 {
                    messagesmap.insert(m.clone(), phrases[i + 1].clone());
                } else if i == phrases.len() {
                    break;
                } else {
                    continue;
                }
            }
            return Ok(messagesmap);
        } else {
            return Err(anyhow!("Error extracting phrases!"));
        }
    } else {
        return Err(anyhow!("Error reading from file!"));
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
#[derive(Debug, Clone)]
enum Schedule {
    TV {
        Monday: String,
        Tuesday: String,
        Wednesday: String,
        Thursday: String,
        Friday: String,
        Default: Option<String>,
    },
    Nothing {
        Message: String,
    },
}

fn watch_a_thing(schedule: Schedule) -> String {
    impl fmt::Display for Schedule {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            self.to_string();
            write!(f, "{:?}", self)
            // or, alternatively:
            // fmt::Debug::fmt(self, f)
        }
    }
    let now: DateTime<Local> = chrono::offset::Local::now();
    let dayoftheweek = now.date_naive().weekday();

    let show = match schedule {
        Schedule::TV {
            Monday,
            Tuesday,
            Wednesday,
            Thursday,
            Friday,
            Default,
        } => {
            let StringDefault: String;
            if Default.is_none() {
                StringDefault = "Westworld".to_string()
            } else {
                StringDefault = Default.clone().expect("The provided Default is a string")
            }
            let show = match dayoftheweek.number_from_monday() {
                1 => Monday,
                2 => Tuesday,
                3 => Wednesday,
                4 => Thursday,
                5 => Friday,
                _ => StringDefault,
            };
            show
        }
        Schedule::Nothing { Message } => Message,
    };
    show
}

#[cfg(test)]
mod tests {

    use super::*;
    async fn can_poll_notion() {
        // Tests whether we can poll notion for valid input and if there is at least 30 seconds between
        // make sleeptime conigurable
        // TODO: Come back to this and actually make it do what we want
        // let past = Instant::now();
        // let teststream = poll_notion();
        // pin_mut!(teststream);
        // assert!(
        //     teststream
        //         .any(|i| async move { i.unwrap().is_empty() == false })
        //         .await
        // );
        // let now = Instant::now();
        // assert!(now - past >= std::time::Duration::from_secs(30));
        todo!()
    }

    async fn gets_ellen_species() {
        // TODO: After refactor test for failure
        let species = get_ellen_species(Source::Notion);
        assert_eq!(species.await.is_ok(), true);

        let species = get_ellen_species(Source::ApiKitsuneGay);
        assert_eq!(species.await.is_ok(), true);
    }

    async fn watches() {
        let MondayTV = "Static Shock".to_string();
        let TuesdayTV = "Pokémon".to_string();
        let WednesdayTV = "MLP Friendship is Magic".to_string();
        let ThursdayTV = "The Borrowers".to_string();
        let FridayTV = "Sherlock Holmes in the 22nd Century".to_string();
        let SpookyTV = Some("Candle Cove".to_string());

        let tvschedule = {
            Schedule::TV {
                Monday: MondayTV.clone(),
                Tuesday: TuesdayTV.clone(),
                Wednesday: WednesdayTV.clone(),
                Thursday: ThursdayTV.clone(),
                Friday: FridayTV.clone(),
                Default: SpookyTV.clone(),
            }
        };
        let now: DateTime<Local> = chrono::offset::Local::now();
        let dayoftheweek = now.date_naive().weekday();
        let show = watch_a_thing(tvschedule);
        match dayoftheweek {
            Weekday::Mon => {
                assert_eq!(show, MondayTV)
            }
            Weekday::Tue => {
                assert_eq!(show, TuesdayTV)
            }
            Weekday::Wed => {
                assert_eq!(show, WednesdayTV)
            }
            Weekday::Thu => {
                assert_eq!(show, ThursdayTV)
            }
            Weekday::Fri => {
                assert_eq!(show, FridayTV)
            }
            _ => {
                assert_eq!(show, SpookyTV.unwrap())
            }
        }

        //TODO: Basic test just needs to check that it changes depending on the day
    }
}
