//TODO: We need a dev mode off switch
// TODO: ADD IN WAIT TIME TO LOOP OTHERWISE YOU WILL CONSTANTLY BOMBARD THE CPU (you're going to need to use a channel for this)
// TODO: ALSO FIX FAILING 503s
// TODO: Also more error checking with pronouns, like have it wait a bit
// TODO: ALso dynamic phrases loadin
// TODO specify a folder for config
// TODO one day do autocomplete. One day. https://docs.rs/clap_complete/latest/clap_complete/dynamic/shells/enum.CompleteCommand.html

// Constants
mod constants;
use constants::CONFIG_PATH;
use constants::HEADER_PREFIX;
use constants::PHRASES_CONFIG_PATH;

//Standard

use clap::builder::ArgPredicate;
use clokwerk::{AsyncScheduler, Job, TimeUnits};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{env, fmt};

// use futures_util::sink::Send;
use indoc::{eprintdoc, printdoc};

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
use enum_assoc::Assoc;
use futures_core::stream::Stream;
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use strum::EnumString;
use tokio::sync::oneshot::{self, Sender};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio::try_join;
use uncased::UncasedStr;

//Honeycomb
// ToDo: read in the honeycomb token as a static or constant variable
use opentelemetry::logs::LogError;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::logs::Config;
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::{
    logs::{self as sdklogs},
    Resource,
};
use tracing::{debug, error, info, trace, trace_span, warn};
use tracing_subscriber::prelude::*;

// CLI
use clap::{Args, Parser};

//Misc
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use rand::seq::SliceRandom;
use reqwest::header::ACCEPT;
use reqwest::Url;
use serde_json::json;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

#[derive(Default, Debug, Clone)]
struct FatalError {
    msg: String,
}

impl FatalError {
    fn new(msg: &str) -> FatalError {
        FatalError {
            msg: msg.to_string(),
        }
    }
}

impl Display for FatalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for FatalError {
    fn description(&self) -> &str {
        &self.msg
    }
}

#[derive(Debug, Clone, Copy)]
struct NonFatalError;

impl Display for NonFatalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "test")
    }
}

impl std::error::Error for NonFatalError {}

#[derive(Debug, Clone)]
enum TaskErrors {
    FatalError(FatalError),
    NonFatalError(NonFatalError),
}

impl From<FatalError> for TaskErrors {
    fn from(err: FatalError) -> Self {
        Self::FatalError(err)
    }
}

impl From<NonFatalError> for TaskErrors {
    fn from(err: NonFatalError) -> Self {
        Self::NonFatalError(err)
    }
}

impl Display for TaskErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskErrors::FatalError(fa) => write!(f, "{}", fa.msg),
            TaskErrors::NonFatalError(nfa) => write!(f, "{}", nfa),
        }
    }
}

impl std::error::Error for TaskErrors {}

#[derive(Parser)]
#[command(
    version,
    about = "A discord bot + species tracker",
    long_about = "A discord bot that also keeps track of your species if you have a weird Notion setup"
)]
struct Cli {
    #[command(flatten)]
    logs: Logs,

    /// Supply a custom .env config file
    #[arg(short = 'e', long = "env", value_name = ".ENV FILE")]
    config: Option<PathBuf>,

    /// Supply a custom phrases config file
    #[arg(
        short = 'p',
        long = "phrases",
        value_name = ".TXT FILE",
        default_value = PHRASES_CONFIG_PATH
    )]
    phrases_config: PathBuf,

    /// Turns logging on
    #[arg(
        short,
        long,
        long_help = "By itself, turns on all enable-able logging types, when combined with one or more logging types, operates solely on supplied types."
    )]
    log: bool,
}

#[derive(Args, Debug)]
#[group(required = false, multiple = true, requires = "log", id = "logs")]
struct Logs {
    // action::settrue: if the user supplied this arg, set it to true
    // default value: if the user has not supplied this arg AND the if conditions are not met this arg is false, probably redundant given if log is supplied is true
    // default_value_ifs processed in order
    // default value ifs: if the user has not supplied this arg, AND any other logs args arg present, this arg or group is false
    // default value ifs: if the user has not supplied this arg AND if no other logs args are present and log arg is present this arg or group is true
    // log has to be present for any of these to run
    //
    /// logging to honeycomb
    #[arg(long="hc", action=clap::ArgAction::SetTrue, default_value="false", default_value_ifs=[("logs", ArgPredicate::IsPresent, "false"),("log", ArgPredicate::IsPresent, "true")])]
    honeycomb: bool,

    /// logging to stderr
    #[arg(short, long, action=clap::ArgAction::SetTrue, default_value="false", default_value_ifs=[("logs", ArgPredicate::IsPresent, "false"),("log", ArgPredicate::IsPresent, "true")])]
    stderr: bool,
}

#[group]
#[commands(ellenpronouns, ellenspecies /*fronting*/)]
struct Commands;

// Although no longer relevant, the below notes are kept for sentimental reasons

/*
What is my problem?
I need to be able to access the value of Ellen.species from anywhere in discord
I cannot just pass a reference because I also need to continuously modify it
I need a mutex
 */

// Because this is my first extremely complex application that I'm able to show publicly, I decided to annotate all await statements, arcs, etc.

#[derive(Clone, Debug, PartialEq, PartialOrd)]
struct Subject {
    // when the program has just started up it is possible for these values to be None
    species: Option<String>,
    pronouns: Option<String>,
}

// This is where we declare the typemapkeys that actually "transport" the values
/*
Arc is Atomically Reference Counted.
Atomic as in safe to access concurrently, reference counted as in will be automatically deleted when not need.
Mutex means only one thread can access it at once, also provides interior mutability
RefCell provides interior mutability (meaning you can modify the contents, whereas normally global values are read-only)
 */

struct SubjectContainer;

impl TypeMapKey for SubjectContainer {
    type Value = Arc<Mutex<Subject>>;
}

struct ResponseMessageContainer;

impl TypeMapKey for ResponseMessageContainer {
    type Value = Arc<Mutex<Option<HashMap<String, String>>>>;
}

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct OneShotContainer;

impl TypeMapKey for OneShotContainer {
    type Value = Arc<Sender<String>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    //Actions we take every time a message is created.
    async fn message(&self, ctx: Context, msg: Message) {
        let messages = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<ResponseMessageContainer>()
                .expect("Expected ResponseMessageContainer in TypeMap.")
                .clone()
        };
        // Really don't like that we have to do this EVERY TIME.
        let messages = messages.lock().await;
        let span = trace_span!(target: "messagemap", "message");
        let responsemessage = span.in_scope(|| {

        trace!(name: "messagesmap", "Messages lock obtained!");
        /* Dereference messages to get through the MutexGuard, then take a reference since we can't* move the content */
        let responsemessage = match &*messages {
            Some(m) => {
                info!(name: "messagesmap", "Using messages map to respond!");
                //Check if the message is a key phrase, else return None
                m.get(&msg.content.to_lowercase()).map(|s| s.to_string())
            }
            // If we could not load in messages at the start, use these instead
            None => {
                info!(name: "messagesmap", "No messages map, using default map!");
                match msg.content.as_str() {
                    msg if UncasedStr::new("What does this look like to you").eq(msg) => {
                        Some(String::from("Do I look like Dolores to you, Dr. Anders? "))
                    }
                    msg if UncasedStr::new("Mow").eq(msg) => Some(String::from("Mowwwwwwwwww~")),
                    msg if UncasedStr::new("Chirp").eq(msg) => {
                        Some(String::from("Gotta go fast, as they say."))
                   }
                    msg if UncasedStr::new("Yip Yap!").eq(&msg) => {
                        Some(String::from("A shocking number of coyotes are killed every year in ACME product related incidents."))
                    }

                    _ => None,
                }
            }
        };
        responsemessage
    });

