package main

import (
	"fmt"
	"os"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

type model struct{ w, h int }

func (m model) Init() tea.Cmd { return nil }

func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		m.w, m.h = msg.Width, msg.Height
	case tea.KeyMsg:
		if msg.String() == "q" {
			return m, tea.Quit
		}
	}
	return m, nil
}

func (m model) View() string {
	if m.w == 0 {
		return ""
	}
	return lipgloss.NewStyle().
		Background(lipgloss.Color("#1A1D21")).
		Foreground(lipgloss.Color("#ECB22E")).
		Width(m.w).
		Height(m.h).
		Align(lipgloss.Center, lipgloss.Center).
		Render(fmt.Sprintf("Background test\n%dx%d\nPress q to quit", m.w, m.h))
}

func main() {
	p := tea.NewProgram(model{}, tea.WithAltScreen())
	if _, err := p.Run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
