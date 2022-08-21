package pluralkit

import "github.com/starshine-sys/pkgo"

var (
	pk *pkgo.Session
)

func main() {
	pk = pkgo.New("wrwerwer")
	sysID := "sysid"
}

func GetCurrentFronter() string {
	front, err := pk.Fronters(sysID)
	// err nil statement goes here
	amember := front.Members[0]
	return amember.Name
}
