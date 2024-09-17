// TODO: COMMIT

// TODO: ADD IN WAIT TIME TO LOOP OTHERWISE YOU WILL CONSTANTLY BOMBARD THE CPU (you're going to need to use a channel for this)
// TODO: ALSO FIX FAILING 503s
// TODO: Also more error checking with pronouns, like have it wait a bit
// TODO: ALso dynamic phrases loadin

// TODO specify a folder for config
// TODO one day do autocomplete. One day. https://docs.rs/clap_complete/latest/clap_complete/dynamic/shells/enum.CompleteCommand.html

use std::borrow::Borrow;
//Standard
use std::collections::HashMap;
use std::fmt::Debug;

use std::fmt::Display;
use std::future::Future;
use std::ops::Deref;
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::thread::spawn;
use std::time::Duration;
use std::{default, env, fmt, task};

use clap::builder::ArgPredicate;
use futures_core::FusedFuture;
use futures_util::future::err;
// use futures_util::sink::Send;
use indoc::{eprintdoc, printdoc};

// Notion
use notion::ids::BlockId;
use notion::NotionApi;

use opentelemetry_sdk::logs::Config;
use opentelemetry_sdk::runtime;
//Serenity
use serenity::client::bridge::gateway::ShardManager;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::json::JsonMap;
use serenity::model::channel::Message;
use serenity::model::gateway::Activity;
use serenity::model::gateway::Ready;
use serenity::model::prelude::{ChannelId, GuildChannel, UserId};
use serenity::prelude::*;
use serenity::utils::MessageBuilder;
use serenity::{async_trait, FutureExt};

//AsyncandConcurrency
use async_stream::try_stream;
use futures_core::stream::{self, Stream};
use futures_util::pin_mut;
use futures_util::stream::StreamExt;
use strum::EnumString;
use tokio::runtime::Handle;
use tokio::select;
use tokio::sync::oneshot::{self, Sender};
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio::try_join;
use uncased::UncasedStr;

//Honeycomb
// TD: read in the honeycomb token as a static or constant variable
use opentelemetry::logs::LogError;
use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    logs::{self as sdklogs},
    Resource,
};

use tracing::{debug, error, info, trace, trace_span, warn};
use tracing_subscriber::prelude::*;

// CLI
use clap::{Args, Parser};

use futures_util::future::{join_all, try_join_all};
use std::error::Error as RegularError;
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
        write!(f, "{}", "test")
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

// impl From<FatalError> for &TaskErrors {
//     fn from(err: FatalError) -> Self {
//         Self::FatalError(err)
//     }
// }

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

//Misc
use anyhow::{anyhow, Error, Result};
use chrono::prelude::*;
use rand::seq::SliceRandom;
use reqwest::header::ACCEPT;
use reqwest::Url;
use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

// Constants
mod constants;
use constants::CONFIG_PATH;
use constants::HEADER_PREFIX;
use constants::PHRASES_CONFIG_PATH;

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

#[derive(Clone, Debug, PartialEq, PartialOrd)]
struct Subject {
    // when the program has just started up it is possible for these values to be None
    species: Option<String>,
    pronouns: Option<String>,
}

