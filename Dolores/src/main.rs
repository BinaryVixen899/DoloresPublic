use core::time;
use std::collections::HashMap;
use std::env;
use std::process::exit;
use std::thread::sleep_ms;

use libhoney::FieldHolder;
use libhoney::Value;
use libhoney::transmission::Transmission;
use reqwest::header::ACCEPT;

use serenity::Client;
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
// TD: read in the honeycomb token as a static or constant variable 



#[group]
#[commands(ellengender, ellenspecies, fronting)]
struct Ellen;

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
        // TODO: Lock this behind a key of some kind
        // TODO: Random musical lyrics
         let content = MessageBuilder::new()
            .push("🎶🎶🎶But I'm alive, I'm alive, I am so alive, ")
            .push("And I feed on the fear that's behind your eyes, ")
            .push("And I need yout o need me it's no surprise, ")
            .push("I'm aliveeeeeeeee🎶🎶🎶")
            .build();
        let message = ChannelId(511743033117638658);
    if let Err(why) = message.send_message(&ctx, |m| m.content(content)).await {
        eprintln!("I'M MELTING, MELTIIIIIIING {:?}", why);
        exit(1)
    };
    }

    // TODO: Check if this crashes things
    async fn reaction_add(&self, _ctx: Context, _add_reaction: Reaction) {
        // When someone new joins, if an admin adds a check reaction bring them in and give them roles 
        // If an admin does an X reaction, kick them out 
        todo!()
    }

    // TODO: Does this actually work? 
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
    
        // Starting Honeycomb
        let mut honeycombclient = libhoney::init(libhoney::Config{
            options: libhoney::client::Options {
              api_key: env::var("HONEYCOMB_API_KEY").unwrap().to_string(),
              dataset: "dolorestest".to_string(),
              ..libhoney::client::Options::default()
            },
            transmission_options: libhoney::transmission::Options::default(),
          });
          

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
    })
    .group(&ELLEN_GROUP);
    // TODO: Only allow commands to work in certain channels
    let token = env::var("DISCORD_TOKEN").expect("But there was no token!");



    // Declare my intents
    // TODO: Edit these, they determine what events the bot will be notifed about
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("And it all started with, Wyatt");


    if let Err(why) = client.start().await {
        println!(
            "We failed, we failed to start listening for events: {:?}",
            why
        );
        let mut data: HashMap<String, Value> = HashMap::new();
        // I wonder if we will have an issue with why. Is it consumed? 
        data.insert("error".to_string(), Value::String(why.to_string()));
        let mut ev = honeycombclient.new_event();
        ev.add(data);
        ev.send(&mut honeycombclient).err(); 
        
    }
    honeycombclient.flush();
    honeycombclient.close();

}


#[command]
async fn ellenspecies(ctx: &Context, msg: &Message) -> CommandResult {
    // let mut honeycombclient = libhoney::init(libhoney::Config{
    //     options: libhoney::client::Options {
    //         api_key: env::var("HONEYCOMB_API_KEY").unwrap().to_string(),
    //       dataset: "Dolores".to_string(),
    //       ..libhoney::client::Options::default()
    //     },
    //     transmission_options: libhoney::transmission::Options::default(),
    //   });
    
    // let mut data: HashMap<String, Value> = HashMap::new();    
    // let mut ev = honeycombclient.new_event();
    // data.insert("msg_id".to_string(), Value::String(msg.id.to_string()));
    // data.insert("channel_id".to_string(), Value::String(msg.channel_id.to_string()));
    // data.insert("command_type".to_string(), Value::String("ellenspecies".to_string()));
    

    // println!("I have received the command.");
    // let client = reqwest::Client::new();
    // let result = client
    //     .get("https://api.kitsune.gay/Fronting")
    //     .header(ACCEPT, "application/json")
    //     .send()
    //     .await;
    
    // // TODO: Look into the idiomatic way to do this. Rusty way?
    // // ugh fine  
    // let mut speciestext = String::from("Kitsune");
    // if let Ok(response) = result {
    //     match response.error_for_status() {
    //         Ok(res) => (
    //             speciestext = res.text().await.unwrap()
    //         ),
    //         // TODO: Change this before production so that we aren't accidentally giving debug info away 
    //         Err(err) => (println!("{:?}", err.status())),
    //     }
        

    // }
    
    // else { 
    //     println!("We either got an incorrect response or no response!")
    //     // Return something witty from dolores about my species 
    // }        
    // let species = speciestext;
    // data.insert("ellen_species".to_string(), Value::String(species.to_string()));
    // ev.add(data);
    // println!("I have gotten the {}", species);
    // let cntnt = format!("Ellen is a {}", species);
    // let response = MessageBuilder::new().push(cntnt).build();
    // msg.reply(ctx, response).await?;
    // // ev.send(&mut honeycombclient); 
    // // honeycombclient.close();
    // match ev.send(&mut honeycombclient) {
    //     Ok(()) => {
    //         let response = honeycombclient.responses().iter().next().unwrap();
    //         let respstring = response.status_code.unwrap();
    //         println!("{}", respstring.as_str());
    //         assert_eq!(response.error, None);
    //     }
    //     Err(e) => {
    //         println!("Could not send event: {}", e);
    //     }
    // }
    // honeycombclient.flush();
    // honeycombclient.close();
    todo!();
    Ok(())
}

