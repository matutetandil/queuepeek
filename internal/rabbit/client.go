package rabbit

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"time"

	"github.com/matutedenda/rabbitpeek/internal/config"
)

type Queue struct {
	Name      string `json:"name"`
	Messages  int    `json:"messages"`
	Consumers int    `json:"consumers"`
	State     string `json:"state"`
	Vhost     string `json:"vhost"`
}

type Message struct {
	Index       int
	RoutingKey  string
	Exchange    string
	Redelivered bool
	Timestamp   time.Time
	ContentType string
	Body        []byte
	BodyString  string
}

type Client struct {
	baseURL    string
	username   string
	password   string
	vhost      string
	httpClient *http.Client
}

func NewClient(profile config.Profile) (*Client, error) {
	transport := &http.Transport{}

	tlsCfg, err := profile.TLSConfig()
	if err != nil {
		return nil, fmt.Errorf("configuring TLS: %w", err)
	}
	if tlsCfg != nil {
		transport.TLSClientConfig = tlsCfg
	}

	return &Client{
		baseURL:  profile.BaseURL(),
		username: profile.Username,
		password: profile.Password,
		vhost:    profile.Vhost,
		httpClient: &http.Client{
			Transport: transport,
			Timeout:   15 * time.Second,
		},
	}, nil
}

func (c *Client) doRequest(method, path string, body io.Reader) (*http.Response, error) {
	reqURL := c.baseURL + path
	req, err := http.NewRequest(method, reqURL, body)
	if err != nil {
		return nil, err
	}
	req.SetBasicAuth(c.username, c.password)
	req.Header.Set("Content-Type", "application/json")
	return c.httpClient.Do(req)
}

func (c *Client) Vhost() string {
	return c.vhost
}

type vhostResponse struct {
	Name string `json:"name"`
}

func (c *Client) ListVhosts() ([]string, error) {
	resp, err := c.doRequest("GET", "/api/vhosts", nil)
	if err != nil {
		return nil, fmt.Errorf("listing vhosts: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("listing vhosts: HTTP %d", resp.StatusCode)
	}

	var vhosts []vhostResponse
	if err := json.NewDecoder(resp.Body).Decode(&vhosts); err != nil {
		return nil, fmt.Errorf("decoding vhosts: %w", err)
	}

	names := make([]string, len(vhosts))
	for i, v := range vhosts {
		names[i] = v.Name
	}
	return names, nil
}

func (c *Client) ListQueues(vhost string) ([]Queue, error) {
	encodedVhost := url.PathEscape(vhost)
	path := fmt.Sprintf("/api/queues/%s", encodedVhost)

	resp, err := c.doRequest("GET", path, nil)
	if err != nil {
		return nil, fmt.Errorf("listing queues: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("listing queues: HTTP %d", resp.StatusCode)
	}

	var queues []Queue
	if err := json.NewDecoder(resp.Body).Decode(&queues); err != nil {
		return nil, fmt.Errorf("decoding queues: %w", err)
	}
	return queues, nil
}

type peekRequest struct {
	Count    int    `json:"count"`
	AckMode  string `json:"ackmode"`
	Encoding string `json:"encoding"`
	Truncate int    `json:"truncate"`
}

type peekResponse struct {
	PayloadBytes    int    `json:"payload_bytes"`
	Redelivered     bool   `json:"redelivered"`
	Exchange        string `json:"exchange"`
	RoutingKey      string `json:"routing_key"`
	MessageCount    int    `json:"message_count"`
	Payload         string `json:"payload"`
	PayloadEncoding string `json:"payload_encoding"`
	Properties      struct {
		ContentType string `json:"content_type"`
		Timestamp   int64  `json:"timestamp"`
	} `json:"properties"`
}

func (c *Client) PeekMessages(vhost, queue string, count int) ([]Message, error) {
	encodedVhost := url.PathEscape(vhost)
	encodedQueue := url.PathEscape(queue)
	path := fmt.Sprintf("/api/queues/%s/%s/get", encodedVhost, encodedQueue)

	reqBody := peekRequest{
		Count:    count,
		AckMode:  "ack_requeue_true",
		Encoding: "auto",
		Truncate: 50000,
	}

	bodyBytes, err := json.Marshal(reqBody)
	if err != nil {
		return nil, fmt.Errorf("marshaling peek request: %w", err)
	}

	resp, err := c.doRequest("POST", path, bytes.NewReader(bodyBytes))
	if err != nil {
		return nil, fmt.Errorf("peeking messages: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("peeking messages: HTTP %d", resp.StatusCode)
	}

	var peekResp []peekResponse
	if err := json.NewDecoder(resp.Body).Decode(&peekResp); err != nil {
		return nil, fmt.Errorf("decoding messages: %w", err)
	}

	messages := make([]Message, len(peekResp))
	for i, pr := range peekResp {
		body := []byte(pr.Payload)

		var ts time.Time
		if pr.Properties.Timestamp > 0 {
			ts = time.Unix(pr.Properties.Timestamp, 0)
		}

		bodyStr := pr.Payload

		messages[i] = Message{
			Index:       i + 1,
			RoutingKey:  pr.RoutingKey,
			Exchange:    pr.Exchange,
			Redelivered: pr.Redelivered,
			Timestamp:   ts,
			ContentType: pr.Properties.ContentType,
			Body:        body,
			BodyString:  bodyStr,
		}
	}

	return messages, nil
}
