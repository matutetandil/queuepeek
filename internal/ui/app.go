package ui

import (
	"fmt"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/matutedenda/rabbitpeek/internal/config"
	"github.com/matutedenda/rabbitpeek/internal/rabbit"
)

type focus int

const (
	focusSidebar focus = iota
	focusMessages
	focusSearch
)

type overlay int

const (
	overlayNone overlay = iota
	overlayHelp
	overlayProfile
)

// Messages
type queuesLoadedMsg struct {
	queues []rabbit.Queue
	err    error
}

type messagesLoadedMsg struct {
	messages   []rabbit.Message
	queueName  string
	totalCount int
	err        error
}

type statusMsg struct {
	text    string
	isError bool
}

type switchProfileMsg struct {
	name    string
	profile config.Profile
}

type App struct {
	client       *rabbit.Client
	config       *config.Config
	profileName  string
	sidebar      Sidebar
	messages     MessagePanel
	searchBar    SearchBar
	statusBar    StatusBar
	spinner      spinner.Model
	focus        focus
	overlay      overlay
	profileIdx   int
	loading      bool
	width        int
	height       int
	ready        bool
}

func NewApp(client *rabbit.Client, profileName string, cfg *config.Config) App {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner

	return App{
		client:      client,
		config:      cfg,
		profileName: profileName,
		sidebar:     NewSidebar(profileName, client.Vhost()),
		messages:    NewMessagePanel(),
		searchBar:   NewSearchBar(),
		statusBar:   NewStatusBar(profileName),
		spinner:     s,
		focus:       focusSidebar,
		loading:     true,
	}
}

func (a App) Init() tea.Cmd {
	return tea.Batch(
		a.spinner.Tick,
		a.loadQueues(),
	)
}

func (a App) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		cmd := a.handleKey(msg)
		if cmd != nil {
			cmds = append(cmds, cmd)
		}

	case tea.WindowSizeMsg:
		a.width = msg.Width
		a.height = msg.Height
		a.ready = true
		a.updateLayout()

	case spinner.TickMsg:
		if a.loading {
			var cmd tea.Cmd
			a.spinner, cmd = a.spinner.Update(msg)
			cmds = append(cmds, cmd)
		}

	case queuesLoadedMsg:
		a.loading = false
		if msg.err != nil {
			a.statusBar.SetMessage(fmt.Sprintf("Error: %s", msg.err), true)
		} else {
			a.sidebar.SetQueues(msg.queues)
			a.statusBar.SetMessage(fmt.Sprintf("Loaded %d queues", len(msg.queues)), false)
			// Auto-select first queue and load messages
			if q := a.sidebar.SelectedQueue(); q != nil {
				a.loading = true
				cmds = append(cmds, a.spinner.Tick, a.loadMessages(q.Vhost, q.Name))
			}
		}

	case messagesLoadedMsg:
		a.loading = false
		if msg.err != nil {
			a.statusBar.SetMessage(fmt.Sprintf("Error: %s", msg.err), true)
		} else {
			a.messages.SetMessages(msg.messages, msg.queueName, msg.totalCount)
			a.statusBar.SetMessage(fmt.Sprintf("Loaded %d messages from %s", len(msg.messages), msg.queueName), false)
		}

	case statusMsg:
		a.statusBar.SetMessage(msg.text, msg.isError)

	case switchProfileMsg:
		newClient, err := rabbit.NewClient(msg.profile)
		if err != nil {
			a.statusBar.SetMessage(fmt.Sprintf("Error switching profile: %s", err), true)
		} else {
			a.client = newClient
			a.profileName = msg.name
			a.sidebar = NewSidebar(msg.name, newClient.Vhost())
			a.messages = NewMessagePanel()
			a.statusBar.SetProfile(msg.name)
			a.updateLayout()
			a.loading = true
			a.overlay = overlayNone
			cmds = append(cmds, a.spinner.Tick, a.loadQueues())
		}
	}

	// Update search bar text input when focused
	if a.focus == focusSearch {
		var cmd tea.Cmd
		input := a.searchBar.Input()
		*input, cmd = input.Update(msg)
		if cmd != nil {
			cmds = append(cmds, cmd)
		}
		a.messages.SetSearch(a.searchBar.Value())
		if errMsg := a.messages.SearchError(); errMsg != "" {
			a.searchBar.SetError(errMsg)
		} else {
			a.searchBar.SetError("")
		}
	}

	if len(cmds) > 0 {
		return a, tea.Batch(cmds...)
	}
	return a, nil
}

