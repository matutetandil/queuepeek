package ui

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/matutedenda/rabbitpeek/internal/config"
	"github.com/matutedenda/rabbitpeek/internal/rabbit"
)

// Layout constants
const (
	sidebarRatio    = 0.25
	minSidebarWidth = 20
	statusBarHeight = 1
	searchBarHeight = 1
	dividerWidth    = 1
)

type screen int

const (
	screenProfileSelect screen = iota
	screenMain
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

// Async messages
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
	screen     screen
	profileSel ProfileSelect
	client     *rabbit.Client
	config     *config.Config
	configPath string

	profileName string
	sidebar     Sidebar
	messages    MessagePanel
	searchBar   SearchBar
	statusBar   StatusBar
	spinner     spinner.Model

	focus   focus
	overlay overlay

	profileIdx int
	loading    bool

	// Dimensions — authoritative, set only in Update on WindowSizeMsg
	width  int
	height int
	ready  bool

	// Computed sub-dimensions, updated by updateLayout
	sidebarWidth  int
	mainWidth     int
	contentHeight int
}

func NewApp(cfg *config.Config, configPath string) App {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner

	return App{
		screen:     screenProfileSelect,
		profileSel: NewProfileSelect(cfg, configPath),
		config:     cfg,
		configPath: configPath,
		messages:   NewMessagePanel(),
		searchBar:  NewSearchBar(),
		spinner:    s,
	}
}

func (a App) Init() tea.Cmd {
	return a.spinner.Tick
}

// updateLayout computes all sub-dimensions from a.width/a.height and pushes
// them to every child component. Called ONLY from Update, never from View.
func (a *App) updateLayout() {
	if a.width == 0 || a.height == 0 {
		return
	}

	a.sidebarWidth = int(float64(a.width) * sidebarRatio)
	if a.sidebarWidth < minSidebarWidth {
		a.sidebarWidth = minSidebarWidth
	}
	a.mainWidth = a.width - a.sidebarWidth - dividerWidth
	if a.mainWidth < 1 {
		a.mainWidth = 1
	}
	a.contentHeight = a.height - statusBarHeight - searchBarHeight
	if a.contentHeight < 1 {
		a.contentHeight = 1
	}

	a.sidebar.SetSize(a.sidebarWidth, a.contentHeight)
	a.sidebar.SetFocused(a.focus == focusSidebar)
	a.messages.SetSize(a.mainWidth, a.contentHeight)
	a.messages.SetFocused(a.focus == focusMessages)
	a.searchBar.SetWidth(a.mainWidth)
	a.statusBar.SetWidth(a.width)
}

func (a App) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		a.width = msg.Width
		a.height = msg.Height
		a.ready = true
		a.profileSel.SetSize(msg.Width, msg.Height)
		a.updateLayout()
	}

	if a.screen == screenProfileSelect {
		return a.updateProfileSelect(msg)
	}
	return a.updateMain(msg)
}

func (a App) updateProfileSelect(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case profileChosenMsg:
		client, err := rabbit.NewClient(msg.profile)
		if err != nil {
			a.profileSel.formErr = fmt.Sprintf("Error: %s", err)
			return a, nil
		}
		a.client = client
		a.profileName = msg.name
		a.screen = screenMain
		a.sidebar = NewSidebar(msg.name, client.Vhost())
		a.messages = NewMessagePanel()
		a.searchBar = NewSearchBar()
		a.statusBar = NewStatusBar(msg.name)
		a.focus = focusSidebar
		a.loading = true
		a.updateLayout()
		return a, tea.Batch(a.spinner.Tick, a.loadQueues())

	case profileSavedMsg:
		if msg.err != nil {
			a.profileSel.formErr = fmt.Sprintf("Error saving: %s", msg.err)
			return a, nil
		}
		return a, nil

	case spinner.TickMsg:
		var cmd tea.Cmd
		a.spinner, cmd = a.spinner.Update(msg)
		cmds = append(cmds, cmd)
	}

	var cmd tea.Cmd
	a.profileSel, cmd = a.profileSel.Update(msg)
	if cmd != nil {
		cmds = append(cmds, cmd)
	}

	if len(cmds) > 0 {
		return a, tea.Batch(cmds...)
	}
	return a, nil
}

