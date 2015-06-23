package ferries

import (
	"fmt"
	"net/http"
	"time"
)

const BaseUrl = "http://www.wsdot.wa.gov/ferries/api/schedule/rest"

type Client struct {
	apiKey string
}

func NewClient(apiKey string) Client {
	return Client{
		apiKey: apiKey,
	}
}

type Terminal struct {
	Description string
	TerminalId  int
}

func (c Client) Terminals(time.Time) (error, []Terminal) {

	url := fmt.Sprintf("%s/terminals/%d-%d-%d?apiaccesstoken=%s", BaseUrl, 2015, 6, 22, c.apiKey)
	_, err := http.Get(url)
	if err != nil {
		return err, []Terminal{}
	}
	return nil, []Terminal{}
}