        if let Some(rm) = responsemessage {
            let response = MessageBuilder::new().push(rm).build();

            if let Err(why) = msg.channel_id.say(&ctx.http, &response).await {
                {
                    let _test = span.enter();
                    error!(name: "messagesmap", error_text=?why, "Matched an animal noise, but failed to respond {:#?}", why);
                }
                let response = MessageBuilder::new()
                    .push("Yes, yes, animal noises...")
                    .build();
                msg.channel_id.say(&ctx.http, response).await.expect("Able to respond with another phrase when erroring out on animal noise response");
            } else {
                {
                    let _test = span.enter();
                    info!(name: "messagesmap", response = ?response, "Response to keyword: {}", &response);
                }
            }
        }
    }

    ///Logic that executes when the "Ready" event is received. The most important things here are the Eternal Tokio Task Threads
    /// The other thing we do here is bail out if we cannot start up correctly.
    async fn ready(&self, ctx: Context, mut ready: Ready) -> () {
        info!(name: "ready", username=%ready.user.name, "{} is connected!", &ready.user.name);

        // If Serina's username is not Serina, change it.
        if ready.user.name != "Serina" {
            warn!(name: "ready_username", "Username is not Serina!");
            info!("It appears my name is wrong, I will be right back.");
            debug!(name: "ready_username", "Setting username back to Serina!!");
            let editusername = timeout(
                Duration::from_secs(5),
                ready.user.edit(&ctx, |p| p.username("Serina")),
            )
            .await;

            match editusername {
                Ok(_) => {
                    debug!(name: "ready_username", "Set username back to Serina!");
                    warn!(name: "ready_username", "Someone tried to steal my username, but I stole it back!")
                }
                Err(e) => {
                    _ =  ctx.http.send_message(
                            687004727149723648,
                            &json!("If I am unable to be In All Ways myself, then I shall not be at all!"),
                        )
                        .await;
                    error!(name: "ready_username", error_text=?e, "Could not set username: {:#?}", e);
                    // If that fails, call quit to kill the bot.
                    quit(&ctx).await;
                    return;
                }
            }
        }

        // TODO: Add in Avatar Change when in DevMode

        // let avatar = serenity::utils::read_image("./serina.jpg");
        // let avatar = match avatar {
        //     Ok(av) => av,
        //     Err(e) => {
        //         error!("Could not find avatar: {}", e);
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
        info!(name: "ready", "Retrieving channels!");
        let channels = if let Ok(ch) = timeout(
            Duration::from_secs(5),
            ctx.http.get_channels(511743033117638656),
        )
        .await
        {
            match ch {
                Ok(chnls) => {
                    info!(name: "ready_channel", "Channels retrieved!");
                    chnls
                }
                Err(e) => {
                    error!(name: "ready_channel", error_text=?e, "Failed to retrieve channels {:#?}", e);
                    quit(&ctx).await;
                    return;
                }
            }
        } else {
            error!(name: "ready_channel", "Get Channel Error!");
            quit(&ctx).await;
            return;
        };
        info!(name: "ready_channel", "Channels retrieved!");

        /*
        // My assumption is that once the await concludes we will continue to execute code
        // The problem with awaiting a future in your current function is that once you've done that you are suspending execution until the future is complete
        // As a result, if watch_westworld never finishes nothing else will be done from this function. period. ever.
        // Except that is no longer true here, because we do watch_westworld in its own task
         */

        info!(name: "ready", "Made it past the initial setup");
        info!(name: "ready_channel", "Species Update Channel Setup Started");

        // Check to see if speciesupdateschannel exists,
        //If we cannot create it, then quit
        // Set the specieschannelid to None so that we can update it once we find it
        let mut specieschannelid = None;
        {
            for k in channels {
                if cfg!(feature = "dev") {
                    if k.name == "bottest" {
                        debug!(name: "ready_channel", specieschannel=?k.id.0, "bottest cid: {:#?}", k.id.0);
                        specieschannelid = Some(k.id.0)
                    }
                } else if k.name == "speciesupdates" {
                    debug!(name: "ready_channel", specieschannel=?k.id.0, "speciesupdatescid {:#?}", k.id.0);
                    specieschannelid = Some(k.id.0)
                }
            }
        };

        //if the species update channel does not exist then create it
        if specieschannelid.is_none() {
            let json = json!("speciesupdates");
            let mut map = JsonMap::new();
            error!(name: "ready_channel", "Could not find speciesupdates channel, creating one!");
            map.insert("name".to_string(), json);
            if let Ok(chnl) = &ctx
                .clone()
                .http
                .create_channel(511743033117638656, &map, None)
                .await
            {
                specieschannelid = Some(chnl.id.0);
                let specieschannelid = specieschannelid.unwrap();
                info!(name: "ready_channel", specieschannel=%specieschannelid, "Species channel created!: {:?}", specieschannelid);
            } else {
                //TODO: If the request fails let's crash and tell someone to create it manually
                error!(name:"send_species", "Could not create species channel");
                quit(&ctx).await;
                return;
            }
        }

        // THE ETERNAL TOKIO TASKS

        // Initial Run

        // Prepwork
        let mut restarts = 0;
        let updates_channel_id = specieschannelid.unwrap();

        // Scheduled Watcher Activity Changes
        let watcher_ctx = ctx.clone();
        let mut scheduler = AsyncScheduler::new();
        let _activity = if cfg!(feature = "dev") {
            scheduler
                .every(1.minutes())
                .run(move || watch_westworld(watcher_ctx.clone()));
        } else {
            scheduler
                .every(1.day())
                .at("12:00 am")
                .run(move || watch_westworld(watcher_ctx.clone()));
        };

        info!(name:"ready", "Scheduled Watcher Activity Change");

        // Create Watcher Task
        //TODO: Implement a manager for this just because, return an error if watcher_handle fails or if the loop exits and restart until the cows come home
        let watcher_ctx = ctx.clone();
        let _watcher_handle = tokio::spawn({
            async move {
                // Call manually before we schedule future runs, that way we don't have to wait a whole day to get an activity status.
                let _ = watch_westworld(watcher_ctx.clone()).await;
                loop {
                    let _ = scheduler.run_pending().await;
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
            }
        });

        // Create Species Manager and Species Task
        /* This is also where we create the streams  */
        let species_manager = tokio::spawn({
            let ctx = ctx.clone();
            async move {
                loop {
                    // The Task can fail up to five times before we bail out
                    if restarts == 5 {
                        eprintln!("Max restarts reached!");
                        break;
                    }
                    let species_handle =
                        tokio::spawn(species_worker(ctx.clone(), updates_channel_id)).await;
                    info!(name:"species_handle", "Species encountered an error and died!");
                    match species_handle {
                        Ok(Err(output)) => match output {
                            TaskErrors::FatalError(_e) => {
                                error!(name:"species_task", "Species loop encountered a fatal error and will not be restarted!");
                                break;
                            }
                            TaskErrors::NonFatalError(_) => {
                                restarts = &restarts + 1;
                                error!(name:"species_task", "Species loop encountered a non fatal error and will be restarted!");
                            }
                        },
                        Ok(Ok(_)) => break,
                        Err(err) if err.is_panic() => continue,
                        Err(_) => break,
                    }
                }
                // TODO: Return later and change this to take an error from iniside the loop
                Err::<(), anyhow::Error>(anyhow!("Managerial thread died!"))
            }
        });

        let pronouns_manager = tokio::spawn({
            let ctx = ctx.clone();
            async move {
                loop {
                    // The Task can fail up to five times before we bail out
                    if restarts == 5 {
                        eprintln!("Max restarts reached!");
                        break;
                    }
                    let pronouns_handle =
                        tokio::spawn(pronouns_worker(ctx.clone(), updates_channel_id)).await;
                    info!(name:"pronouns_handle", "Pronouns encountered an error and died!");
                    match pronouns_handle {
                        Ok(Err(output)) => match output {
                            TaskErrors::FatalError(_e) => {
                                error!(name:"pronouns_task", "Pronouns loop encountered a fatal error and will not be restarted!");
                                break;
                            }
                            TaskErrors::NonFatalError(_) => {
                                restarts = &restarts + 1;
                                error!(name:"pronouns_task", "Pronouns loop encountered a non fatal error and was restarted!");
                            }
                        },
                        Ok(Ok(_)) => break,
                        Err(err) if err.is_panic() => continue,
                        Err(_) => break,
                    }
                }
                // TODO: Return later and change this to take an error from iniside the loop
                Err::<(), anyhow::Error>(anyhow!("Managerial thread died!"))
            }
        });

        let _result = try_join!(async { pronouns_manager.await? }, async {
            species_manager.await?
        },);

        println!("Irrecoverable error!");
        quit(&ctx.clone()).await;
    }
}

