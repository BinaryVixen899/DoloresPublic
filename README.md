# Serina
A bot for my personal discord server, and maybe for yours!

# Dev Mode 
Dev mode can be activated via supplying the appropriate feature at compile time.
In dev mode, Serina switches her behavior: 
* Her activity changes to a reference to [feeding maned wolves](https://twitter.com/longboieli/status/1533881489019969541?lang=en). 
* Instead of posting in the #speciesupdates channel, Serina posts in the #bottest channel.
* Multiple instances of Serina can be connected to a server at the same time, although this may change as it is entirely dependent on the Discord API. 
* Please note that Devrina will not respond to any commands issued over DM.

# TO DOS
* Set up git build hooks for CI/CD
* Merge in the changes w/ the KV store
* Is this multithreaded? If not, it should be. 
* Proper observability
* Ability to enable and disable commands via CLI.
    // Maybe give the CLI user a list of commands to enable and then dynamically pass in disabled ones
    // something like let disabled = vec![""].into_iter().map(|x| x.to_string()).collect();
* Command Restriction to specific channels
* Function to list commands
* All the TODOs in the code itself 
* Why in the hell is dev mode a compile time option? Change that.
* Testing(?)
* Fix Notion Logic 
* Add in member joining logic 
* Get the whole Ellen object and just read the fields.

# Installation

In order to install Serina, you will need to do a few things. 
`sudo mkdir -p /etc/serina && touch /etc/serina/.env` 
Then, fill out the `.env` file with the following entries: 
* `BOT_USER_ID`
* `DISCORD_TOKEN`
* `NotionApiToken`

Next, run the following command from the root of the folder:
`cp phrases.txt /etc/serina/`

Third, if you have never installed Serina before, you will need to do something along the lines of...
`sudo cp serina.service /etc/systemd/system/serina.service && sudo systemctl daemon-reload && sudo systemctl start serina.service && sudo systemctl enable serina`

And finally
`cd Serina && cargo build --release && sudo mv target/release/Serina /usr/bin/serina`