func (a *App) handleKey(msg tea.KeyMsg) tea.Cmd {
	key := msg.String()

	// Global quit
	if key == "ctrl+c" {
		return tea.Quit
	}

	// Handle overlay keys first
	if a.overlay == overlayHelp {
		if key == "?" || key == "f1" || key == "esc" || key == "enter" {
			a.overlay = overlayNone
		}
		return nil
	}

	if a.overlay == overlayProfile {
		return a.handleProfileOverlay(key)
	}

	// Handle search input mode
	if a.focus == focusSearch {
		switch key {
		case "esc":
			a.searchBar.Clear()
			a.searchBar.Blur()
			a.messages.SetSearch("")
			a.focus = focusMessages
		case "enter":
			a.searchBar.Blur()
			a.focus = focusMessages
		}
		return nil
	}

	// Normal mode keys
	switch key {
	case "q":
		return tea.Quit

	case "?", "f1":
		a.overlay = overlayHelp
		return nil

	case "tab":
		if a.focus == focusSidebar {
			a.focus = focusMessages
			a.sidebar.SetFocused(false)
			a.messages.SetFocused(true)
		} else {
			a.focus = focusSidebar
			a.sidebar.SetFocused(true)
			a.messages.SetFocused(false)
		}
		return nil

	case "j", "down":
		if a.focus == focusSidebar {
			a.sidebar.MoveDown()
		} else {
			a.messages.MoveDown()
		}
		return nil

	case "k", "up":
		if a.focus == focusSidebar {
			a.sidebar.MoveUp()
		} else {
			a.messages.MoveUp()
		}
		return nil

	case "enter":
		if a.focus == focusSidebar {
			if q := a.sidebar.SelectedQueue(); q != nil {
				a.loading = true
				return tea.Batch(a.spinner.Tick, a.loadMessages(q.Vhost, q.Name))
			}
		}
		return nil

	case "r":
		if q := a.sidebar.SelectedQueue(); q != nil {
			a.loading = true
			a.statusBar.SetMessage("Reloading messages...", false)
			return tea.Batch(a.spinner.Tick, a.loadMessages(q.Vhost, q.Name))
		}
		return nil

	case "R":
		a.loading = true
		a.statusBar.SetMessage("Reloading queues...", false)
		return tea.Batch(a.spinner.Tick, a.loadQueues())

	case "n":
		if q := a.sidebar.SelectedQueue(); q != nil {
			a.loading = true
			a.statusBar.SetMessage("Fetching messages...", false)
			return tea.Batch(a.spinner.Tick, a.loadMessages(q.Vhost, q.Name))
		}
		return nil

	case "/":
		a.focus = focusSearch
		a.searchBar.Focus()
		return textinput.Blink

	case "p":
		a.overlay = overlayProfile
		a.profileIdx = 0
		return nil

	case "left", "h":
		if a.focus == focusMessages {
			a.messages.ScrollLeft()
		}
		return nil

	case "right", "l":
		if a.focus == focusMessages {
			a.messages.ScrollRight()
		}
		return nil

	case "+", "=":
		a.statusBar.IncreaseFetchCount()
		a.statusBar.SetMessage(fmt.Sprintf("Fetch count: %d", a.statusBar.FetchCount()), false)
		return nil

	case "-":
		a.statusBar.DecreaseFetchCount()
		a.statusBar.SetMessage(fmt.Sprintf("Fetch count: %d", a.statusBar.FetchCount()), false)
		return nil
	}

	return nil
}

func (a *App) handleProfileOverlay(key string) tea.Cmd {
	names := a.config.ProfileNames()
	switch key {
	case "esc":
		a.overlay = overlayNone
	case "j", "down":
		if a.profileIdx < len(names)-1 {
			a.profileIdx++
		}
	case "k", "up":
		if a.profileIdx > 0 {
			a.profileIdx--
		}
	case "enter":
		if a.profileIdx < len(names) {
			name := names[a.profileIdx]
			profile, err := a.config.GetProfile(name)
			if err == nil {
				return func() tea.Msg {
					return switchProfileMsg{name: name, profile: profile}
				}
			}
		}
	}
	return nil
}

func (a *App) updateLayout() {
	if !a.ready {
		return
	}
	sidebarWidth := a.width / 4
	if sidebarWidth < 20 {
		sidebarWidth = 20
	}
	mainWidth := a.width - sidebarWidth - 1 // 1 for border

	contentHeight := a.height - 2 // status bar + search bar

	a.sidebar.SetSize(sidebarWidth, contentHeight)
	a.sidebar.SetFocused(a.focus == focusSidebar)
	a.messages.SetSize(mainWidth, contentHeight-2) // search bar space
	a.messages.SetFocused(a.focus == focusMessages)
	a.searchBar.SetWidth(mainWidth)
	a.statusBar.SetWidth(a.width)
}