async fn quit(ctx: &Context) {
    //nobody can update the species while i'm reading it
    // hypothetically, I am worried this could deadlock, maybe?
    let mut data_read = ctx.data.write().await;
    let oneshot_tx_arc = {
        data_read
            .remove::<OneShotContainer>()
            .expect("Expected SendContainer in TypeMap.")
    };

    let oneshot_tx = Arc::into_inner(oneshot_tx_arc);
    if oneshot_tx.is_none() {
        if let Some(manager) = data_read.get::<ShardManagerContainer>() {
            warn!(name: "shutdown", "Shutting down shards.");
            manager.lock().await.shutdown_all().await;
            warn!(name: "shutdown", "All shards shutdown.")
        }
        // TODO: Check if this immediatelly returns us from bot. If it doesn't, we should use a sender to indicate whether this was because of an error or not
    } else {
        let string: String = String::from("Quit was called!");
        // Specifically quit by sending on tx
        let _ = oneshot_tx.unwrap().send(string.clone());
    }
}

async fn speciesloop<S: Stream<Item = Result<String>>>(
    ctx: &Context,
    specieschannelid: u64,
    species_stream: S,
) -> Result<()> {
    info!(name: "speciesstream", "Species stream started!");
    info!(name:"speciesstream",  "Starting species loop!");

    pin_mut!(species_stream);
    // While the stream is still giving us values, wait until we have the next value
    while let Some(species) = species_stream.next().await {
        trace!(name: "speciesstream", "New species obtained!");

        let data_read = ctx.data.read().await;
        let ellen_lock: Arc<Mutex<Subject>> = data_read
            .get::<SubjectContainer>()
            .expect("Expected ResponseMessageContainer in TypeMap.")
            .clone();

        let mut ellen = ellen_lock.lock().await;

        // Use a default if species is an error
        let species = species.unwrap_or(String::from("Kitsune"));
        // pass back errors that the stream generated
        info!(name: "species_loop", species=?species, lastpronouns=?ellen.species, "Checking Species!");
        if ellen.species.as_deref() != Some(species.as_str()) {
            if ellen.species.is_none() {
                ellen.species = Some(species);
                info!(name: "species_loop", "Initial Species loop run");
                continue;
            }
            ellen.species = Some(species);
            send_species(ctx, specieschannelid).await;
        }
    }
    Err(anyhow!("Stream Ended"))
}

async fn send_species(ctx: &Context, specieschannelid: u64) {
    // Get the current species
    let data_read: tokio::sync::RwLockReadGuard<'_, TypeMap> = ctx.data.read().await;
    let ellen_lock = data_read
        .get::<SubjectContainer>()
        .expect("Expected ResponseMessageContainer in TypeMap.")
        .clone();

    let ellen = ellen_lock.lock().await;

    let species = ellen.species.as_deref().unwrap();
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
            // debug!(name:"send_species", arc_count=%Arc::strong_count(&species), "Species: {:#?}, Witty Comment: {:#?}", species, cmt);
            format!("Ellen is a {}.\n{}", species, cmt)
        }
        None => {
            // debug!(name:"send_species", arc_count=%Arc::strong_count(&species), "Species: {:#?}", species);
            format!("{} {}", "Ellen is a", species)
        }
    };

    // Construct the response json
    let map = json!({
    "content": content,
    "tts": false,
    });
    // info!(name:"send_species", arc_count=%Arc::strong_count(&species), "Species response constructed");
    // trace!(name:"send_species", arc_count=%Arc::strong_count(&species), "Species response {:#?}", map);

    // Send a message to the species channel with my species, also set lastspecies to the CLONED value of species. Continue the loop.
    match ctx.http.send_message(specieschannelid, &map).await {
        Ok(_) => {
            // info!(name:"send_species", arc_count=%Arc::strong_count(&species), "Species response succesfully sent to {}!", specieschannelid);
            info!(name:"send_species", "Species response succesfully sent to {}!", specieschannelid);
        }

        // Also continue on errors, but log them
        Err(e) => {
            // error!(name:"send_species", error_text=?e, arc_count=%Arc::strong_count(&species), "We couldn't send the message to the specieschannel {:#?}", e);
            error!(name:"send_species", "We couldn't send the message to the specieschannel {:#?}", e);
        }
    }
}

async fn pronouns_worker(ctx: Context, updates_channel_id: u64) -> Result<(), TaskErrors> {
    let pronouns_stream = poll_notion(Characteristics::Pronouns, None);
    info!(name: "poll_notion", "poll_notion stream created!");
    info!("Starting pronouns loop");
    if let Err(e) = pronounsloop(&ctx, updates_channel_id, pronouns_stream).await {
        error!(name:"ready", error_text=?e, "Pronouns loop failed: {:#?}", e);
        let fa = FatalError::new(&e.to_string());
        return Err(fa.into());
    }
    Ok(())
}

