package main

import (
	"Dolores/internal/messages"
	"flag"
	"fmt"
	"time"

	"github.com/bwmarrin/discordgo"
)

var (
	AuthToken        string
	ServerID         string
	GeneralChannelID string
)

// Make sure to giver  peroper accredence to discordgo examples
// Global variable

// Not sure why we have init here
func init() {
	flag.StringVar(&AuthToken, "t", "", "Token")
	flag.StringVar(&ServerID, "s", "", "Server to connect to")
	flag.Parse()
	// you have to have this
}

func main() {
	// notion.UpdateFrontingAlter()

	if AuthToken == "" {
		fmt.Println("No token provided. Please run: dolores -t <bot token>")
		return
	}

	dolores, err := discordgo.New("Bot " + AuthToken)
	if err != nil {
		fmt.Println("Error creating Discord session: ", err)
		return
		// Returning to exit out of the program entirely, presumably
	}

	dolores.AddHandler(ready)
	dolores.AddHandler(messageCreate)
	//Right, we need to handle errors here

}

func ready(s *discordgo.Session, e *discordgo.Ready) {
	var generalchannelid string

	username := "Dolores Abernathy"
	avatar := "./assets/dolores.jpg"
	music := "Wicked Games (Ramin Djawadi)"
	time := time.Now().Weekday()
	//dolores.Identify.Intents = discordgo.IntentGuilds

	// generalchannelid
	channels, err := s.GuildChannels(ServerID)

	if err != nil {
		fmt.Println("Could not find channels", err)
		return
	}

	for _, v := range channels {
		if v.Name == "general" {
			generalchannelid = v.ID
		}
	}

	if generalchannelid == "" {
		fmt.Println("No general channel ID could be found")
		return
	}

	s.UserUpdate(username, avatar)
	if err != nil {
		fmt.Println("Could not update avatar or username")
	}
	s.UpdateListeningStatus(music)
	if err != nil {
		fmt.Println("Could not update music")
	}

	if time == 3 {
		wakeupmessage := "I feel like I just had the strangest dream..."
		s.ChannelMessageSend(generalchannelid, wakeupmessage)
		// strings are pointers underneath so don't worry about them
	}
	// log messages go here

}

func messageCreate(s *discordgo.Session, m *discordgo.MessageCreate) {
	doloresuser, err := s.User("Dolores")
	if err != nil {
		fmt.Println("Could not find Dolores' user ID ")
	}
	doloresid := doloresuser.ID
	//this is what's being called back on message create
	message := messages.DoloresConvoRoutines(m, doloresid)
	// I'm guessing we don't have to call this with & because it takes a pointer
	s.ChannelMessageSend(m.ChannelID, message)
}

// func GetAvatar() string {
// 	f, err := os.Open("./prod/dolores.jpg")
// 	if err != nil {
// 		return err.Error()
// 	}
// 	defer f.Close()
// 	return f.WriteString("filestring")

// }
