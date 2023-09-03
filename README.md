# Dolores
A bot for my personal discord server

# Dev Mode 
When the dev feature is activated, Dolores switches her behavior. 
Instead of posting in the #speciesupdates channel, Dolores posts in the #bottest channel.
Additionally, Dolores sets her activity to "Watching Westworld (1973)"
Multiple instances of Dolores can be connected to a server at the same time, although this may change as it is entirely dependent on the Discord API. 
Please note that Devlores will not respond for any commands issued over DM.

# TO DOS
* Set up git build hooks
* Get a functioning version running without the Notion Calls 
* I really, really want to make this multithreaded
*  FIX LOGGER ISSUES 
* Disabled commands
    // Maybe give the CLI user a list of commands to enable and then dynamically pass in disabled ones
    // let disabled = vec![""].into_iter().map(|x| x.to_string()).collect();
    // this won't work but we're definitely onto something
* Something to make it so certain commands only work in certain channels
* Function to list commands
* All the TODOs in the code itself 


Testing
Case Insensitivity 
Move "is this different than the last" logic to poll_notion and ditch the value there if it's not 
Add in member joining logic 
Change from "ellen.species" to just having "ellen" and getting the individual fields (or making the fields mutices)