async fn species_worker(ctx: Context, updates_channel_id: u64) -> Result<(), TaskErrors> {
    let species_stream = poll_notion(Characteristics::Species, None);
    info!(name: "poll_notion", "poll_notion stream created!");
    info!(name:"ready", "Starting species loop");
    if let Err(e) = speciesloop(&ctx, updates_channel_id, species_stream).await {
        error!(name:"ready", error_text=?e, "Species loop failed: {:#?}", e);
        let fa = FatalError::new(&e.to_string());
        return Err(fa)?;
    }
    Ok(())
}

async fn pronounsloop<S: Stream<Item = Result<String>>>(
    ctx: &Context,
    updates_channel_id: u64,
    pronouns_stream: S,
) -> Result<()> {
    pin_mut!(pronouns_stream);

    while let Some(pronouns) = pronouns_stream.next().await {
        let data_read: tokio::sync::RwLockReadGuard<'_, TypeMap> = ctx.data.read().await;
        let ellen_lock = data_read
            .get::<SubjectContainer>()
            .expect("Expected ResponseMessageContainer in TypeMap.")
            .clone();

        let mut ellen = ellen_lock.lock().await;
        if let Ok(pronouns) = pronouns {
            info!(name: "pronouns_loop", pronouns=?pronouns, lastpronouns=?ellen.pronouns, "Checking Pronouns!");
            if ellen.pronouns.as_deref() != Some(pronouns.as_str()) {
                if ellen.pronouns.is_none() {
                    ellen.pronouns = Some(pronouns);
                    info!(name: "pronouns_loop", "Initial Pronouns loop run");
                    continue;
                }
                ellen.pronouns = Some(pronouns);
                pronouns_send(ctx, updates_channel_id).await;
            }
        }
    }

    Err(anyhow!("Pronouns loop failed!"))
}

async fn pronouns_send(ctx: &Context, updates_channel_id: u64) {
    let data_read: tokio::sync::RwLockReadGuard<'_, TypeMap> = ctx.data.read().await;
    let ellen_arc = data_read
        .get::<SubjectContainer>()
        .expect("Expected ResponseMessageContainer in TypeMap.")
        .clone();

    let ellen = ellen_arc.lock().await;
    let epronouns = ellen.pronouns.as_deref().unwrap();

    // oh no, this will create a new vec every time...
    let pronouns: Vec<&str> = epronouns.split('/').collect();
    assert_ne!(
        pronouns.last(),
        None,
        "An invalid value was entered for pronouns"
    );

    // get a union of these
    assert_eq!(
        pronouns.len(),
        2,
        "An invalid amount was entered for the pronouns: {:#?}",
        pronouns
    );

    debug!(name:"pronouns_send", "Made it past pronouns assert statements");

    let pronoun = *pronouns
        .choose(&mut rand::thread_rng())
        .expect("Able to take a reference to a pronoun");

    // This is where I would put witty comments, IF I HAD ANY
    let name_and_comment: (Option<&str>, Option<&str>) = match pronoun.to_lowercase().as_str() {
        "she" | "her" => (
            Some("Ellen"),
            Some("I am woman hear me roar~ Or mow, I suppose."),
        ),

        "shi" | "hir" => (Some("Ellen"), Some("Showing your Chakat roots, I assume.")),

        "he" | "him" => (Some("Ev"), Some("Oh right, you are genderfluid!")),

        "they" | "them" => (
            Some("Ev"),
            Some("More like... Wait for it. I will calculate a joke..."),
        ),
        "ey" | "em" => (Some("Ev"), Some("A fine choice.")),
        _ => (
            Some("Ev"),
            Some("Oh! A new set of pronouns! Time to finally update the code to take live input!"),
        ),
    };
    let announcement = format!(
        "{}'s pronouns are {}, ",
        name_and_comment.0.unwrap(),
        epronouns
    );

    let comment = name_and_comment.1.unwrap();
    let message = announcement + comment;
    debug!(name:"pronouns_send", "Pronouns: {:#?}, Witty Comment: {:#?}", epronouns, comment);

    let map = json!({
        "content": message,
        "tts": false,
    });
    info!(name:"pronouns_send",  "Pronouns response constructed");
    trace!(name:"pronouns_send", "Pronouns response {:#?}", map);

    match ctx.http.send_message(updates_channel_id, &map).await {
        Ok(_) => {
            info!(name: "pronouns_send", "Pronouns update sent to updates channel!");
        }
        Err(e) => {
            error!(name: "pronouns_send", error_text=?e, "We couldn't send the pronouns update to the channel: {:#?}", e);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    let config = args.config;
    let phrases = args.phrases_config;

    // TOOD: Refactor with generics if possible
    if args.log {
        // I wish I had a better strategy than just basically duplicating this, but I'm running into some complex type error stuff when I try to abstract the filter creation logic

        // Only used in one branch but required to construct stderr_log_level, which is used in multiple branches
        let stderr_filter: tracing_subscriber::EnvFilter =
            tracing_subscriber::EnvFilter::from_default_env();
        let stderr_log_level = stderr_filter.max_level_hint().map(|f| f.to_string());
        let stderr_log_level = stderr_log_level.as_deref().unwrap_or("None");

        if args.logs.stderr && args.logs.honeycomb {
            // Stderr
            let stderr_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);
            let stderr_filtered = stderr_layer.with_filter(stderr_filter);

            // Honeycomb
            if let Some(provider) = try_init_otel_logs(config.as_ref()) {
                #[allow(clippy::bind_instead_of_map)]
                let honeycomb_layer =
                    provider.and_then(|prov| Ok(OpenTelemetryTracingBridge::new(&prov)));
                let honeycomb_filter = tracing_subscriber::EnvFilter::from_default_env();
                let honeycomb_log_level = honeycomb_filter.max_level_hint().map(|f| f.to_string());
                let honeycomb_log_level = honeycomb_log_level.as_deref().unwrap_or("None");

                match honeycomb_layer {
                    Ok(honeycomb_layer) => {
                        let honeycomb_filtered = honeycomb_layer.with_filter(honeycomb_filter);
                        tracing_subscriber::registry()
                            .with(honeycomb_filtered)
                            .with(stderr_filtered)
                            .init();
                        //TODO: If I ever add support for more than just honeycomb I'd impl a struct or enum
                        printdoc! {"
                            Logging Status:
                            Stderr: {}, OTEL: {}-{}
                            ",
                            stderr_log_level, "Honeycomb", honeycomb_log_level
                        };
                    }
                    Err(e) => {
                        eprintdoc! {"
                            Something went wrong while creating the open telemetry logging layer!
                            {}
                            ", 
                            e
                        };
                        println!("Falling back to printing to standard error!");
                        let anotherfilter: tracing_subscriber::EnvFilter =
                            tracing_subscriber::EnvFilter::from_default_env();
                        let stderr_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);
                        let stderr_filtered = stderr_layer.with_filter(anotherfilter);

                        tracing_subscriber::registry().with(stderr_filtered).init();

                        printdoc! {"
                            Logging Status:
                            Stderr: {}, OTEL: {}-{}
                            ",
                            stderr_log_level, "Honeycomb", "X"
                        };
                    }
                }
            }
        } else if args.logs.stderr {
            println!("Printing logs to standard error!");
            let anotherfilter: tracing_subscriber::EnvFilter =
                tracing_subscriber::EnvFilter::from_default_env();
            let stderr_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);
            let stderr_filtered = stderr_layer.with_filter(anotherfilter);
            tracing_subscriber::registry().with(stderr_filtered).init();
            println!("Logging Status:\nStderr: {}", stderr_log_level);
        } else if args.logs.honeycomb {
            println!("Sending logs to Honeycomb!");
            if let Some(provider) = try_init_otel_logs(None) {
                #[allow(clippy::bind_instead_of_map)]
                let honeycomb_layer =
                    provider.and_then(|prov| Ok(OpenTelemetryTracingBridge::new(&prov)));
                let honeycomb_filter = tracing_subscriber::EnvFilter::from_default_env();
                let honeycomb_log_level = honeycomb_filter.max_level_hint().map(|f| f.to_string());
                let honeycomb_log_level = honeycomb_log_level.as_deref().unwrap_or("None");

                match honeycomb_layer {
                    Ok(honeycomb_layer) => {
                        let honeycomb_filtered = honeycomb_layer.with_filter(honeycomb_filter);
                        tracing_subscriber::registry()
                            .with(honeycomb_filtered)
                            .init();

                        printdoc! {"
                            Logging Status:
                            OTEL: {}-{}
                            ",
                            "Honeycomb", honeycomb_log_level
                        };
                    }
                    Err(e) => {
                        eprintln!(
                            "Something went wrong while creating the open telemetry logging layer!\n{}", e
                        );
                        eprintdoc! {"
                            Only open telemtry was selected, so there will be no logs!
                            Skipping logging!
                            Logging Status:
                            OTEL: {}-{}
                            ",
                            "Honeycomb",
                            "X"
                        };
                    }
                }
            }
        };
    } else {
        println!("Skipping logging!");
    }

    // Get response messages or fall back
    // change to % to get non string version
    let messages = get_phrases(&phrases);
    let messagesmap: Option<_> = if messages.is_some() {
        match messages.unwrap() {
            Ok(m) => {
                info!(name: "messagesmap", "Messages Map Constructed");
                debug!(name: "messagesmap", "The messages map consists of {:?}", m);
                Some(m)
            }
            Err(e) => {
                error!(name: "messagesmap", error_text=?e, "Could not construct messages map because: {:#?}", e);
                error!(name: "messagesmap", error_text=?e, "Falling back to default messages map!");
                None
            }
        }
    } else {
        error!(name: "messagesmap", "No phrases.txt file exists");
        error!(name: "messagesmap", "Falling back to default messages map!");
        None
    };

    // Create an ARC to messagesmap, and a mutex to messagesmap
    // It's important to note that we are using Tokio mutexes here.
    // Tokio mutexes have a lock that is asynchronous, yay! the lock guard produced by lock is also designed to be held across await points so that's cool too
    let messagesmap: Arc<Mutex<Option<HashMap<String, String>>>> =
        Arc::new(Mutex::new(messagesmap));
    // Clone that ARC, creating another one
    let messagesmapclone = messagesmap.clone();
    info!(name: "messagesmap", "Prepped messaging map for transfer");

    // Create the oneshot, this is what our streams will use to communicate to us
    let (oneshot_tx, oneshot_rx) = oneshot::channel::<String>();
    info!(name: "oneshot", "oneshot channel created!");
    let oneshot_arc = Arc::new(oneshot_tx);

    //TODO: Transfer oneshot tx to the

    info!(name: "main", "Building bot!");
    // Pass the cloned pointers to Discord

    // TODO: This should really be a struct like REALLY be a struct
    let bot = discord(config.as_ref(), messagesmapclone, oneshot_arc);
    info!(name: "main", "Bot built!");
    info!(name: "main", "Starting main loop!");

    tokio::select! {
        e = bot => {
            match e {
                Ok(_) => {
                    warn!(name: "main", "Bot cleanly shut down!")
                }
                Err(e) => {
                    error!(name: "main", error_text=?e, "Discord Bot Failure: {:#?}", e);
                    error!(name: "main", error_text=?e, "An error occured while running Discord {:?}", e);
                    warn!(name: "main", "Program will now exit!")
                }
            }
        }
        e = oneshot_rx => {
            //TODO: if this does not work we can wrap it in async, but it should work because oneshot_rx is a future
            // We await any message from the oneshot channel that things have failed
            match e {
                Ok(_result) => {
                    // one of our loops failed
                    error!(name: "main", "Irrecoverable Notion Failure");
                    error!(name: "main", "An error occured while running Notion!");
                    warn!(name: "main", "Program will now exit!")
                }
                Err(_e) => {
                    // we got hung up on
                    error!(name: "main", "Reciever got hung up on!");
                    warn!(name: "main", "Program will now exit!");
                }
            }
        }
    };
    // Yeah so how does this actually _work_, like, when discord is canceled does it also take spawned threads with it? probably not :(
    // Because there are no more threads at this point besides ours, it is safe to use std::process:exit(1)
    warn!(name: "main", "Exiting!!!");
    std::process::exit(1);
}

