package ferries

type Client struct {
	apiKey string
}

func NewClient(apiKey string) Client {
	return Client{
		apiKey: apiKey,
	}
}
