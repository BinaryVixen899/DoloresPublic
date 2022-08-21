package messages

import (
	"fmt"
	"strings"
	"time"

	"github.com/bwmarrin/discordgo"
)

// var (
// 	integerOptionMinValue = 1.0

// 	commands = []*discordgo.ApplicationCommand{
// 		{
// 			Name:        "EllenSpecies",
// 			Description: "Tells you what species Ellen is!",
// 		},
// 		{
// 			Name:        "EllenGender",
// 			Description: "Tells you what gender Ellen is",
// 		},
// 	}

// 	commandHandlers = map[string]func(s *discordgo.Session, i *discordgo.InteractionCreate){
// 		"EllenSpecies": func(s *discordgo.Session, i *discordgo.InteractionCreate) {
// 			species := "Kitsune"
// 			// Here is where you can call the other command
// 			message := fmt.Sprintf("Hmm, I believe Ellen is a %species", species)
// 			s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
// 				Type: discordgo.InteractionResponseChannelMessageWithSource,
// 				Data: &discordgo.InteractionResponseData{
// 					Content: message,
// 				},
// 			})
// 		},
// 		"EllenGender": func(s *discordgo.Session, i *discordgo.InteractionCreate) {
// 			pronouns := "She/Her"
// 			// Here is where you can call the other command
// 			message := fmt.Sprintf("Hmm, it looks like Ellen uses %pronouns right now.", pronouns)
// 			s.InteractionRespond(i.Interaction, &discordgo.InteractionResponse{
// 				Type: discordgo.InteractionResponseChannelMessageWithSource,
// 				Data: &discordgo.InteractionResponseData{
// 					Content: message,
// 				},
// 			})
// 		},
// 	}
// )

// func NewMemberJoin() {

// }

// func StartupMessages() {
// 	t := time.Now()
// 	if t.Weekday() == 0 {
// 	}

// }

// func messageCreate (s *discordgo.Session, m *discordgo.MessageCreate)
// {
// 	for _, v := range m.Mentions{
// 		if v.ID == doloresid   {
// 			m.Author.ID
// 			message = DoloresSundayCheck()
// 			dolores.ChannelMessageSend(generalchannelid ,message)

// 		}

// 	// basically we check mentions in every message to see if dolores was @'d
// 	// If she was, we respond, if not we don't care
// 	//This allows us to do @ messages

func DoloresSundayCheck() (string, bool) {
	t := time.Now().Weekday()
	if t == 0 {
		response := "I'm sorry, but I'm doing the Lord's work today."
		return response, true
	}
	// put a rescue statement here?

	return "", false

}

func DoloresConvoRoutines(m *discordgo.MessageCreate, doloresid string) string {

	for _, v := range m.Mentions {
		if v.ID == doloresid {
			message, care := DoloresSundayCheck()

			if message == "" && care == false {
				switch {
				case m.Content == "What does this look like to you?":
					message = "Doesn't look like much of anything to me."
				case m.Content == "Hi" || m.Content == "Howdy" || m.Content == "Hey":
					message = fmt.Sprintf("Hello %name, welcome to Westworld", m.Author.Username)
				case strings.Contains(m.Content, "Mow"):
					message = "Father, I think we're having Kieran for dinner tonight"
				case strings.Contains(m.Content, "Woof"):
					message = "Father, I think we're having Savannah for dinner tonight."
				case strings.Contains(m.Content, "Skunk Noises"):
					message = "No, she is too precious to eat."
				case strings.Contains(m.Content, "Chirp"):
					message = "Father, I think we're having Kauko for dinner!"
				case strings.Contains(m.Content, "Yip Yap!"):
					message = "Father, I think I've found dinner. And it's coyote!"
					// You know this isn't the best way to do the animal noises...
				default:
					message = "I'm sorry. I don't know what you're trying to say to me. Teddy, do you have any ideas?"

				}

			}
			return message

		}

	}
	return ""

}

// External API calls

// func EllenSpecies() {

// }

// func EllenPronouns() {

// }
