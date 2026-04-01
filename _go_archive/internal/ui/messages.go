package ui

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/charmbracelet/bubbles/viewport"
	"github.com/charmbracelet/lipgloss"
	"github.com/matutedenda/rabbitpeek/internal/rabbit"
	"github.com/tidwall/pretty"
)

type MessagePanel struct {
	viewport    viewport.Model
	messages    []rabbit.Message
	filtered    []rabbit.Message
	width       int
	height      int
	queueName   string
	totalCount  int
	searchQuery string
	searchErr   string
	ready       bool
}

func NewMessagePanel() MessagePanel {
	return MessagePanel{}
}

func (m *MessagePanel) SetMessages(msgs []rabbit.Message, queueName string, totalCount int) {
	m.messages = msgs
	m.queueName = queueName
	m.totalCount = totalCount
	m.applyFilter()
	m.renderContent()
	m.viewport.GotoTop()
}

func (m *MessagePanel) SetSize(width, height int) {
	m.width = width
	m.height = height
	if !m.ready {
		m.viewport = viewport.New(width, height)
		m.viewport.MouseWheelEnabled = true
		m.ready = true
	} else {
		m.viewport.Width = width
		m.viewport.Height = height
	}
	m.renderContent()
}

func (m *MessagePanel) SetFocused(_ bool) {
	// No-op, viewport handles its own focus
}

func (m *MessagePanel) SetSearch(query string) {
	m.searchQuery = query
	m.searchErr = ""
	m.applyFilter()
	m.renderContent()
	m.viewport.GotoTop()
}

func (m *MessagePanel) applyFilter() {
	if m.searchQuery == "" {
		m.filtered = m.messages
		m.searchErr = ""
		return
	}

	re, err := regexp.Compile("(?i)" + m.searchQuery)
	if err != nil {
		m.searchErr = "invalid regex"
		m.filtered = m.messages
		return
	}

	m.searchErr = ""
	var result []rabbit.Message
	for _, msg := range m.messages {
		if re.MatchString(msg.BodyString) || re.MatchString(msg.RoutingKey) || re.MatchString(msg.Exchange) {
			result = append(result, msg)
		}
	}
	m.filtered = result
}

func (m *MessagePanel) SearchError() string {
	return m.searchErr
}

func (m *MessagePanel) Viewport() *viewport.Model {
	return &m.viewport
}

func (m *MessagePanel) renderContent() {
	if m.width == 0 {
		return
	}

	var b strings.Builder
	contentWidth := m.width - 4
	if contentWidth < 1 {
		contentWidth = 1
	}

	if len(m.filtered) == 0 {
		if len(m.messages) == 0 {
			b.WriteString(StyleMessageMeta.Render("  No messages. Select a queue and press Enter to peek."))
		} else {
			b.WriteString(StyleMessageMeta.Render("  No messages match the current filter."))
		}
		m.viewport.SetContent(b.String())
		return
	}

	header := StyleMessageMeta.Render(fmt.Sprintf("  Showing %d of %d messages", len(m.filtered), len(m.messages)))
	b.WriteString(header)
	b.WriteString("\n\n")

	for i, msg := range m.filtered {
		// Meta line
		indexStr := StyleMessageIndex.Render(fmt.Sprintf("#%d", msg.Index))
		metaParts := []string{indexStr}

		if !msg.Timestamp.IsZero() {
			metaParts = append(metaParts, StyleMessageMeta.Render(msg.Timestamp.Format("2006-01-02 15:04:05")))
		}
		if msg.RoutingKey != "" {
			metaParts = append(metaParts, StyleMessageMeta.Render(fmt.Sprintf("key=%s", msg.RoutingKey)))
		}
		if msg.Exchange != "" {
			metaParts = append(metaParts, StyleMessageMeta.Render(fmt.Sprintf("ex=%s", msg.Exchange)))
		}
		if msg.Redelivered {
			metaParts = append(metaParts, lipgloss.NewStyle().Foreground(ColorAccent).Render("redelivered"))
		}

		b.WriteString("  " + strings.Join(metaParts, "  "))
		b.WriteString("\n")

		// Body (compact: max 3 lines)
		bodyStr := formatBody(msg.BodyString, contentWidth-4)
		b.WriteString(StyleMessageBody.Padding(0, 2).Render(bodyStr))

		if i < len(m.filtered)-1 {
			b.WriteString("\n")
			b.WriteString(StyleMessageMeta.Render("  " + strings.Repeat("─", contentWidth-4)))
			b.WriteString("\n")
		}
	}

	m.viewport.SetContent(b.String())
}

func (m MessagePanel) View() string {
	if m.width == 0 || m.height == 0 {
		return ""
	}
	return m.viewport.View()
}

func formatBody(body string, maxWidth int) string {
	trimmed := strings.TrimSpace(body)
	if strings.HasPrefix(trimmed, "{") || strings.HasPrefix(trimmed, "[") {
		prettyJSON := pretty.Pretty([]byte(trimmed))
		if len(prettyJSON) > 0 {
			body = string(prettyJSON)
		}
	}

	lines := strings.Split(strings.TrimRight(body, "\n"), "\n")
	var result []string
	for _, line := range lines {
		if maxWidth > 0 && len(line) > maxWidth {
			line = line[:maxWidth]
		}
		result = append(result, line)
	}

	if len(result) > 3 {
		result = append(result[:3], StyleMessageMeta.Render(fmt.Sprintf("... (%d more lines)", len(result)-3)))
	}

	return strings.Join(result, "\n")
}
