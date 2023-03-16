# Dolores
A bot for my personal discord server



# TO DOS
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