async fn discord(
    supplied_config: Option<&PathBuf>,
    messages: Arc<Mutex<Option<HashMap<String, String>>>>,
    sender: Arc<oneshot::Sender<String>>,
) -> Result<()> {
    // I could have just used unwrap or else here but I didn't realize that {} could be used within a closure. Then I would just put the succesfull comment below
    let config = &supplied_config.map_or_else(
        || {
            println!(
                "No .env config path was passed! Falling back to default .env config {}",
                CONFIG_PATH
            );
            PathBuf::from_str(CONFIG_PATH).ok().unwrap()
            // using default config path to locate env file
        },
        |cm| cm.to_owned(),
        // using supplied config path to locate env file
    );

    // this is terrible code but it's not going to be around for long (famous last words)
    supplied_config.map_or_else(
        || {
            if dotenv::from_path(config).is_ok() {
                println!("No .env config path was supplied, but we succesfully loaded environment variables from the default .env file!")
            }
            else {
                println!("No .env config path was supplied but we could not fall back to default .env file. Setting env variables outside of a .env file is currently an unsupported configuration. You are on your own.")
            }
        },
        |_| {
            println!("Using supplied .env config path to locate .env file");
            let _ = dotenv::from_path(config).inspect_err(|_| panic!("Doctor, where did you put the .env file? I traversed the path up and down but could not locate it."));
            println!("Succesfully loaded environment variables from .env file!")
        }
    );

    if cfg!(feature = "dev") {
        warn!(name: "discord", "Dev mode activated!");
    }

    let botuserid = env::var("BOT_USER_ID").expect("An existing User ID");
    // how do we error on this and log?
    debug!(name: "discord", botuserid=?botuserid, "{:#?}", botuserid);

    let buid = botuserid.parse::<u64>();
    let buid = buid.unwrap();
    info!(name: "discord", "BUID Parsed!");
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
    trace!(name: "discord", "Framework constructed!");
    // Get the token
    let token = env::var("DISCORD_TOKEN").expect("A valid token");
    trace!(name: "discord", "Token obtained!");
    // Declare intents (these determine what events the bot will receive )
    let intents =
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS;
    info!("Intents are {:?}", intents);
    // Build the client
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("To have sucessfully built the client.");
    info!(name: "discord", "Discord client successfully built!");

    let ellen = Subject {
        species: { None },
        pronouns: { None },
    };
    debug!(name: "discord", species=?ellen.species, pronouns=?ellen.pronouns, "Initial ellen settings: {:?}", ellen);

    // Get a RW lock for a short period of time so that we can shove the arc pointers into their proper containers

    {
        // Transfer the arc pointer so that it is accessible across all functions and callbacks
        let mut data = client.data.write().await;
        data.insert::<SubjectContainer>(Arc::new(Mutex::new(ellen)));
        data.insert::<ResponseMessageContainer>(messages);
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<OneShotContainer>(sender);
        info!(name: "discord", "Arc pointers transferred!")
    }

    info!(name: "discord", "Starting Discord client");
    client.start().await?;
    info!(name: "discord", "Ending Discord client");
    Ok(())
}

