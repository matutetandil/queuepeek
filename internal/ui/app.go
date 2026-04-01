package ui

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/help"
	"github.com/charmbracelet/bubbles/list"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/matutedenda/rabbitpeek/internal/config"
	"github.com/matutedenda/rabbitpeek/internal/rabbit"
)

type screen int

const (
	screenProfileSelect screen = iota
	screenMain
)

type panel int

const (
	panelQueues panel = iota
	panelMessages
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
	vhost  string
	err    error
}

type messagesLoadedMsg struct {
	messages   []rabbit.Message
	queueName  string
	totalCount int
	err        error
}

type vhostsLoadedMsg struct {
	vhosts []string
	err    error
}

type clusterInfoMsg struct {
	clusterName string
	err         error
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
	clusterName string

	// Vhost state
	vhosts        []string
	selectedVhost string

	// UI components
	queueList  list.Model
	messages   MessagePanel
	helpModel  help.Model
	spinner    spinner.Model

	allQueues  []rabbit.Queue
	activePanel panel
	overlay     overlay
	profileIdx  int

	loading    bool
	fetchCount int

	width  int
	height int
	ready  bool
}

func NewApp(cfg *config.Config, configPath string) App {
	s := spinner.New()
	s.Spinner = spinner.Dot
	s.Style = StyleSpinner

	h := help.New()

	return App{
		screen:     screenProfileSelect,
		profileSel: NewProfileSelect(cfg, configPath),
		config:     cfg,
		configPath: configPath,
		helpModel:  h,
		spinner:    s,
		fetchCount: 50,
	}
}

func (a App) Init() tea.Cmd {
	return a.spinner.Tick
}

func (a *App) initMainScreen(profileName string) {
	a.profileName = profileName
	a.screen = screenMain
	a.activePanel = panelQueues

	// Queue list with built-in filtering
	delegate := list.NewDefaultDelegate()
	delegate.Styles.SelectedTitle = delegate.Styles.SelectedTitle.
		Foreground(ColorAccent).
		BorderLeftForeground(ColorAccent)
	delegate.Styles.SelectedDesc = delegate.Styles.SelectedDesc.
		Foreground(ColorMuted).
		BorderLeftForeground(ColorAccent)

	a.queueList = list.New(nil, delegate, 0, 0)
	a.queueList.Title = "Queues"
	a.queueList.SetShowHelp(false)
	a.queueList.SetShowStatusBar(true)
	a.queueList.SetFilteringEnabled(true)
	a.queueList.Styles.Title = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true).
		Padding(0, 1)
	a.queueList.Styles.FilterPrompt = lipgloss.NewStyle().Foreground(ColorAccent)
	a.queueList.Styles.FilterCursor = lipgloss.NewStyle().Foreground(ColorAccent)

	a.messages = NewMessagePanel()
	a.helpModel = help.New()
}

func (a *App) updateLayout() {
	if a.width == 0 || a.height == 0 {
		return
	}

	// Header: 1 line, Help footer: 1 line
	contentH := a.height - 2
	if contentH < 1 {
		contentH = 1
	}

	// Split: left panel (queues) ~30%, right panel (messages) ~70%
	leftW := a.width * 3 / 10
	if leftW < 25 {
		leftW = 25
	}
	rightW := a.width - leftW - 1 // 1 for divider
	if rightW < 20 {
		rightW = 20
	}

	a.queueList.SetSize(leftW, contentH)
	a.messages.SetSize(rightW, contentH)

	a.helpModel.Width = a.width
}