func (a App) updateMain(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		cmd := a.handleKey(msg)
		if cmd != nil {
			cmds = append(cmds, cmd)
		}

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

	if key == "ctrl+c" {
		return tea.Quit
	}

	// Overlay keys
	if a.overlay == overlayHelp {
		if key == "?" || key == "f1" || key == "esc" || key == "enter" {
			a.overlay = overlayNone
		}
		return nil
	}
	if a.overlay == overlayProfile {
		return a.handleProfileOverlay(key)
	}

	// Search input mode
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

	// Normal mode
	switch key {
	case "q":
		return tea.Quit

	case "?", "f1":
		a.overlay = overlayHelp

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

	case "j", "down":
		if a.focus == focusSidebar {
			a.sidebar.MoveDown()
		} else {
			a.messages.MoveDown()
		}

	case "k", "up":
		if a.focus == focusSidebar {
			a.sidebar.MoveUp()
		} else {
			a.messages.MoveUp()
		}

	case "enter":
		if a.focus == focusSidebar {
			if q := a.sidebar.SelectedQueue(); q != nil {
				a.loading = true
				return tea.Batch(a.spinner.Tick, a.loadMessages(q.Vhost, q.Name))
			}
		}

	case "r":
		if q := a.sidebar.SelectedQueue(); q != nil {
			a.loading = true
			a.statusBar.SetMessage("Reloading messages...", false)
			return tea.Batch(a.spinner.Tick, a.loadMessages(q.Vhost, q.Name))
		}

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

	case "/":
		a.focus = focusSearch
		a.searchBar.Focus()
		return textinput.Blink

	case "p":
		a.overlay = overlayProfile
		a.profileIdx = 0

	case "left", "h":
		if a.focus == focusMessages {
			a.messages.ScrollLeft()
		}

	case "right", "l":
		if a.focus == focusMessages {
			a.messages.ScrollRight()
		}

	case "+", "=":
		a.statusBar.IncreaseFetchCount()
		a.statusBar.SetMessage(fmt.Sprintf("Fetch count: %d", a.statusBar.FetchCount()), false)

	case "-":
		a.statusBar.DecreaseFetchCount()
		a.statusBar.SetMessage(fmt.Sprintf("Fetch count: %d", a.statusBar.FetchCount()), false)
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

// View renders the entire UI. No mutations happen here.
func (a App) View() string {
	// Guard: before first WindowSizeMsg
	if a.width == 0 || a.height == 0 {
		return ""
	}

	if !a.ready {
		return fmt.Sprintf("\n  %s Loading rabbitpeek...", a.spinner.View())
	}

	// Profile select screen
	if a.screen == screenProfileSelect {
		return a.profileSel.View()
	}

	// Minimum terminal size guard
	if a.width < MinTermWidth || a.height < MinTermHeight {
		msg := fmt.Sprintf(
			"Terminal too small. Minimum size: %dx%d\nCurrent: %dx%d",
			MinTermWidth, MinTermHeight, a.width, a.height)
		style := lipgloss.NewStyle().
			Foreground(ColorAccent).
			Bold(true)
		return lipgloss.Place(a.width, a.height, lipgloss.Center, lipgloss.Center,
			style.Render(msg),
			lipgloss.WithWhitespaceBackground(ColorBg))
	}

	// Main layout
	sidebarView := a.sidebar.View()

	var mainPanelView string
	if a.loading {
		loadingStyle := lipgloss.NewStyle().
			Background(ColorBg).
			Width(a.mainWidth).
			Height(a.contentHeight).
			Padding(0, 1)
		mainPanelView = loadingStyle.Render(fmt.Sprintf("\n  %s Loading...", a.spinner.View()))
	} else {
		mainPanelView = a.messages.View()
	}

	searchView := a.searchBar.View()
	rightPanel := lipgloss.JoinVertical(lipgloss.Left, mainPanelView, searchView)

	// Vertical divider
	divider := lipgloss.NewStyle().
		Width(dividerWidth).
		Foreground(ColorDivider).
		Render(strings.Repeat("|\n", a.contentHeight+searchBarHeight))

	mainView := lipgloss.JoinHorizontal(lipgloss.Top, sidebarView, divider, rightPanel)

	statusView := a.statusBar.View()
	fullView := lipgloss.JoinVertical(lipgloss.Left, mainView, statusView)

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
		{"Tab", "Switch focus sidebar <-> messages"},
		{"j/k up/dn", "Navigate lists"},
		{"Enter", "Select queue / peek messages"},
		{"r", "Reload current queue messages"},
		{"R", "Reload queue list"},
		{"/", "Focus search bar"},
		{"Esc", "Clear search / close overlay"},
		{"p", "Switch profile"},
		{"n", "Fetch messages from selected queue"},
		{"h/l lt/rt", "Horizontal scroll (message panel)"},
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
			indicator = "* "
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

func placeOverlay(x, y int, fg, bg string) string {
	bgLines := splitLines(bg)
	fgLines := splitLines(fg)

	for i, fgLine := range fgLines {
		row := y + i
		if row < 0 || row >= len(bgLines) {
			continue
		}
		bgRunes := []rune(bgLines[row])
		fgRunes := []rune(fgLine)

		var newLine []rune
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
		newLine = append(newLine, fgRunes...)
		after := x + len(fgRunes)
		if after < len(bgRunes) {
			newLine = append(newLine, bgRunes[after:]...)
		}
		bgLines[row] = string(newLine)
	}

	return strings.Join(bgLines, "\n")
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