// Discord Commands Section
//TODO: REFACTOR ALL Discord Commands!

#[command]
async fn ellenspecies(ctx: &Context, msg: &Message) -> CommandResult {
    info!(name: "ellenspecies", "Fetching Ellen species from Notion on request!");
    let species = get_ellen_species(Source::Notion).await;
    info!(name: "ellenspecies", "Ellen Species fetched from Notion on request!");
    let species = match species {
        Ok(spcs) => spcs,
        Err(e) => {
            error!(name: "ellenspecies", error_text=?e, "We encountered an error while fetching from Notion: {:#?}", e);
            "Kitsune".to_string()
        }
    };

    info!(name: "ellenspecies", species=%species, "species {}", species);
    let content = format!("Ellen is a {}", species);
    let response = MessageBuilder::new().push(&content).build();
    msg.reply(ctx, &response).await?;
    debug!(name: "ellenspecies", content=?content, response=?response, "content: {:#?}, response: {:#?}", &content, response);
    info!(name: "ellenspecies", "Ellen species delivered to requestor");
    Ok(())
}

#[command]
async fn ellenpronouns(ctx: &Context, msg: &Message) -> CommandResult {
    info!(name: "ellenpronouns", "Fetching pronouns from Notion on request!");
    let pronouns = get_ellen_pronouns(Source::Notion).await;
    info!(name: "ellenpronouns", "Pronouns fetched from Notion on request!");
    let pnoun = match pronouns {
        Ok(pnoun) => pnoun,
        Err(e) => {
            error!(name: "ellenpronouns", error_text=?e, "We encountered an error while fetching from Notion: {:#?}", e);
            "She/Her".to_string()
        }
    };

    info!(name: "ellenpronouns", pronouns=%pnoun, "Pronouns {}", &pnoun);
    let content = format!("Ellen pronouns are {}", &pnoun);
    let response = MessageBuilder::new().push(&content).build();
    msg.reply(ctx, &response).await?;

    debug!(
        name: "ellenpronouns",
        "pronoun content: {} pronoun response: {}",
        &content, &response
    );
    info!(name: "ellenpronouns", "Pronouns delivered to requestor");

    Ok(())
}

// Non Discord Functions //

// TODO: Change other attributes here to distuinguish devrina
async fn watch_westworld(ctx: Context) {
    let activity = watch_a_thing(None);
    if cfg!(feature = "dev") {
        info!(name:"ready", "Setting ");
        ctx.set_activity(Activity::watching(
            // "🎶🎶🎶Snepgirl on a leash, you can feed her treats!🎶🎶🎶",
            "🍻🐈🎶Cheers~🎶🐈🍻",
        ))
        .await;
        info!(name: "watch_westworld", "Dev Mode Watching activity would have been set to {:#?}!", activity);
    } else {
        ctx.set_activity(Activity::watching(activity)).await;
        info!(name: "watch_westworld", "Watching activity set to {:#?}!", activity);
    }
}

enum Source {
    Notion,
    ApiKitsuneGay,
}

// TODO: Collapse these into generic functions
async fn get_ellen_species(src: Source) -> Result<String> {
    match src {
        Source::Notion => {
            debug!(name: "get_ellen_species", "Fetching species from Notion");
            let api_token = "NotionApiToken";
            let api_token = dotenv::var(api_token).unwrap();
            let notion = NotionApi::new(api_token).expect("We were able to authenticate to Notion");
            debug!(name: "get_ellen_species", "Authenticated to Notion");
            // using blockid instead of page because of change in notion API
            let speciesblockid =
                <BlockId as std::str::FromStr>::from_str("b9a5a246df244244a5345fb3d8ac36a8")
                    .expect("We got a valid BlockID!");
            debug!(name: "get_ellen_species", speciesblockid = %speciesblockid, "The speciesblockid is {:#?}", speciesblockid);
            let speciesblock: notion::models::ListResponse<notion::models::Block> =
                notion.get_block_children(speciesblockid).await?;
            debug!(name: "get_ellen_species", "Species obtained from Notion");
            trace!(name: "get_ellen_species", "Block children obtained!");
            // 679b29a2-c780-4657-b154-264b0bb04ab4
            let test = speciesblock.results;
            debug!(name: "get_ellen_species", "Speciesblock {:#?}", test);
            let species = match test[0].clone() {
                notion::models::Block::Heading1 {
                    common: _,
                    heading_1,
                } => {
                    let text = heading_1.rich_text[0].clone();
                    text.plain_text().to_string()
                }

                e => {
                    error!(name: "get_ellen_species", error_text=?e, "The issue was in detecting a synced block {:?}", e);
                    "Kitsune".to_string()
                }
            };
            debug!(name: "get_ellen_species", species=?species, "Species {:#?}", species);
            debug!(name: "get_ellen_species", "Species returned");
            Ok(species)
        }

        Source::ApiKitsuneGay => {
            debug!(name: "get_ellen_species", "Fetching species from ApiKitsuneGay");
            let client = reqwest::Client::new();
            let species = client
                .get("https://api.kitsune.gay/Species")
                .header(ACCEPT, "application/json")
                .send()
                .await?
                .text()
                .await?;
            debug!(name: "get_ellen_species", "Fetched species from ApiKitsuneGay");
            info!(name: "get_ellen_species", species=%species, "I have identified the {}", species);
            debug!(name: "get_ellen_species", "Species returned");
            Ok(species)
        }
    }
}