func (a App) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		a.width = msg.Width
		a.height = msg.Height
		a.ready = true
		a.profileSel.SetSize(msg.Width, msg.Height)
		if a.screen == screenMain {
			a.updateLayout()
		}
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
		a.initMainScreen(msg.name)
		a.loading = true
		a.updateLayout()
		return a, tea.Batch(
			a.spinner.Tick,
			a.loadVhosts(),
			a.loadClusterInfo(),
		)

	case profileSavedMsg:
		if msg.err != nil {
			a.profileSel.formErr = fmt.Sprintf("Error saving: %s", msg.err)
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

	case vhostsLoadedMsg:
		if msg.err != nil {
			a.queueList.NewStatusMessage(fmt.Sprintf("Error: %s", msg.err))
			a.loading = false
		} else {
			a.vhosts = msg.vhosts
			initialVhost := a.client.Vhost()
			found := false
			for _, v := range msg.vhosts {
				if v == initialVhost {
					found = true
					break
				}
			}
			if !found && len(msg.vhosts) > 0 {
				initialVhost = msg.vhosts[0]
			}
			a.selectedVhost = initialVhost
			cmds = append(cmds, a.loadQueuesForVhost(initialVhost))
		}

	case clusterInfoMsg:
		if msg.err == nil {
			a.clusterName = msg.clusterName
		}

	case queuesLoadedMsg:
		if msg.vhost != "" && msg.vhost != a.selectedVhost {
			break
		}
		a.loading = false
		if msg.err != nil {
			a.queueList.NewStatusMessage(fmt.Sprintf("Error: %s", msg.err))
		} else {
			a.allQueues = msg.queues
			items := make([]list.Item, len(msg.queues))
			for i, q := range msg.queues {
				items[i] = QueueItem{queue: q}
			}
			cmd := a.queueList.SetItems(items)
			cmds = append(cmds, cmd)

			// Auto-peek first queue
			if len(msg.queues) > 0 {
				a.loading = true
				cmds = append(cmds, a.spinner.Tick, a.loadMessages(a.selectedVhost, msg.queues[0].Name))
			}
		}

	case messagesLoadedMsg:
		a.loading = false
		if msg.err != nil {
			a.queueList.NewStatusMessage(fmt.Sprintf("Error: %s", msg.err))
		} else {
			a.messages.SetMessages(msg.messages, msg.queueName, msg.totalCount)
			a.queueList.NewStatusMessage(fmt.Sprintf("Loaded %d msgs from %s", len(msg.messages), msg.queueName))
		}

	case switchProfileMsg:
		newClient, err := rabbit.NewClient(msg.profile)
		if err != nil {
			a.queueList.NewStatusMessage(fmt.Sprintf("Error: %s", err))
		} else {
			a.client = newClient
			a.initMainScreen(msg.name)
			a.overlay = overlayNone
			a.loading = true
			a.updateLayout()
			cmds = append(cmds, a.spinner.Tick, a.loadVhosts(), a.loadClusterInfo())
		}
	}

	// Forward messages to active panel
	if a.activePanel == panelQueues && a.overlay == overlayNone {
		var cmd tea.Cmd
		a.queueList, cmd = a.queueList.Update(msg)
		if cmd != nil {
			cmds = append(cmds, cmd)
		}
	} else if a.activePanel == panelMessages && a.overlay == overlayNone {
		var cmd tea.Cmd
		vp := a.messages.Viewport()
		*vp, cmd = vp.Update(msg)
		if cmd != nil {
			cmds = append(cmds, cmd)
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

	// Overlays
	if a.overlay != overlayNone {
		return a.handleOverlayKey(key)
	}

	// If the queue list is filtering, let it handle all keys
	if a.queueList.SettingFilter() {
		return nil // keys forwarded via updateMain
	}

	switch key {
	case "q":
		return tea.Quit

	case "?":
		a.helpModel.ShowAll = !a.helpModel.ShowAll
		return nil

	case "tab":
		if a.activePanel == panelQueues {
			a.activePanel = panelMessages
		} else {
			a.activePanel = panelQueues
		}
		return nil

	case "enter":
		if a.activePanel == panelQueues {
			if item, ok := a.queueList.SelectedItem().(QueueItem); ok {
				a.activePanel = panelMessages
				a.loading = true
				return tea.Batch(a.spinner.Tick, a.loadMessages(a.selectedVhost, item.queue.Name))
			}
		}
		return nil

	case "r":
		if sel, ok := a.queueList.SelectedItem().(QueueItem); ok {
			a.loading = true
			return tea.Batch(a.spinner.Tick, a.loadMessages(a.selectedVhost, sel.queue.Name))
		}
		return nil

	case "R":
		a.loading = true
		return tea.Batch(a.spinner.Tick, a.loadQueuesForVhost(a.selectedVhost))

	case "v":
		// Cycle vhost
		if len(a.vhosts) > 1 {
			idx := 0
			for i, v := range a.vhosts {
				if v == a.selectedVhost {
					idx = (i + 1) % len(a.vhosts)
					break
				}
			}
			return a.switchVhost(a.vhosts[idx])
		}
		return nil

	case "p":
		a.overlay = overlayProfile
		a.profileIdx = 0
		return nil

	case "+", "=":
		if a.fetchCount < 500 {
			a.fetchCount += 10
		}
		a.queueList.NewStatusMessage(fmt.Sprintf("Fetch count: %d", a.fetchCount))
		return nil

	case "-":
		if a.fetchCount > 1 {
			a.fetchCount -= 10
			if a.fetchCount < 1 {
				a.fetchCount = 1
			}
		}
		a.queueList.NewStatusMessage(fmt.Sprintf("Fetch count: %d", a.fetchCount))
		return nil
	}

	return nil
}

func (a *App) handleOverlayKey(key string) tea.Cmd {
	if a.overlay == overlayProfile {
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
	}
	return nil
}

func (a *App) switchVhost(newVhost string) tea.Cmd {
	a.selectedVhost = newVhost
	a.allQueues = nil
	a.messages = NewMessagePanel()
	a.messages.SetSize(a.width-a.width*3/10-1, a.height-2)
	a.loading = true
	a.queueList.NewStatusMessage(fmt.Sprintf("Switching to vhost: %s", newVhost))
	return tea.Batch(a.spinner.Tick, a.loadQueuesForVhost(newVhost))
}

// View

func (a App) View() string {
	if a.width == 0 || a.height == 0 {
		return ""
	}
	if !a.ready {
		return fmt.Sprintf("\n  %s Loading rabbitpeek...", a.spinner.View())
	}
	if a.screen == screenProfileSelect {
		return a.profileSel.View()
	}

	// Header
	headerView := a.renderHeader()

	// Content height
	contentH := a.height - 2
	if contentH < 1 {
		contentH = 1
	}

	// Queue list (left)
	queueView := a.queueList.View()

	// Messages (right)
	var msgView string
	if a.loading {
		msgView = lipgloss.NewStyle().
			Width(a.width - a.width*3/10 - 1).
			Height(contentH).
			Render(fmt.Sprintf("\n  %s Loading...", a.spinner.View()))
	} else {
		msgView = a.messages.View()
	}

	// Divider
	divider := lipgloss.NewStyle().
		Foreground(ColorDivider).
		Render(strings.Repeat("│\n", contentH))

	// Join panels
	panels := lipgloss.JoinHorizontal(lipgloss.Top, queueView, divider, msgView)

	// Help footer
	helpView := a.helpModel.View(Keys)

	// Stack
	fullView := lipgloss.JoinVertical(lipgloss.Left, headerView, panels, helpView)

	// Profile overlay
	if a.overlay == overlayProfile {
		fullView = a.renderOverlay(fullView, a.profileView())
	}

	return fullView
}

func (a App) renderHeader() string {
	accentStyle := lipgloss.NewStyle().Foreground(ColorAccent).Bold(true)
	mutedStyle := lipgloss.NewStyle().Foreground(ColorMuted)
	sepStyle := lipgloss.NewStyle().Foreground(ColorDivider)
	sep := sepStyle.Render(" │ ")

	left := accentStyle.Render("  rabbitpeek")

	if a.clusterName != "" {
		left += sep + mutedStyle.Render(a.clusterName)
	}

	left += sep + mutedStyle.Render("vhost:") + " " + accentStyle.Render(a.selectedVhost)
	if len(a.vhosts) > 1 {
		left += " " + mutedStyle.Render(fmt.Sprintf("[v: cycle %d]", len(a.vhosts)))
	}

	left += sep + mutedStyle.Render(fmt.Sprintf("fetch: %d", a.fetchCount))

	return lipgloss.NewStyle().
		Width(a.width).
		Render(left)
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

func (a App) renderOverlay(base, content string) string {
	w := lipgloss.Width(content)
	h := lipgloss.Height(content)
	x := (a.width - w) / 2
	y := (a.height - h) / 2
	if x < 0 {
		x = 0
	}
	if y < 0 {
		y = 0
	}
	return placeOverlay(x, y, content, base)
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

func (a *App) loadVhosts() tea.Cmd {
	client := a.client
	return func() tea.Msg {
		vhosts, err := client.ListVhosts()
		return vhostsLoadedMsg{vhosts: vhosts, err: err}
	}
}

func (a *App) loadClusterInfo() tea.Cmd {
	client := a.client
	return func() tea.Msg {
		overview, err := client.GetOverview()
		if err != nil {
			return clusterInfoMsg{err: err}
		}
		return clusterInfoMsg{clusterName: overview.ClusterName}
	}
}

func (a *App) loadQueuesForVhost(vhost string) tea.Cmd {
	client := a.client
	return func() tea.Msg {
		queues, err := client.ListQueues(vhost)
		return queuesLoadedMsg{queues: queues, vhost: vhost, err: err}
	}
}

func (a *App) loadMessages(vhost, queue string) tea.Cmd {
	client := a.client
	count := a.fetchCount
	return func() tea.Msg {
		messages, err := client.PeekMessages(vhost, queue, count)
		if err != nil {
			return messagesLoadedMsg{err: err, queueName: queue}
		}
		return messagesLoadedMsg{
			messages:   messages,
			queueName:  queue,
			totalCount: len(messages),
		}
	}
}
