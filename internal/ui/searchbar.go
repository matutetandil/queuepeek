package ui

import (
	"github.com/charmbracelet/bubbles/textinput"
	"github.com/charmbracelet/lipgloss"
)

type SearchBar struct {
	input   textinput.Model
	active  bool
	width   int
	errMsg  string
}

func NewSearchBar() SearchBar {
	ti := textinput.New()
	ti.Placeholder = "Search messages (regex)..."
	ti.CharLimit = 256
	ti.Prompt = "/ "
	ti.PromptStyle = StyleSearchLabel
	ti.TextStyle = lipgloss.NewStyle().Foreground(ColorPrimary)
	ti.PlaceholderStyle = lipgloss.NewStyle().Foreground(ColorMuted)

	return SearchBar{
		input: ti,
	}
}

func (s *SearchBar) SetWidth(width int) {
	s.width = width
	s.input.Width = width - 6
}

func (s *SearchBar) Focus() {
	s.active = true
	s.input.Focus()
}

func (s *SearchBar) Blur() {
	s.active = false
	s.input.Blur()
}

func (s *SearchBar) Clear() {
	s.input.SetValue("")
	s.errMsg = ""
}

func (s *SearchBar) SetError(msg string) {
	s.errMsg = msg
}

func (s *SearchBar) Value() string {
	return s.input.Value()
}

func (s *SearchBar) IsActive() bool {
	return s.active
}

func (s *SearchBar) Input() *textinput.Model {
	return &s.input
}

func (s SearchBar) View() string {
	inputView := s.input.View()

	content := StyleSearchBar.Width(s.width).Render(inputView)

	if s.errMsg != "" {
		content += "\n" + StyleSearchError.Render("  "+s.errMsg)
	}

	return content
}
