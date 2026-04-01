package ui

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/matutedenda/rabbitpeek/internal/rabbit"
)

type Sidebar struct {
	queues  []rabbit.Queue
	cursor  int
	offset  int
	height  int
	width   int
	focused bool
	vhost   string
	profile string
}

func NewSidebar(profile, vhost string) Sidebar {
	return Sidebar{
		profile: profile,
		vhost:   vhost,
		focused: true,
	}
}

func (s *Sidebar) SetQueues(queues []rabbit.Queue) {
	s.queues = queues
	if s.cursor >= len(queues) {
		s.cursor = max(0, len(queues)-1)
	}
}

func (s *Sidebar) SetSize(width, height int) {
	s.width = width
	s.height = height
	// Clamp scroll offset after resize
	vis := s.visibleItems()
	if s.offset > 0 && s.offset+vis > len(s.queues) {
		s.offset = max(0, len(s.queues)-vis)
	}
}

func (s *Sidebar) SetFocused(focused bool) {
	s.focused = focused
}

func (s *Sidebar) MoveUp() {
	if s.cursor > 0 {
		s.cursor--
		if s.cursor < s.offset {
			s.offset = s.cursor
		}
	}
}

func (s *Sidebar) MoveDown() {
	if s.cursor < len(s.queues)-1 {
		s.cursor++
		vis := s.visibleItems()
		if s.cursor >= s.offset+vis {
			s.offset = s.cursor - vis + 1
		}
	}
}

func (s *Sidebar) SelectedQueue() *rabbit.Queue {
	if len(s.queues) == 0 || s.cursor >= len(s.queues) {
		return nil
	}
	return &s.queues[s.cursor]
}

func (s *Sidebar) visibleItems() int {
	// header (1) + vhost (1) + blank (1) = 3 lines of chrome
	available := s.height - 3
	if available < 1 {
		return 1
	}
	return available
}

func (s Sidebar) View() string {
	if s.width == 0 || s.height == 0 {
		return ""
	}

	var b strings.Builder

	innerW := s.width - 2 // 1 padding each side
	if innerW < 1 {
		innerW = 1
	}

	// Header
	b.WriteString(StyleSidebarHeader.Width(innerW).Render(fmt.Sprintf(" %s", s.profile)))
	b.WriteString("\n")

	b.WriteString(lipgloss.NewStyle().Foreground(ColorMuted).Render(fmt.Sprintf(" vhost: %s", s.vhost)))
	b.WriteString("\n\n")

	if len(s.queues) == 0 {
		b.WriteString(lipgloss.NewStyle().Foreground(ColorMuted).Render(" No queues found"))

		style := lipgloss.NewStyle().
			Background(ColorSidebarBg).
			Width(s.width).
			Height(s.height)
		return style.Render(b.String())
	}

	visible := s.visibleItems()
	end := s.offset + visible
	if end > len(s.queues) {
		end = len(s.queues)
	}

	for i := s.offset; i < end; i++ {
		q := s.queues[i]
		countStr := formatCount(q.Messages)
		name := truncateStr(q.Name, innerW-8)
		line := fmt.Sprintf("%s %s", name, countStr)

		if i == s.cursor && s.focused {
			b.WriteString(StyleQueueItemSelected.Width(innerW).Render(line))
		} else if i == s.cursor {
			b.WriteString(StyleQueueItem.Foreground(ColorWhite).Width(innerW).Render(line))
		} else {
			b.WriteString(StyleQueueItem.Width(innerW).Render(line))
		}

		if i < end-1 {
			b.WriteString("\n")
		}
	}

	style := lipgloss.NewStyle().
		Background(ColorSidebarBg).
		Width(s.width).
		Height(s.height).
		Padding(0, 1)

	return style.Render(b.String())
}

func formatCount(count int) string {
	s := fmt.Sprintf("(%d)", count)
	switch {
	case count == 0:
		return StyleQueueCountZero.Render(s)
	case count > 1000:
		return StyleQueueCountHigh.Render(s)
	default:
		return StyleQueueCount.Render(s)
	}
}

func truncateStr(s string, maxLen int) string {
	if maxLen <= 0 {
		return ""
	}
	if len(s) <= maxLen {
		return s
	}
	if maxLen <= 3 {
		return s[:maxLen]
	}
	return s[:maxLen-3] + "..."
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
