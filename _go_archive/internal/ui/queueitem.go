package ui

import (
	"fmt"

	"github.com/matutedenda/rabbitpeek/internal/rabbit"
)

// QueueItem wraps a rabbit.Queue to implement bubbles/list interfaces.
type QueueItem struct {
	queue rabbit.Queue
}

func (q QueueItem) Title() string {
	return q.queue.Name
}

func (q QueueItem) Description() string {
	return fmt.Sprintf("%d msgs, %d consumers, %s", q.queue.Messages, q.queue.Consumers, q.queue.State)
}

func (q QueueItem) FilterValue() string {
	return q.queue.Name
}

func (q QueueItem) Queue() rabbit.Queue {
	return q.queue
}