#[command]
async fn ellengender() -> CommandResult {
    // let mut honeycombclient = libhoney::init(libhoney::Config{
    //     options: libhoney::client::Options {
    //       api_key: env::var("HONEYCOMB_API_KEY").unwrap().to_string(),
    //       dataset: "Dolores".to_string(),
    //       ..libhoney::client::Options::default()
    //     },
    //     transmission_options: libhoney::transmission::Options::default(),
    //   });

    println!("I have received the command.");
    todo!()
}

#[command]
async fn fronting(ctx: &Context, msg: &Message) -> CommandResult {
    let mut honeycombclient = libhoney::init(libhoney::Config{
        options: libhoney::client::Options {
          api_key: env::var("HONEYCOMB_API_KEY").unwrap().to_string(),
          dataset: "Dolores".to_string(),
          ..libhoney::client::Options::default()
        },
        transmission_options: libhoney::transmission::Options::default(),
      });

    let mut data: HashMap<String, Value> = HashMap::new();    
    let mut ev = honeycombclient.new_event();
    data.insert("msg_id".to_string(), Value::String(msg.id.to_string()));
    data.insert("channel_id".to_string(), Value::String(msg.channel_id.to_string()));
    data.insert("command_type".to_string(), Value::String("fronting".to_string()));
    
    
    let client = reqwest::Client::new();
    let result = client
        .get("https://api.kitsune.gay/Fronting")
        .header(ACCEPT, "application/json")
        .send()
        .await;
    
    let mut frontingtext = String::from("Kitsune");
    if let Ok(response) = result {
        match response.error_for_status() {
            Ok(res) => (
                frontingtext = res.text().await.unwrap()
            ),
            // TODO: Change this before production so that we aren't accidentally giving debug info away 
            Err(err) => (println!("{:?}", err.status())),
        }
        

    }
    
    else { 
        println!("We either got an incorrect response or no response!")
        // Return something witty from dolores about my species 
    }        
    let alter = frontingtext;
    data.insert("ellen_species".to_string(), Value::String(alter.to_string()));
    ev.add(data);
    println!("I have gotten the {}", alter);
    let cntnt = format!("{} is in front", alter);
    let response = MessageBuilder::new().push(cntnt).build();
    msg.reply(ctx, response).await?;
    ev.send(&mut honeycombclient); 
    match ev.send(&mut honeycombclient) {
        Ok(()) => {
            let response = honeycombclient.responses().iter().next().unwrap();
            let respstring = response.status_code.unwrap();
            println!("{}", respstring.as_str());
            assert_eq!(response.error, None);
        }
        Err(e) => {
            println!("Could not send event: {}", e);
        }
    }
    honeycombclient.flush();
    honeycombclient.close();
    
    Ok(())
}

async fn watch_westworld(ctx: &Context) {
    ctx.set_activity(Activity::watching("Westworld")).await;
    // TODO: Make what Dolores activity is change every day

}