func (a App) View() string {
	if !a.ready {
		return fmt.Sprintf("\n  %s Loading rabbitpeek...", a.spinner.View())
	}

	a.updateLayout()

	sidebarView := a.sidebar.View()

	var rightPanel string
	if a.loading {
		loadingStyle := StyleMainPanel.Width(a.width - a.width/4 - 1).Height(a.height - 4)
		rightPanel = loadingStyle.Render(fmt.Sprintf("\n  %s Loading...", a.spinner.View()))
		rightPanel = lipgloss.JoinVertical(lipgloss.Left, rightPanel, a.searchBar.View())
	} else {
		rightPanel = lipgloss.JoinVertical(lipgloss.Left, a.messages.View(), a.searchBar.View())
	}

	// Join sidebar and main panel
	border := StyleBorder.Height(a.height - 2).Render(" ")
	mainView := lipgloss.JoinHorizontal(lipgloss.Top, sidebarView, border, rightPanel)

	// Add status bar
	fullView := lipgloss.JoinVertical(lipgloss.Left, mainView, a.statusBar.View())

	// Overlays
	if a.overlay == overlayHelp {
		fullView = a.renderOverlay(fullView, a.helpView())
	} else if a.overlay == overlayProfile {
		fullView = a.renderOverlay(fullView, a.profileView())
	}

	return fullView
}

func (a App) helpView() string {
	help := []struct{ key, desc string }{
		{"?/F1", "Toggle help"},
		{"Tab", "Switch focus sidebar ↔ messages"},
		{"j/k ↑/↓", "Navigate lists"},
		{"Enter", "Select queue / peek messages"},
		{"r", "Reload current queue messages"},
		{"R", "Reload queue list"},
		{"/", "Focus search bar"},
		{"Esc", "Clear search / close overlay"},
		{"p", "Switch profile"},
		{"n", "Fetch messages from selected queue"},
		{"h/l ←/→", "Horizontal scroll (message panel)"},
		{"+/-", "Increase/decrease fetch count"},
		{"q/Ctrl+C", "Quit"},
	}

	var lines string
	for _, h := range help {
		lines += fmt.Sprintf("  %s  %s\n",
			StyleHelpKey.Width(14).Render(h.key),
			StyleHelpDesc.Render(h.desc))
	}

	title := StyleMessageHeader.Render("  Keyboard Shortcuts")
	return StyleHelpOverlay.Render(title + "\n\n" + lines)
}

func (a App) profileView() string {
	names := a.config.ProfileNames()
	title := StyleMessageHeader.Render("  Select Profile")
	var lines string
	for i, name := range names {
		indicator := "  "
		if name == a.profileName {
			indicator = "● "
		}
		if i == a.profileIdx {
			lines += StyleProfileSelected.Width(30).Render(indicator+name) + "\n"
		} else {
			lines += StyleProfileItem.Render(indicator+name) + "\n"
		}
	}
	return StyleProfileOverlay.Render(title + "\n\n" + lines)
}

func (a App) renderOverlay(base, overlayContent string) string {
	overlayW := lipgloss.Width(overlayContent)
	overlayH := lipgloss.Height(overlayContent)

	x := (a.width - overlayW) / 2
	y := (a.height - overlayH) / 2

	if x < 0 {
		x = 0
	}
	if y < 0 {
		y = 0
	}

	return placeOverlay(x, y, overlayContent, base)
}

// placeOverlay places an overlay string on top of a background string at the given position.
func placeOverlay(x, y int, overlay, background string) string {
	bgLines := splitLines(background)
	olLines := splitLines(overlay)

	for i, olLine := range olLines {
		bgIdx := y + i
		if bgIdx < 0 || bgIdx >= len(bgLines) {
			continue
		}
		bgLine := bgLines[bgIdx]
		bgRunes := []rune(bgLine)
		olRunes := []rune(olLine)

		// Build the new line
		var newLine []rune
		// Prefix
		if x > 0 {
			if x <= len(bgRunes) {
				newLine = append(newLine, bgRunes[:x]...)
			} else {
				newLine = append(newLine, bgRunes...)
				for len(newLine) < x {
					newLine = append(newLine, ' ')
				}
			}
		}
		// Overlay content
		newLine = append(newLine, olRunes...)
		// Suffix
		afterOverlay := x + len(olRunes)
		if afterOverlay < len(bgRunes) {
			newLine = append(newLine, bgRunes[afterOverlay:]...)
		}

		bgLines[bgIdx] = string(newLine)
	}

	result := ""
	for i, line := range bgLines {
		result += line
		if i < len(bgLines)-1 {
			result += "\n"
		}
	}
	return result
}

func splitLines(s string) []string {
	var lines []string
	start := 0
	for i := 0; i < len(s); i++ {
		if s[i] == '\n' {
			lines = append(lines, s[start:i])
			start = i + 1
		}
	}
	lines = append(lines, s[start:])
	return lines
}

// Commands

func (a *App) loadQueues() tea.Cmd {
	client := a.client
	return func() tea.Msg {
		queues, err := client.ListQueues(client.Vhost())
		return queuesLoadedMsg{queues: queues, err: err}
	}
}

func (a *App) loadMessages(vhost, queue string) tea.Cmd {
	client := a.client
	count := a.statusBar.FetchCount()
	return func() tea.Msg {
		messages, err := client.PeekMessages(vhost, queue, count)
		if err != nil {
			return messagesLoadedMsg{err: err, queueName: queue}
		}
		return messagesLoadedMsg{
			messages:   messages,
			queueName:  queue,
			totalCount: len(messages),
			err:        nil,
		}
	}
}