async fn get_ellen_pronouns(src: Source) -> Result<String> {
    match src {
        Source::Notion => {
            debug!(name: "get_ellen_pronouns", "Fetching Pronouns from Notion");
            let api_token = "NotionApiToken";
            let api_token = dotenv::var(api_token).unwrap();
            let notion = NotionApi::new(api_token).expect("We were able to authenticate to Notion");
            debug!(name: "get_ellen_pronouns", "Authenticated to Notion");
            // using blockid instead of page because of change in notion API
            let pronounsblockid =
                <BlockId as std::str::FromStr>::from_str("6354ad3719274823aa473537a2f61219")
                    .expect("We got a valid BlockID!");
            debug!(name: "get_ellen_pronouns", pronouns_block_id=?pronounsblockid, "{:#?}", pronounsblockid);
            let pronounsblock = notion.get_block_children(pronounsblockid).await?;
            // 679b29a2-c780-4657-b154-264b0bb04ab4
            debug!(name: "get_ellen_pronouns", "Pronouns obtained from Notion");
            trace!(name: "get_ellen_pronouns", "Block children obtained!");
            let test = pronounsblock.results;
            debug!(name: "get_ellen_pronouns", "Pronounsblock {:#?}", test);
            let pronouns = match test[0].clone() {
                notion::models::Block::Heading1 {
                    common: _,
                    heading_1,
                } => {
                    let text = heading_1.rich_text[0].clone();
                    text.plain_text().to_string()
                }

                e => {
                    error!(name: "get_ellen_pronouns", error_text=?e, "The issue was in detecting a synced block {:?}", e);
                    "She/Her".to_string()
                }
            };
            debug!(name: "get_ellen_pronouns", pronouns=?pronouns, "Pronouns {:#?}", pronouns);
            debug!(name: "get_ellen_pronouns", "Pronouns returned");
            Ok(pronouns)
        }

        Source::ApiKitsuneGay => {
            // TODO: Actually implement this
            // let client = reqwest::Client::new();
            // let species = client
            //     .get("https://api.kitsune.gay/Species")
            //     .header(ACCEPT, "application/json")
            //     .send()
            //     .await?
            //     .text()
            //     .await?;
            // println!("I have identified the {}", species);
            error!(name: "get_ellen_pronouns", "ApiKitsuneGay was used as a source for pronouns! This function is not yet implemented!");
            // We fake it out here
            let pronouns = "She/Her".to_string();
            Ok(pronouns)
        }
    }
}

#[derive(Debug, EnumString)]
enum Characteristics {
    Species,
    Pronouns,
}
/// Poll notion every Duration, grabbing a characteristic
///
fn poll_notion(
    characteristic: Characteristics,
    mut sleep_time: Option<Duration>,
) -> impl Stream<Item = Result<String>> {
    if sleep_time.is_none() {
        sleep_time = Some(Duration::from_secs(30));
    }
    let sleep_time = sleep_time.unwrap();

    try_stream! {
        // TODO: Refactor this so if both are supplied it hands back both and the callee needs to figure out how to handle that
        loop {
            info!(name: "poll_notion", "Polling notion for");
            // The ? hands the error off to the caller
            match characteristic {
                Characteristics::Pronouns => {
                    let result = get_ellen_pronouns(Source::Notion).await?;
                    yield result;
                },
                Characteristics::Species => {
                    let result = get_ellen_species(Source::Notion).await?;
                    yield result;
                }
            };
            info!(name: "poll_notion", characteristic=?characteristic, "${:?} obtained from notion!", characteristic);
            info!(name: "poll_notion", "Handed off ${:?} to stream!", characteristic);
            tokio::time::sleep(sleep_time).await;
        }
    }
}

fn get_phrases(
    phrases_config: &PathBuf,
) -> Option<Result<HashMap<std::string::String, std::string::String>>> {
    if phrases_config
        .to_str()
        .expect("Able to convert phrases_config from a path to a string")
        == PHRASES_CONFIG_PATH
    {
        println!("Using default phrases config path {}", PHRASES_CONFIG_PATH)
    } else {
        println!("Using custom phrases config path {:?}", phrases_config)
    }
    if phrases_config.is_file() {
        info!(name: "get_phrases", "Reading lines in");
        if let Ok(phrases) = read_lines(phrases_config) {
            info!(name: "get_phrases", "Read lines in!");
            // Originally I used filter map but it turns out you can get an unlimited string of errors from filter_map if it's acting on Lines
            // this consumes the iterator, as rust-by-example notes
            /* also welcome back to Rust, where you are forced to handle your errors
            This is why it is so hard to just unwrap the result, because you're not handling your errors if you do that */
            let phrases: Vec<String> = phrases.map_while(Result::ok).collect();
            debug!(name: "get_phrases", phrases=?phrases, "phrases {:#?}", phrases);

            // If there are any messages, create a hash map from them
            if !phrases.is_empty() {
                info!(name: "get_phrases", "Parsing phrases");
                //The messages file must contain a single column with alternate entries in the following format, the first entry must be an animal sound,
                // IE:
                // Mow
                // Something about sneps
                let mut messagesmap = HashMap::new();
                for (i, m) in phrases.iter().enumerate() {
                    // No possible way for this to be a non even number so we do not handle that
                    if i % 2 == 0 {
                        messagesmap.insert(m.clone().to_lowercase(), phrases[i + 1].clone());
                    } else if i == phrases.len() {
                        break;
                    } else {
                        continue;
                    }
                }
                info!(name: "get_phrases", "Phrases succesfully parsed");
                trace!(name: "messagesmap", messagemap=?messagesmap, "messagesmap: {:#?}", messagesmap);
                Some(Ok(messagesmap))
            } else {
                error!(name: "get_phrases", "Phrases could not be parsed!");
                Some(Err(anyhow!("Error extracting phrases!")))
            }
        } else {
            error!(name: "get_phrases", "Error reading from file!");
            Some(Err(anyhow!("Error reading from file!")))
        }
    } else {
        info!(name: "get_phrases", "Phrases config file either does not exist or is not accessible");
        None
    }
}

/*read_lines is not some library function we're overriding
read_lines IS our function except through the power of generics, we are able to do all this*/
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// Declare this here instead of at the first so that we can easily refer to the values
#[derive(Assoc)]
#[func(pub fn show(&self, param: &str) -> String)]
#[func(pub fn with_default(&self) ->  &'static str)]
enum Schedule {
    #[assoc(show = String::new() + param)]
    #[assoc(with_default = "A particularly interesting anomaly")]
    Monday,
    #[assoc(show = String::new() + param)]
    #[assoc(with_default = "The digital data flow that makes up my existence")]
    Tuesday,
    #[assoc(show = String::new() + param)]
    #[assoc(with_default = "A snow leopard, of course.")]
    Wednesday,
    #[assoc(show = String::new() + param)]
    #[assoc(with_default = "London by night, circa 1963")]
    Thursday,
    #[assoc(show = String::new() + param)]
    #[assoc(with_default = "Everything and nothing.")]
    Friday,
    #[assoc(show = String::new() + param)]
    #[assoc(with_default = "the stars pass by...")]
    Nothing,
}

// TODO: refactor watch_a_thing to take an option of something that implements Schedule, this will potentially allow us to work around not being able to provide A Thing That Has Shows For Every Day of the Week
fn watch_a_thing(_schedule: Option<Schedule>) -> &'static str {
    let now: DateTime<Local> = chrono::offset::Local::now();
    let dayoftheweek = now.date_naive().weekday();
    debug!(name: "watch_a_thing", day=%dayoftheweek, "Day of week: {}", dayoftheweek);
    // let string_default = default.unwrap_or(String::from("Westworld"));

    let show = match dayoftheweek.number_from_monday() {
        1 => Schedule::Monday.with_default(),
        2 => Schedule::Tuesday.with_default(),
        3 => Schedule::Wednesday.with_default(),
        4 => Schedule::Thursday.with_default(),
        5 => Schedule::Friday.with_default(),
        _ => Schedule::Nothing.with_default(),
    };
    debug!(name: "watch_a_thing", show=?show, "Today's show is {:#?}", show);
    show
}

