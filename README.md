# Serina
A bot for my personal discord server, and maybe for yours!

# Dev Mode 
Dev mode can be activated via supplying the appropriate feature **at compile time**.
In dev mode, Serina switches her behavior: 
* Her activity changes to a reference to [feeding maned wolves](https://twitter.com/longboieli/status/1533881489019969541?lang=en). 
* Instead of posting in the #speciesupdates channel, Serina posts in the #bottest channel.
* Multiple instances of Serina can be connected to a server at the same time, although this may change as it is entirely dependent on the Discord API. 
* Please note that Devrina will not respond to any commands issued over DM.

# TO DOS
* Set up git build hooks for CI/CD
* Merge in the changes w/ the KV store
* Ability to enable and disable commands via CLI.
    // Maybe give the CLI user a list of commands to enable and then dynamically pass in disabled ones
    // something like let disabled = vec![""].into_iter().map(|x| x.to_string()).collect();
* Command Restriction to specific channels
* Function to list commands
* All the TODOs in the code itself 
* Testing(?)
* Fix Notion Logic 
* Add in member joining logic 
* Get the whole Ellen object and just read the fields.
* Dev mode rework

# Installation
**NOTE: Although I have tested this via a devcontainer, a lot of these commands need root and wthere is a possibility that I missed something. If this is broken, please see manual install and make sure to file an Issue.**

You will need two things to install Serina.
* [Cargo Make](https://crates.io/crates/cargo-make)
* Sudo access on a linux machine which runs SystemD

_Ideally_, you will need to run one command (and input your password at the sudo prompt, sorry) to install or update serina, regardless of whether you are developing or installing in production.

**Please note that in order to _run_ serina, you will need to supply the needed environment variables to the .envfile. The installation process will only install a template. Supplying a phrases file however, is optional** 

There are two Makefile runs possible:

**As noted above, it is suggested that you create /etc/serina and supply your own `.env` file prior to running either of the following commands, so that serina.service does not instantly go into a restart loop and fail upon activation**

`cargo make production-flow` will 
* Create `/etc/serina` if the folder does not exist, along with example files
* Copy any _new_ files or directories to `/etc/serina`
* Install the serina binary to `/usr/bin/serina`
* Install (and set to start up on system boot), start, reload and then restart the `serina.service` unit file (which calls `serina -le` on launch, **logging to stderr**), _as noted `serina.service is restarted at least once during this process`_.


`cargo make development-flow` which does the following: 
* Create `/etc/serina` if the folder does not exist, along with example files
* Copy any _new_ files or directories to `/etc/serina`
* Compile serina in dev mode and install it to whatever directory `cargo install` defaults to.

# Manual Installation

In order to install Serina, you will need to do a few things. 
`sudo mkdir -p /etc/serina && touch /etc/serina/.env` 
Then, fill out the `.env` file with the following entries: 
* `BOT_USER_ID`
* `DISCORD_TOKEN`
* `NotionApiToken`
With logging turned on, the following additional entries are required:
* `OTEL_EXPORTER_OTLP_ENDPOINT`
* `OTEL_SERVICE_NAME`

Next, run the following command from the root of the folder:
`cp phrases.txt /etc/serina/`

Third, if you have never installed Serina before, you will need to do something along the lines of...
`sudo cp serina.service /etc/systemd/system/serina.service && sudo systemctl daemon-reload && sudo systemctl start serina.service && sudo systemctl enable serina`

And finally
`cd Serina && cargo build --release && sudo mv target/release/Serina /usr/bin/serina`

# Operation
Serina takes command line options that expose multiple capabilities.
These can be viewed by supplying `--help` or `-h`.
In short, these allow a user to: 
* Specify the `.env` file used
* Specify the `phrases.txt` file used (if any)
* Specify whether otel logging, stderr logging, both, or none are used.
* Serina does NOT log by default, you need to supply a logging option for Serina to log!


# Devcontainer stuff
I use a devcontainer, you can take it or leave it. Many thanks to Chuxel for the systemd dev file basis: https://github.com/Chuxel/systemd-devcontainer