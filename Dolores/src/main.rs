use std::env; 

use serenity::async_trait;
use serenity::prelude::*;

use serenity::model::channel::Message;
use serenity::model::prelude::UserId;
use serenity::model::gateway::Ready;

use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{StandardFramework, CommandResult,};
use serenity::http::Http;
use serenity::utils::MessageBuilder;



#[group]
#[commands(EllenSpecies)]
#[commands(EllenGender)]

// Create commands here
struct General;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
//    here is where we do some async func message stuff


async fn message(&self, context: Context, msg: Message){

        // Refactor all of this with a match statement
  

      // You cannot just match on msg.content with a string because it's a struct field of type String
      // This is why converting it to a String works, as well as why assigning its value to a variable works 
       let responsemessage = match msg.content.as_str() {
        "What does this look like to you" => String::from("Doesn't look like much of anything to me."),
        "Mow" => String::from("Father, I think we're having Kieran for dinner tonight"),
        "Skunk Noises" => String::from("No, she is too precious to eat"),
        "Chirp" => String::from("Father, I think we're having Kauko for dinner!"),
        "Yip Yap!" => String::from("Father, I think I've found dinner. And it's coyote.")
        };
        let response = MessageBuilder::new()
        .push(responsemessage)
        .build();

    
        // TODO: below command in msg.channel_id
        // .push("Hello")
        // .push_bold_safe(&msg.author.name)
        // .push("welcome to Westworld!")
        // .build();
        // }

        // TODO: Ellen species and gender commands 


   }


   async fn ready(&self, _: Context, ready: Ready) {
    // This is where we declare everything about the Bot (avatar, music, etc.)
    // TODO: Right away we need to get the values of the channels for it to listen in 
    println!("{} is connected!", ready.user.name);
}







}


// Is Prelude supposed to handle tokio? 

#[tokio::main]
async fn main() {
    // Let's use an env file for the token and server to connect to 
    dotenv::dotenv().expect("Teddy, have you seen the .env file?");

    // Again, what declares dotenv? 
  
    
    //TODO: Disabled commands
    let disabled = vec![""].into_iter().map(|x| x.to_string()).collect();
    
    let botuid = Some(UserId(234234234));
        
    // Create framework 
    let framework = StandardFramework::new()
    .configure(|c| c.allow_dm(true).case_insensitivity(true).disabled_commands(disabled).on_mention(botuid).prefix("!"));
    // TODO: Only allow commands to work in certain channels
    let token = env::var("DISCORD_TOKEN").expect("But there was no token!");

    let http = Http::new(&token);
 
    
    let bot_id = match http.get_current_user().await {
            Ok(bot_id) => (bot_id.id),
            Err(why) => panic!("Couldn't access the bot id: {why:?}")
        };
    
    
    
    

    // Declare my intents
    let intents = GatewayIntents::non_privileged();

    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await
        .expect("And it all started with, Wyatt");

    if let Err(why) = client.start().await {
        println!("We failed, we failed to start listening for events: {:?}", why)
    }


    

    // TODO: At some point we need to tell it what server to connect to 
    
    

}

#[command]
async fn EllenSpecies() -> CommandResult {
    todo!()
}

#[command]
async fn EllenGender() -> CommandResult {
    todo!()
}