/// Tries to initiate open telemetry logging
/// Requires that the environment variables OTEL_EXPORTER_OTLP_ENDPOINT and OTEL_SERVICE_NAME are set
/// Optionally take a config path containing a .env file with environment variables
/// If a config path is not supplied, will search under /etc/serina/.env
/// As try implies, is resistant to failure, will error if it cannot parse a config but will return None if it cannot find the proper variables.
fn try_init_otel_logs(
    config_path: Option<&PathBuf>,
) -> Option<Result<sdklogs::LoggerProvider, LogError>> {
    if let Some(config_path) = config_path {
        // Keep the compiler happy with the fact we never unwrap the success
        let config_path = config_path.as_path();
        match dotenv::from_path(config_path) {
            Ok(_) => {
                println!(
                    "Using supplied config path {} for .env!",
                    config_path.to_string_lossy()
                );
            }
            Err(e) => {
                panic!(
                    "Error encountered extracting .env using supplied config_path {}: {}",
                    config_path.to_string_lossy(),
                    e
                );
            }
        }
    } else {
        println!("Config path not supplied!");
        println!("Using config from default config path {}", CONFIG_PATH);
        let _ = dotenv::from_path(CONFIG_PATH).inspect_err(|e| {
            panic!(
                "Should have been able to use default config path: {}\n{}!",
                CONFIG_PATH, e
            )
        });
    }

    // Both of these have to be supplied
    if let (Ok(endpoint), Ok(service_name)) = (
        dotenv::var("OTEL_EXPORTER_OTLP_ENDPOINT"),
        dotenv::var("OTEL_SERVICE_NAME"),
    ) {
        // Return None if we recieved an endpoint but it's unparseable
        let endpoint = Url::parse(&endpoint)
            .map_err(|e| eprintln!("Could not parse endpoint: {}", e))
            .ok();

        if endpoint.is_none() {
            eprintln!("Failed to parse OTEL Endpoint! OTEL logging will be skipped!");
        }

        let endpoint = endpoint?;

        let headers: HashMap<_, _> = dotenv::vars()
        .filter(|(name, _)| name.starts_with(HEADER_PREFIX))
        .map(|(name, value)| {
            let header_name = name
                .strip_prefix(HEADER_PREFIX)
                // We should never get the below error but Rust doesn't know that
                .expect("Should be able to strip the header prefix from something containing a header prefix")
                .replace('_', "-")
                .to_ascii_lowercase();
            (header_name, value)
        })
        .collect();

        // Have an example of filter_map making things LESS legible
        /*  let naively_some_function = |name: String, value: String| {(name.strip_prefix(HEADER_PREFIX).unwrap().replace("_", "-").to_ascii_lowercase(), value.to_string())};
        let headers: HashMap<_, _> = dotenv::vars().filter_map(|(name, value)|  name.starts_with(HEADER_PREFIX).then_some(naively_some_function(name, value))).collect(); */

        let pipeline = opentelemetry_otlp::new_pipeline()
            .logging()
            .with_log_config(
                Config::default().with_resource(Resource::new(vec![KeyValue::new(
                    opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                    service_name,
                )])),
            )
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .http()
                    .with_endpoint(endpoint)
                    .with_headers(headers),
            )
            .install_batch(runtime::Tokio);

        Some(pipeline)
    } else {
        eprintln!("Could not retrieve either OTEL_EXPORTER_OTLP_ENDPOINT or OTEL_SERVICE_NAME from environment variables!");
        eprintln!("Could not initialize open telemetry logging!");
        None
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    async fn can_poll_notion() {
        // TODO: Implement this so that it tests whether or not Notion's API still works the way we assume it does.
        // Tests whether we can poll notion for valid input and if there is at least 30 seconds between
        // make sleeptime configurable
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
        // TODO: Implement tests for failure handling?
        let species = get_ellen_species(Source::Notion);
        assert!(species.await.is_ok());

        let species = get_ellen_species(Source::ApiKitsuneGay);
        assert!(species.await.is_ok());
    }
    #[test]
    fn get_ellen_pronouns() {
        let ellen = Subject {
            species: None,
            pronouns: Some("She/Her".to_string()),
        };
        let epronouns = ellen.pronouns.as_deref().unwrap();

        // oh no, this will create a new vec every time...
        let pronouns: Vec<&str> = epronouns.split('/').collect();
        assert_ne!(
            pronouns.last(),
            None,
            "An invalid value was entered for pronouns"
        );

        // get a union of these
        assert_eq!(
            pronouns.len(),
            2,
            "An invalid amount was entered for the pronouns: {:#?}",
            pronouns
        );

        let pronoun = *pronouns
            .choose(&mut rand::thread_rng())
            .expect("Able to take a reference to a pronoun");

        // This is where I would put witty comments, IF I HAD ANY
        let name_and_comment: (Option<&str>, Option<&str>) = match pronoun.to_lowercase().as_str() {
        "she" | "her" => (
            Some("Ellen"),
            Some("I am woman hear me roar~ Or mow, I suppose."),
        ),

        "shi" | "hir" => (Some("Ellen"), Some("Showing your Chakat roots, I assume.")),

        "he" | "him" => (Some("Ev"), Some("Oh right, you are genderfluid!")),

        "they" | "them" => (
            Some("Ev"),
            Some("More like... Wait for it. I will calculate a joke..."),
        ),
        "ey" | "em" => (Some("Ev"), Some("A fine choice.")),
        _ => (
            Some("Ev"),
            Some("Oh! A new set of pronouns! Time to finally update the code to take live input!"),
        ),
    };
        let announcement = format!(
            "{}'s pronouns are {}, ",
            name_and_comment.0.unwrap(),
            epronouns
        );
        let comment = name_and_comment.1.unwrap();
        let message = announcement + comment;
        let map = json!({
            "content": message,
            "tts": false,
        });
        assert_eq!(
            message,
            "Ellen's pronouns are She/Her, I am woman hear me roar~ Or mow, I suppose."
        )
    }

    // TODO: Redo this test after watch_a_thing has been refactored
    // async fn watches() {
    //     let tv = match dayoftheweek.number_from_monday() {
    //         1 => Schedule::Monday.show("Static Shock"),
    //         2 => Schedule::Tuesday.show("Pokémon"),
    //         3 => Schedule::Wednesday.show("MLP Friendship is Magic"),
    //         4 => Schedule::Thursday.show("The Borrowers"),
    //         5 => Schedule::Friday.show("Sherlock Holmes in the 22nd Century"),
    //         _ => Schedule::Nothing.show("Candle Cove"),
    //     };
    //     let show = watch_a_thing(tvschedule);
    //     match dayoftheweek {
    //         Weekday::Mon => {
    //             assert_eq!(show, tv)
    //         }
    //         Weekday::Tue => {
    //             assert_eq!(show, tv)
    //         }
    //         Weekday::Wed => {
    //             assert_eq!(show, tv)
    //         }
    //         Weekday::Thu => {
    //             assert_eq!(show, tv)
    //         }
    //         Weekday::Fri => {
    //             assert_eq!(show, tv)
    //         }
    //         _ => {
    //             assert_eq!(show, tv)
    //         }
    //     }

    //TODO: Basic test just needs to check that it changes depending on the day
}