// This is where we declare the typemapkeys that actually "transport" the values
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
        let span = trace_span!(target: "messagemap", "message");
        let responsemessage = span.in_scope(|| {

        trace!(name: "messagesmap", "Messages lock obtained!");
        /* Dereference messages to get through the MutexGuard, then take a reference since we can't move the content */
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

                    // Return None if not a key phrase
                    _ => None,
                }
            }
        };
        responsemessage
    });

        if let Some(rm) = responsemessage {
            let response = MessageBuilder::new().push(rm).build();
            // let responseref = &response.clone();

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

    ///Logic that executes when the "Ready" event is received. The most important thing here is the logic for posting my species
    /// The other most important thing here is that things actually die if they fail, otherwise the bot will be left in a zombie state.
    async fn ready(&self, ctx: Context, mut ready: Ready) -> () {
        // TODO: Unpack
        info!(name: "ready", username=%ready.user.name, "{} is connected!", &ready.user.name);

        // If Serina's username is not Serina, change it. If that fails, call quit to kill the bot.

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
                    quit(&ctx).await;
                    return;
                }
            }
        }

        // TODO: Add in Avatar Change when in DevMode

        // Defunct Avatar Logic

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

        // My assumption is that once the await concludes we will continue to execute code
        // The problem with awaiting a future in your current function is that once you've done that you are suspending execution until the future is complete
        // As a result, if watch_westworld never finishes nothing else will be done from this function. period. ever.
        // Except that is no longer true here, because we do watch_westworld in its own task

        info!(name: "ready", "Made it past the initial setup");

        /*  Introducing the stream */

        info!(name: "poll_notion", "poll_notion streams created!");

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

        //if it does not exist then create it
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

        // THE MONSTER

        //TO TRY:
        // JOIN SET
        // STRUCTS
        // INDEPENDENT MANAGER THREADS
        // JUST DONT CARE

        // WHAT I HAVE TRIED:

        // CURRENTLY TRYING
        // Shared  FUTURES BUT SPAWNING NEW ONES

        // Initial Run
        // Create shared futures, restarts counter, unwrap updates_channel_id
        let mut restarts = 0;
        let mut not_a_wh_restart = true;
        let updates_channel_id = specieschannelid.unwrap();

        // info!(name:"ready", "made it to res");
        // let res = tokio::try_join!(
        //     async(watcher_join_handle),
        //     flatten(pronouns_join_handle),
        //     flatten(species_join_handle)
        // );
        // match res {
        //     Ok((first, second, third)) => {
        //         info!(name:"ready", "made it after res {:?} {:?} {:?}", first, second, third);
        //     }
        //     Err(err) => {
        //         info!(name: "ready", "processing failed; error = {}", err);
        //     }
        // };
        // info!(name:"ready", "made it after res");

        // Spawn two streams that operate asynchronously

        info!(name:"ready", "made it inside");
        let watcher_manager = tokio::spawn({
            let ctx = ctx.clone();
            async move {
                loop {
                    let watcher_handle = tokio::spawn(watch_westworld(ctx.clone(), None)).await;
                    info!(name:"watcher_task", "Watcher encountered an error and was respawned!!");
                    match watcher_handle {
                        Ok(Err(output)) => match output {
                            TaskErrors::FatalError(_) => break,
                            TaskErrors::NonFatalError(_) => continue,
                        },
                        Ok(Ok(_)) => break,

                        Err(err) if err.is_panic() => continue,
                        Err(_) => break,
                    }
                }
            }
        });

        let species_manager = tokio::spawn({
            let ctx = ctx.clone();
            async move {
                loop {
                    if restarts == 5 {
                        eprintln!("Max restarts reached!");
                        break;
                    }
                    let species_handle =
                        tokio::spawn(species_worker(ctx.clone(), updates_channel_id)).await;
                    info!(name:"species_handle", "Species encountered an error and died!");
                    match species_handle {
                        Ok(Err(output)) => match output {
                            TaskErrors::FatalError(e) => {
                                error!(name:"species_task", "Species loop encountered a fatal error and will not be restarted!");
                                // Err::<(), FatalError>(e).unwrap();
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
                return Err::<(), anyhow::Error>(anyhow!("Managerial thread died!"));
                // TODO: Return later and change this to take an error from iniside the loop
            }
        });

        let pronouns_manager = tokio::spawn({
            let ctx = ctx.clone();
            async move {
                loop {
                    if restarts == 5 {
                        eprintln!("Max restarts reached!");
                        break;
                    }
                    let pronouns_handle =
                        tokio::spawn(pronouns_worker(ctx.clone(), updates_channel_id)).await;
                    info!(name:"pronouns_handle", "Pronouns encountered an error and died!");
                    match pronouns_handle {
                        Ok(Err(output)) => match output {
                            TaskErrors::FatalError(e) => {
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
                return Err::<(), anyhow::Error>(anyhow!("Managerial thread died!"));
                // TODO: Return later and change this to take an error from iniside the loop
            }
        });

        let result = try_join!(async { pronouns_manager.await? }, async {
            species_manager.await?
        },);

        println!("Irrecoverable error!");

        quit(&ctx.clone()).await;
    }
}

// Trying to convert a joinhandle of any error (or maybe anyhow error?) to a result of something implementing an error
// https://stackoverflow.com/questions/71751269/how-to-convert-anyhow-error-to-std-error
// they're literally calling it on it
async fn flatten<T: Into<TaskErrors>>(handle: JoinHandle<Result<(), T>>) -> Result<(), TaskErrors> {
    match handle.await {
        Ok(Ok(result)) => Ok(()),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(FatalError::new("Some Join Errors"))?,
    }
}

// async fn flatten_ref<T: Into<TaskErrors>>(
//     handle: JoinHandle<Result<(), &T>>,
// ) -> Result<(), TaskErrors> {
//     match handle.await {
//         Ok(Ok(result)) => Ok(()),
//         Ok(Err(err)) => Err(err.into()),
//         Err(err) => Err(FatalError::new("Some Join Errors"))?,
//     }
// }

// fn start_shared_worker<F>(worker: WorkerType, context: &Context, handle: F) -> JoinHandle<()>
// where
//     F: Future + Send + 'static,
//     F::Output: Send + 'static,
// {
//     // match worker {
//     //     WorkerType::watcher => {
//     let ctx = context.clone();
//     // let handle = async move {
//     //     watch_westworld(&ctx, None).await;
//     // };
//     let worker = tokio::spawn(handle);
//     return worker;
//     // } // WorkerType::pronounsloop => {
//     //     let pronouns = async move {
//     //         info!("Starting pronouns loop");
//     //         pronounsloop(
//     //             &context,
//     //             specieschannelid.expect("A valid updates channel ID"),
//     //             pronouns_stream,
//     //         )
//     //         .await;
//     //     };

//     //     return pronouns;
//     // }
//     // WorkerType::speciesloop => {
//     //     return worker;
//     // }
// }
// // }

enum WorkerType {
    watcher,
    speciesloop,
    pronounsloop,
}

async fn quit(ctx: &Context) {
    let mut data_read = ctx.data.write().await;
    //nobody can update the species while i'm reading it
    // hypothetically, this could deadlock, maybe?
    let oneshot_tx_arc = {
        data_read
            .remove::<OneShotContainer>()
            .expect("Expected SendContainer in TypeMap.")
    };

    // closed.await
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
        let _ = oneshot_tx.unwrap().send(string.clone());
        // Specifically quit by sending on tx
    }

    // TODO: Shut down this

    // Using `let _ =` to ignore send errors.
}

async fn speciesloop<S: Stream<Item = Result<String>>>(
    ctx: &Context,
    specieschannelid: u64,
    species_stream: S,
) -> Result<()> {
    info!(name: "speciesstream", "Species stream started!");
    // info!(name:"send_species", arc_count=%Arc::strong_count(&species), "Starting species loop!");
    info!(name:"speciesstream",  "Starting species loop!");
    // TODO: Need to unpack the Ellen species
    // TODO: Fallback to speciesstream

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
        // pass back errors that the stream generatedb n00
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

    //     match species {
    //         Ok(transformation) => {
    //             // If this is the first time we are doing this species will be None
    //             // In this case, set the species to whatever we get
    //             // We then have to continue the loop, probably because Rust doesn't know that NOTHING will be modifying
    //             // the value of species inbetween is_none and is_some
    //             if ellen.species.is_none() {
    //                 warn!(
    //                     name: "speciesstream",
    //                     species=?transformation,
    //                     "Setting initial species to {:#?}, This should only happen once!",
    //                     &transformation
    //                 );
    //                 // TODO: We should look for the last species and if it's the same not send an update
    //                 ellen.species = Some(transformation);
    //                 continue;
    //             };

    //             if ellen.species.is_some() {
    //                 /* If the new item from the stream is the same as the current lastspecies,
    //                   Then we know nothing has changed and we should continue the loop
    //                 */
    //                 if ellen.species.as_ref().expect("Species has a value") == &transformation {
    //                     trace!(name: "speciesstream", species=?transformation, "Species remained the same.");
    //                     continue;
    //                 } else {
    //                     // However, if it is different, then update current species
    //                     // trace!("Species updated! {}", &transformation);
    //                     ellen.species = Some(transformation);
    //                 }
    //             }

    //             send_species(ctx, specieschannelid);
    //         }
    //         Err(e) => {
    //             //TODO: We can go ahead and restart here if we hit a certain number of errors
    //             // TODO: This doesn't make sense we will never hit error
    //             error!(name: "speciesstream", "Could not set species");
    //             error!(name: "speciesstream", error_text=?e, "Species loop failure: {:#?}", &e);
    //             return Err(anyhow!(e));
    //         }
    //     };
    // }
    // Ok(())
}

async fn send_species(ctx: &Context, specieschannelid: u64) {
    // Get the current species
    let data_read: tokio::sync::RwLockReadGuard<'_, TypeMap> = ctx.data.read().await;
    let ellen_lock = data_read
        .get::<SubjectContainer>()
        .expect("Expected ResponseMessageContainer in TypeMap.")
        .clone();

    let mut ellen = ellen_lock.lock().await;

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
    info!(name:"ready", "Starting species loop");
    if let Err(e) = speciesloop(&ctx, updates_channel_id, species_stream).await {
        error!(name:"ready", error_text=?e, "Species loop failed: {:#?}", e);
        let fa = FatalError::new(&e.to_string());
        return Err(fa)?;
    }
    Ok(())
}

// It would be WAY more readable to just go ahead and do both species and pronouns in one function but I want to get this up as soon as possible
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
        // This is where I would put witty comments, IF I HAD ANY
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

    let pronouns: Vec<&str> = epronouns.split('/').collect();
    // oh no, this will create a new vec every time...
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
        "{}'s pronouns are {:#?}.\n",
        name_and_comment.0.unwrap(),
        pronouns
    );
    let comment = name_and_comment.1.unwrap();
    let message = announcement + comment;
    let map = json!({
        "content": message,
        "tts": false,
    });
    match ctx.http.send_message(updates_channel_id, &map).await {
        Ok(_) => {
            info!(name: "pronouns_loop", "Pronouns update sent to updates channel!");
        }
        Err(e) => {
            error!(name: "pronouns_loop", error_text=?e, "We couldn't send the pronouns update to the channel: {:#?}", e);
        }
    }
}
// TODO: Pronouns map
// let map = json!({
//     "content": content,
//     "tts": false,
//     });

//    Err(e) => {
//        println!("Could not set pronouns");
//        return Err(anyhow!(e));

//    }

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
//         error!("Boss, there's a guest who's not supposed to be in Westworld, looks like they're wanted for: {:?}", why);
//     };
// }
// }

#[tokio::main]
async fn main() -> Result<()> {
    // Initializing the global Ellen object
    // Sets the global loggerprovider

    // Get the global loggerprovider here
    // Okay that is cool, how is that done?!
    // Arc gets passed around

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
                        // If I ever add support for more than just honeycomb I'd impl a struct or enum
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
                        // If this ever supported more than honeycomb I'd impl a struct or enum

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

    // change to % to get non string version

    // Get response messages , if this fails return None
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
                Ok(result) => {
                    // one of our loops failed
                    error!(name: "main", "Irrecoverable Notion Failure");
                    error!(name: "main", "An error occured while running Notion!");
                    warn!(name: "main", "Program will now exit!")
                }
                Err(e) => {
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
    // TODO: Implement custom debug method with newtype in order to print configuration
    // Get the token
    let token = env::var("DISCORD_TOKEN").expect("A valid token");
    trace!(name: "discord", "Token obtained!");
    // Declare intents (these determine what events the bot will receive )
    let intents =
        GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS;
    // info!("Intents are {:?}", intents);
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
//TODO: REFACTOR ALL BELOW!
// Also, what happens if ellenspecis or ellenpronouns fail the reply? Does the whole thing crash? If not, need to log

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
    // let species = match response {
    //     Ok(r) => {
    //         let species = match r.error_for_status() {
    //             Ok(r) => r.text().await.expect("A valid species string"),

    //             Err(e) => {
    //                 error!(
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
    info!(name: "ellenspecies", species=%species, "species {}", species);
    let content = format!("Ellen is a {}", species);
    // let content = content.as_str();
    let response = MessageBuilder::new().push(&content).build();
    // let response = response.as_str();
    msg.reply(ctx, &response).await?;
    debug!(name: "ellenspecies", content=?content, response=?response, "content: {:#?}, response: {:#?}", &content, response);
    info!(name: "ellenspecies", "Ellen species delivered to requestor");
    // ev.send(&mut honeycomb_client);
    // match ev.send(&mut honeycomb_client) {
    //     Ok(()) => {
    //         let response = honeycomb_client.responses().iter().next().unwrap();
    //         let respstring = response.status_code.unwrap();
    //         println!("{}", respstring.as_str());
    //         assert_eq!(response.error, None);
    //     }
    //     Err(e) => {
    //         println!("Could not send event: {}", e);
    //     }
    // }
    // honeycomb_client.flush();
    // honeycomb_client.close();

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

async fn watch_westworld(ctx: Context, fetch_duration: Option<Duration>) -> Result<(), TaskErrors> {
    //TODO: We need a dev mode off switch
    // TODO: Change other attributes here to distuinguish devlores

    let watching_schedule = {
        Schedule::TV {
            monday: "A particularly interesting anomaly".to_string(),
            tuesday: "The digital data flow that makes up my existence".to_string(),
            wednesday: "A snow leopard, of course.".to_string(),
            thursday: "London by night, circa 1963".to_string(),
            friday: "Everything and nothing.".to_string(),
            default: Some("the stars pass by...".to_string()),
        }
    };

    loop {
        let thing = watch_a_thing(watching_schedule.clone());
        if cfg!(feature = "dev") {
            ctx.set_activity(Activity::watching(
                "🎶🎶🎶Snepgirl on a leash, you can feed her treats!🎶🎶🎶",
            ))
            .await;
            info!(name: "watch_westworld", "Dev Mode Watching activity set!");
            debug!(name: "watch_westworld", "Watch Westworld sleeping for 60 seconds");
            if let Some(d) = fetch_duration {
                tokio::time::sleep(Duration::from_secs(d.as_secs())).await;
            } else {
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        } else {
            ctx.set_activity(Activity::watching(thing)).await;
            info!(name: "watch_westworld", "Watching activity set!");
        }
        // TODO FIX BUG
        // match fetch_duration {
        //     Some(d) => {
        //         debug!(name: "watch_westworld", "Watch Westworld sleeping for {:#?} seconds", d);
        //         tokio::time::sleep(Duration::from_secs(d.as_secs())).await;
        //     }
        //     None => {
        //         debug!(name: "watch_westworld", "Watch Westworld sleeping for 84600 seconds");
        //         tokio::time::sleep(Duration::from_secs(86400)).await;
        //     }
        // }
    }
    return Err(FatalError::default())?;
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
            // change to % in order to get non string version
            debug!(name: "get_ellen_pronouns", "Pronouns returned");
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
            error!(name: "get_ellen_pronouns", "ApiKitsuneGay was used as a source for pronouns! This function is not yet implemented!");
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
/// Poll notion every 30 seconds grabbing a characteristic
///
fn poll_notion(
    characteristic: Characteristics,
    mut sleep_time: Option<Duration>,
) -> impl Stream<Item = Result<String>> {
    //TODO: Consider reducing this... Want to get things as fast as possible
    if sleep_time.is_none() {
        sleep_time = Some(Duration::from_secs(30));
    }
    let sleep_time = sleep_time.unwrap();

    try_stream! {
        // TODO: Why in the hell does this not actually return errors?!?!?
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
    // TODO: give different messages depending on whether phrases_config is default
    if phrases_config
        .to_str()
        .expect("Able to convert phrases_config from a path to a string")
        == PHRASES_CONFIG_PATH
    {
        println!("Using default phrases config path {}", PHRASES_CONFIG_PATH)
    } else {
        println!("Using custom phrases config path {:?}", phrases_config)
    }
    // TODO: Need an entire thing from before where if the phrases config path does not exist, we return a NONE
    if phrases_config.is_file() {
        info!(name: "get_phrases", "Reading lines in");
        if let Ok(phrases) = read_lines(phrases_config) {
            info!(name: "get_phrases", "Read lines in!");
            // Originally I used filter map but it turns out you can get an unlimited string of errors from filter_map if it's acting on Lines
            let phrases: Vec<String> = phrases.map_while(Result::ok).collect();
            debug!(name: "get_phrases", phrases=?phrases, "phrases {:#?}", phrases);
            // this consumes the iterator, as rust-by-example notes
            // although this should also be obvious imo
            // also welcome back to Rust, where you are forced to handle your errors
            // This is why it is so hard to just unwrap the result, because you're not handling your errors if you do that

            // If there are any messages, create a hash map from them
            if !phrases.is_empty() {
                info!(name: "get_phrases", "Parsing phrases");
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
        monday: String,
        tuesday: String,
        wednesday: String,
        thursday: String,
        friday: String,
        default: Option<String>,
    },
    Nothing {
        message: String,
    },
}

fn watch_a_thing(schedule: Schedule) -> String {
    impl fmt::Display for Schedule {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{:?}", self)
            // or, alternatively:
            // fmt::Debug::fmt(self, f)
        }
    }
    let now: DateTime<Local> = chrono::offset::Local::now();
    let dayoftheweek = now.date_naive().weekday();
    debug!(name: "watch_a_thing", day=%dayoftheweek, "Day of week: {}", dayoftheweek);

    match schedule {
        Schedule::TV {
            monday,
            tuesday,
            wednesday,
            thursday,
            friday,
            default,
        } => {
            let string_default = default.unwrap_or(String::from("Westworld"));
            let show = match dayoftheweek.number_from_monday() {
                1 => monday,
                2 => tuesday,
                3 => wednesday,
                4 => thursday,
                5 => friday,
                _ => string_default,
            };
            debug!(name: "watch_a_thing", show=?show, "Today's show is {:#?}", show);
            show
        }
        Schedule::Nothing { message } => message,
    }
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
        // let naively_some_function = |name: String, value: String| {(name.strip_prefix(HEADER_PREFIX).unwrap().replace("_", "-").to_ascii_lowercase(), value.to_string())};
        // let headers: HashMap<_, _> = dotenv::vars().filter_map(|(name, value)|  name.starts_with(HEADER_PREFIX).then_some(naively_some_function(name, value))).collect();

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
        assert!(species.await.is_ok());

        let species = get_ellen_species(Source::ApiKitsuneGay);
        assert!(species.await.is_ok());
    }

    async fn watches() {
        let monday_tv = "Static Shock".to_string();
        let tuesday_tv = "Pokémon".to_string();
        let wednesday_tv = "MLP Friendship is Magic".to_string();
        let thursday_tv = "The Borrowers".to_string();
        let friday_tv = "Sherlock Holmes in the 22nd Century".to_string();
        let spooky_tv = "Candle Cove".to_string();

        let tvschedule = {
            Schedule::TV {
                monday: monday_tv.clone(),
                tuesday: tuesday_tv.clone(),
                wednesday: wednesday_tv.clone(),
                thursday: thursday_tv.clone(),
                friday: friday_tv.clone(),
                default: Some(spooky_tv.clone()),
            }
        };
        let now: DateTime<Local> = chrono::offset::Local::now();
        let dayoftheweek = now.date_naive().weekday();
        let show = watch_a_thing(tvschedule);
        match dayoftheweek {
            Weekday::Mon => {
                assert_eq!(show, monday_tv)
            }
            Weekday::Tue => {
                assert_eq!(show, tuesday_tv)
            }
            Weekday::Wed => {
                assert_eq!(show, wednesday_tv)
            }
            Weekday::Thu => {
                assert_eq!(show, thursday_tv)
            }
            Weekday::Fri => {
                assert_eq!(show, friday_tv)
            }
            _ => {
                assert_eq!(show, spooky_tv)
            }
        }

        //TODO: Basic test just needs to check that it changes depending on the day
    }
}
