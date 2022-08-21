package notion

import (
	"context"
	"encoding/json"

	"github.com/jomei/notionapi"
)

var (
	client *notionapi.Client
)

func main() {
	client = notionapi.NewClient("somenotiontoken")

}

type FoxInfo interface {
	GetSpecies() string
	GetPronouns() string
}

type response struct {
	Results    []string `json:"results"`
	Heading_1  string   `json:"heading_1"`
	Text       []string `json:"text"`
	Plain_text string   `json:"plain_text"`
}

type Fox struct {
	species, pronouns string
}

func (f Fox) GetSpecies() string {
	// and make sure you're putting in
	// TODO: Make it so that these block IDs are read in
	blockid := "REDACTED"
	page, err := client.Page.Get(context.Background(), "REDACTED")
	somejson := page.Object

	// okay so at this point we have the json object

	str := `{"results": [foo, heading_1: {"text" : ["plain_text"] } ] }`
	// I think str should actually be somejson.str
	// adn then &somejson should be like, &somestruct
	//ala somestruct = {responsestruct }
	//https://gobyexample.com/json
	//in that example how does json.Unmarshal take an interface?
	json.Unmarshal([]byte(str), &somejson)
	return somejson
}

// Okay so we should be passing this a &Fox though
//Just so we're all clear

func (f Fox) GetPronouns() string {
	blockid := "REDACTED"
	page, err := client.Page.Get(context.Background(), notionapi.PageID(blockid))
	somejson := page.Object.String()
	// awesome so we have the json string
	res := response{}
	json.Unmarshal([]byte(somejson), &res)
	// Aha! I was right!
	return ""
}

func UpdateFrontingAlter() {
	blockid := "REDACTED"
	client.Page.Update(notionapi.PageUpdateRequest{Properties: []string{"heading_1: text", "text: type", "type: text", "text: content", "content: alter"}}
	})
	// if this doesn't work we could just use like, do them in order or something 
	// okay so it looks like i have to do something with json marshalling 
	// client.Page.Update(context.Background(), notionapi.PageID(blockid), &notionapi.PageUpdateRequest{Properties:})
	
}

// Okay so we should be passing this a &Fox though
//Just so we're all clear
