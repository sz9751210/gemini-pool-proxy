package proxy

type Client interface {
	Models(apiKey string) (status int, body []byte, err error)
	Chat(apiKey string, body []byte, query string) (status int, resp []byte, err error)
	Native(apiKey, path string, body []byte, query string, method string) (status int, resp []byte, err error)
}

type NoopClient struct{}

func (n *NoopClient) Models(apiKey string) (int, []byte, error) {
	return 200, []byte(`{"object":"list","data":[]}`), nil
}

func (n *NoopClient) Chat(apiKey string, body []byte, query string) (int, []byte, error) {
	return 200, []byte(`{"id":"chatcmpl-noop"}`), nil
}

func (n *NoopClient) Native(apiKey, path string, body []byte, query string, method string) (int, []byte, error) {
	return 200, []byte(`{"ok":true}`), nil
}
