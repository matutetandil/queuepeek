package ui

import (
	"fmt"
	"regexp"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/matutedenda/rabbitpeek/internal/rabbit"
	"github.com/tidwall/pretty"
)

type MessagePanel struct {
	messages    []rabbit.Message
	filtered    []rabbit.Message
	cursor      int
	offset      int
	hScroll     int
	height      int
	width       int
	focused     bool
	queueName   string
	totalCount  int
	searchQuery string
	searchErr   string
}

func NewMessagePanel() MessagePanel {
	return MessagePanel{}
}

func (m *MessagePanel) SetMessages(msgs []rabbit.Message, queueName string, totalCount int) {
	m.messages = msgs
	m.queueName = queueName
	m.totalCount = totalCount
	m.cursor = 0
	m.offset = 0
	m.hScroll = 0
	m.applyFilter()
}

func (m *MessagePanel) SetSize(width, height int) {
	m.width = width
	m.height = height
}

func (m *MessagePanel) SetFocused(focused bool) {
	m.focused = focused
}

func (m *MessagePanel) SetSearch(query string) {
	m.searchQuery = query
	m.searchErr = ""
	m.applyFilter()
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
	if m.cursor >= len(m.filtered) {
		m.cursor = max(0, len(m.filtered)-1)
	}
	m.offset = 0
}

func (m *MessagePanel) MoveUp() {
	if m.cursor > 0 {
		m.cursor--
		if m.cursor < m.offset {
			m.offset = m.cursor
		}
	}
}

func (m *MessagePanel) MoveDown() {
	if m.cursor < len(m.filtered)-1 {
		m.cursor++
		visible := m.visibleItems()
		if m.cursor >= m.offset+visible {
			m.offset = m.cursor - visible + 1
		}
	}
}

func (m *MessagePanel) ScrollLeft() {
	if m.hScroll > 0 {
		m.hScroll -= 4
		if m.hScroll < 0 {
			m.hScroll = 0
		}
	}
}

func (m *MessagePanel) ScrollRight() {
	m.hScroll += 4
}

func (m *MessagePanel) visibleItems() int {
	// Each message takes ~6 lines, header takes 3
	available := (m.height - 3) / 6
	if available < 1 {
		return 1
	}
	return available
}

func (m *MessagePanel) SearchError() string {
	return m.searchErr
}

func (m MessagePanel) View() string {
	var b strings.Builder

	contentWidth := m.width - 4

	// Header
	header := fmt.Sprintf("  %s", m.queueName)
	if m.queueName != "" {
		header = fmt.Sprintf("  %s  %s",
			m.queueName,
			StyleQueueCount.Render(fmt.Sprintf("(%d messages)", m.totalCount)))
	}
	b.WriteString(StyleMessageHeader.Width(contentWidth).Render(header))
	b.WriteString("\n")

	if len(m.filtered) == 0 {
		if len(m.messages) == 0 {
			b.WriteString(StyleMessageMeta.Render("  No messages. Press Enter to peek messages from selected queue."))
		} else {
			b.WriteString(StyleMessageMeta.Render("  No messages match the current filter."))
		}
		return StyleMainPanel.Width(m.width).Height(m.height).Render(b.String())
	}

	b.WriteString(StyleMessageMeta.Render(fmt.Sprintf("  Showing %d of %d fetched messages", len(m.filtered), len(m.messages))))
	b.WriteString("\n\n")

	visible := m.visibleItems()
	end := m.offset + visible
	if end > len(m.filtered) {
		end = len(m.filtered)
	}

	for i := m.offset; i < end; i++ {
		msg := m.filtered[i]

		// Index line
		indexStr := StyleMessageIndex.Render(fmt.Sprintf("#%d", msg.Index))
		metaParts := []string{indexStr}

		if !msg.Timestamp.IsZero() {
			metaParts = append(metaParts, StyleMessageMeta.Render(msg.Timestamp.Format("2006-01-02 15:04:05")))
		}
		if msg.RoutingKey != "" {
			metaParts = append(metaParts, StyleMessageMeta.Render(fmt.Sprintf("key=%s", msg.RoutingKey)))
		}
		if msg.Exchange != "" {
			metaParts = append(metaParts, StyleMessageMeta.Render(fmt.Sprintf("exchange=%s", msg.Exchange)))
		}
		if msg.Redelivered {
			metaParts = append(metaParts, lipgloss.NewStyle().Foreground(ColorAccent).Render("redelivered"))
		}

		b.WriteString("  " + strings.Join(metaParts, "  "))
		b.WriteString("\n")

		// Body
		bodyStr := formatBody(msg.BodyString, contentWidth-4, m.hScroll)

		if i == m.cursor && m.focused {
			bodyStyle := lipgloss.NewStyle().
				Background(lipgloss.Color("#252729")).
				Foreground(ColorPrimary).
				Width(contentWidth - 2).
				Padding(0, 1)
			b.WriteString(bodyStyle.Render(bodyStr))
		} else {
			b.WriteString(StyleMessageBody.Padding(0, 1).Render(bodyStr))
		}

		if i < end-1 {
			b.WriteString("\n")
			b.WriteString(StyleMessageMeta.Render(strings.Repeat("─", contentWidth-2)))
			b.WriteString("\n")
		}
	}

	return StyleMainPanel.Width(m.width).Height(m.height).Render(b.String())
}

func formatBody(body string, maxWidth, hScroll int) string {
	// Try to pretty print JSON
	trimmed := strings.TrimSpace(body)
	if (strings.HasPrefix(trimmed, "{") || strings.HasPrefix(trimmed, "[")) {
		prettyJSON := pretty.Pretty([]byte(trimmed))
		if len(prettyJSON) > 0 {
			body = string(prettyJSON)
		}
	}

	lines := strings.Split(body, "\n")
	var result []string
	for _, line := range lines {
		if hScroll > 0 && len(line) > hScroll {
			line = line[hScroll:]
		} else if hScroll > 0 {
			line = ""
		}
		if maxWidth > 0 && len(line) > maxWidth {
			line = line[:maxWidth]
		}
		result = append(result, line)
	}

	// Limit the number of body lines shown
	if len(result) > 12 {
		result = append(result[:12], StyleMessageMeta.Render(fmt.Sprintf("  ... (%d more lines)", len(result)-12)))
	}

	return strings.Join(result, "\n")
}
