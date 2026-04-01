package ui

import (
	"fmt"
	"strconv"
	"strings"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/matutedenda/rabbitpeek/internal/config"
)

type profileSelectMode int

const (
	modeSelectProfile profileSelectMode = iota
	modeAddProfile
	modeEditProfile
	modeConfirmDelete
)

type profileChosenMsg struct {
	name    string
	profile config.Profile
}

type profileSavedMsg struct {
	name string
	err  error
}

type profileDeletedMsg struct {
	err error
}

type ProfileSelect struct {
	config     *config.Config
	configPath string
	mode       profileSelectMode
	cursor     int
	width      int
	height     int

	// Form (shared between add and edit)
	formInputs  []textinput.Model
	formFocus   int
	formLabels  []string
	formHints   []string
	formErr     string
	tlsEnabled  bool
	editingName string // original name when editing
}

const (
	fieldName    = 0
	fieldHost    = 1
	fieldPort    = 2
	fieldUsername = 3
	fieldPassword = 4
	fieldVhost   = 5
)

var cloudHostPatterns = []string{
	"cloudamqp.com",
	"amazonaws.com",
	"azure.com",
	"rabbitmq.cloud",
}

func isCloudHost(host string) bool {
	h := strings.ToLower(host)
	for _, pattern := range cloudHostPatterns {
		if strings.Contains(h, pattern) {
			return true
		}
	}
	return false
}

func newFormInputs() []textinput.Model {
	inputs := make([]textinput.Model, 6)
	for i := range inputs {
		inputs[i] = textinput.New()
		inputs[i].CharLimit = 256
		inputs[i].PromptStyle = StyleSearchLabel
		inputs[i].TextStyle = lipgloss.NewStyle().Foreground(ColorPrimary)
		inputs[i].PlaceholderStyle = lipgloss.NewStyle().Foreground(ColorMuted)
		inputs[i].Prompt = "> "
	}
	inputs[fieldName].Placeholder = "my-rabbit"
	inputs[fieldHost].Placeholder = "localhost"
	inputs[fieldPort].Placeholder = "15672"
	inputs[fieldUsername].Placeholder = "guest"
	inputs[fieldPassword].Placeholder = "guest"
	inputs[fieldPassword].EchoMode = textinput.EchoPassword
	inputs[fieldVhost].Placeholder = "/"
	return inputs
}

func NewProfileSelect(cfg *config.Config, configPath string) ProfileSelect {
	return ProfileSelect{
		config:     cfg,
		configPath: configPath,
		formInputs: newFormInputs(),
		formLabels: []string{"Profile Name", "Host", "Port", "Username", "Password", "Vhost"},
		formHints: []string{
			"A friendly name for this connection",
			"Hostname or IP of the RabbitMQ management API",
			"15672 = local default, 443 = cloud/TLS (e.g. CloudAMQP)",
			"RabbitMQ management user",
			"RabbitMQ management password",
			"Virtual host (default: /)",
		},
	}
}

func (p *ProfileSelect) SetSize(width, height int) {
	p.width = width
	p.height = height
	for i := range p.formInputs {
		p.formInputs[i].Width = 40
	}
}

func (p ProfileSelect) Init() tea.Cmd {
	return nil
}

func (p ProfileSelect) Update(msg tea.Msg) (ProfileSelect, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		return p.handleKey(msg)
	case profileDeletedMsg:
		if msg.err != nil {
			p.formErr = fmt.Sprintf("Error deleting: %s", msg.err)
		}
		p.mode = modeSelectProfile
		names := p.config.ProfileNames()
		if p.cursor >= len(names)+1 {
			p.cursor = len(names)
		}
		return p, nil
	}

	if p.mode == modeAddProfile || p.mode == modeEditProfile {
		var cmd tea.Cmd
		p.formInputs[p.formFocus], cmd = p.formInputs[p.formFocus].Update(msg)
		return p, cmd
	}

	return p, nil
}

func (p ProfileSelect) handleKey(msg tea.KeyMsg) (ProfileSelect, tea.Cmd) {
	key := msg.String()

	if key == "ctrl+c" {
		return p, tea.Quit
	}

	switch p.mode {
	case modeAddProfile, modeEditProfile:
		return p.handleFormKey(msg)
	case modeConfirmDelete:
		return p.handleDeleteConfirm(key)
	default:
		return p.handleSelectKey(key)
	}
}

func (p ProfileSelect) handleSelectKey(key string) (ProfileSelect, tea.Cmd) {
	names := p.config.ProfileNames()
	totalItems := len(names) + 1 // profiles + "Add new"

	switch key {
	case "q":
		return p, tea.Quit
	case "j", "down":
		if p.cursor < totalItems-1 {
			p.cursor++
		}
	case "k", "up":
		if p.cursor > 0 {
			p.cursor--
		}
	case "enter":
		if p.cursor < len(names) {
			name := names[p.cursor]
			profile, err := p.config.GetProfile(name)
			if err == nil {
				return p, func() tea.Msg {
					return profileChosenMsg{name: name, profile: profile}
				}
			}
		} else {
			p.enterAddMode()
			return p, textinput.Blink
		}
	case "e":
		if p.cursor < len(names) {
			p.enterEditMode(names[p.cursor])
			return p, textinput.Blink
		}
	case "d", "x":
		if p.cursor < len(names) {
			p.mode = modeConfirmDelete
			p.formErr = ""
		}
	}
	return p, nil
}

func (p *ProfileSelect) enterAddMode() {
	p.mode = modeAddProfile
	p.formFocus = 0
	p.formErr = ""
	p.tlsEnabled = false
	p.editingName = ""
	p.formInputs = newFormInputs()
	for i := range p.formInputs {
		p.formInputs[i].Width = 40
	}
	p.formInputs[p.formFocus].Focus()
}

func (p *ProfileSelect) enterEditMode(name string) {
	profile, _ := p.config.GetProfile(name)
	p.mode = modeEditProfile
	p.formFocus = 0
	p.formErr = ""
	p.editingName = name
	p.tlsEnabled = profile.TLS

	p.formInputs = newFormInputs()
	for i := range p.formInputs {
		p.formInputs[i].Width = 40
	}
	p.formInputs[fieldName].SetValue(name)
	p.formInputs[fieldHost].SetValue(profile.Host)
	p.formInputs[fieldPort].SetValue(strconv.Itoa(profile.Port))
	p.formInputs[fieldUsername].SetValue(profile.Username)
	p.formInputs[fieldPassword].SetValue(profile.Password)
	p.formInputs[fieldVhost].SetValue(profile.Vhost)
	p.formInputs[p.formFocus].Focus()
}

func (p ProfileSelect) handleFormKey(msg tea.KeyMsg) (ProfileSelect, tea.Cmd) {
	key := msg.String()
	switch key {
	case "esc":
		p.mode = modeSelectProfile
		p.formErr = ""
		return p, nil

	case "tab", "down":
		p.formInputs[p.formFocus].Blur()
		p.formFocus++
		if p.formFocus >= len(p.formInputs) {
			p.formFocus = 0
		}
		p.formInputs[p.formFocus].Focus()
		return p, textinput.Blink

	case "shift+tab", "up":
		p.formInputs[p.formFocus].Blur()
		p.formFocus--
		if p.formFocus < 0 {
			p.formFocus = len(p.formInputs) - 1
		}
		p.formInputs[p.formFocus].Focus()
		return p, textinput.Blink

	case "enter":
		return p.submitForm()
	}

	var cmd tea.Cmd
	p.formInputs[p.formFocus], cmd = p.formInputs[p.formFocus].Update(msg)

	if p.formFocus == fieldHost {
		p.detectCloudHost()
	}

	return p, cmd
}

func (p ProfileSelect) handleDeleteConfirm(key string) (ProfileSelect, tea.Cmd) {
	names := p.config.ProfileNames()
	switch key {
	case "y", "Y":
		if p.cursor < len(names) {
			name := names[p.cursor]
			cfg := p.config
			configPath := p.configPath
			return p, func() tea.Msg {
				err := cfg.DeleteProfile(name, configPath)
				return profileDeletedMsg{err: err}
			}
		}
		p.mode = modeSelectProfile
	default:
		p.mode = modeSelectProfile
	}
	return p, nil
}

func (p *ProfileSelect) detectCloudHost() {
	host := p.formInputs[fieldHost].Value()
	if isCloudHost(host) {
		currentPort := p.formInputs[fieldPort].Value()
		if currentPort == "" || currentPort == "15672" {
			p.formInputs[fieldPort].SetValue("443")
		}
		p.tlsEnabled = true
	} else if p.tlsEnabled {
		currentPort := p.formInputs[fieldPort].Value()
		if currentPort == "443" {
			p.formInputs[fieldPort].SetValue("")
		}
		p.tlsEnabled = false
	}
}

func (p ProfileSelect) submitForm() (ProfileSelect, tea.Cmd) {
	name := strings.TrimSpace(p.formInputs[fieldName].Value())
	if name == "" {
		p.formErr = "Profile name is required"
		return p, nil
	}

	host := p.formInputs[fieldHost].Value()
	if host == "" {
		host = "localhost"
	}

	portStr := p.formInputs[fieldPort].Value()
	if portStr == "" {
		if isCloudHost(host) {
			portStr = "443"
		} else {
			portStr = "15672"
		}
	}
	port, err := strconv.Atoi(portStr)
	if err != nil {
		p.formErr = "Port must be a number"
		return p, nil
	}

	username := p.formInputs[fieldUsername].Value()
	if username == "" {
		username = "guest"
	}

	password := p.formInputs[fieldPassword].Value()
	if password == "" {
		password = "guest"
	}

	vhost := p.formInputs[fieldVhost].Value()
	if vhost == "" {
		vhost = "/"
	}

	useTLS := p.tlsEnabled || isCloudHost(host) || port == 443

	profile := config.Profile{
		Host:     host,
		Port:     port,
		Username: username,
		Password: password,
		Vhost:    vhost,
		TLS:      useTLS,
	}

	cfg := p.config
	configPath := p.configPath
	editingName := p.editingName

	return p, func() tea.Msg {
		// If editing and name changed, delete old profile
		if editingName != "" && editingName != name {
			delete(cfg.Profiles, editingName)
		}
		err := cfg.AddProfile(name, profile, configPath)
		if err != nil {
			return profileSavedMsg{name: name, err: err}
		}
		return profileChosenMsg{name: name, profile: profile}
	}
}

// Views

func (p ProfileSelect) View() string {
	switch p.mode {
	case modeAddProfile:
		return p.viewForm("New Connection")
	case modeEditProfile:
		return p.viewForm("Edit Connection")
	default:
		return p.viewSelect()
	}
}

func (p ProfileSelect) viewSelect() string {
	names := p.config.ProfileNames()

	titleStyle := lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true).
		MarginBottom(1)

	subtitleStyle := lipgloss.NewStyle().
		Foreground(ColorMuted).
		MarginBottom(2)

	logo := lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true).
		Render("rabbitpeek")

	var b strings.Builder
	b.WriteString(titleStyle.Render("  " + logo))
	b.WriteString("\n")
	b.WriteString(subtitleStyle.Render("  RabbitMQ Queue Inspector"))
	b.WriteString("\n\n")

	if len(names) > 0 {
		sectionStyle := lipgloss.NewStyle().
			Foreground(ColorPrimary).
			Bold(true)
		b.WriteString(sectionStyle.Render("  Saved Profiles"))
		b.WriteString("\n\n")

		for i, name := range names {
			profile, _ := p.config.GetProfile(name)
			scheme := "http"
			if profile.TLS {
				scheme = "https"
			}
			detail := fmt.Sprintf("%s://%s:%d", scheme, profile.Host, profile.Port)

			if i == p.cursor {
				b.WriteString(StyleProfileSelected.Width(60).Render(
					fmt.Sprintf("  > %s  %s", name, lipgloss.NewStyle().Foreground(ColorMuted).Render(detail))))
			} else {
				b.WriteString(StyleProfileItem.Render(
					fmt.Sprintf("    %s  %s", name, lipgloss.NewStyle().Foreground(ColorMuted).Render(detail))))
			}

			// Delete confirmation inline
			if i == p.cursor && p.mode == modeConfirmDelete {
				b.WriteString("\n")
				b.WriteString(StyleSearchError.Render("    Delete this profile? (y/n)"))
			}

			b.WriteString("\n")
		}
		b.WriteString("\n")
	} else {
		noProfilesStyle := lipgloss.NewStyle().
			Foreground(ColorMuted).
			Italic(true)
		b.WriteString(noProfilesStyle.Render("  No profiles configured yet."))
		b.WriteString("\n\n")
	}

	addIdx := len(names)
	if p.cursor == addIdx {
		b.WriteString(StyleProfileSelected.Width(60).Render("  > + Add new connection"))
	} else {
		addStyle := lipgloss.NewStyle().Foreground(ColorSuccess)
		b.WriteString(StyleProfileItem.Render("    " + addStyle.Render("+ Add new connection")))
	}
	b.WriteString("\n\n")

	hintStyle := lipgloss.NewStyle().Foreground(ColorMuted)
	if p.mode == modeConfirmDelete {
		b.WriteString(hintStyle.Render("  y confirm delete  any other key cancel"))
	} else {
		b.WriteString(hintStyle.Render("  j/k navigate  enter connect  e edit  d delete  q quit"))
	}

	if p.formErr != "" {
		b.WriteString("\n")
		b.WriteString(StyleSearchError.Render("  " + p.formErr))
	}

	content := b.String()

	boxStyle := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("#2C2D31")).
		Padding(1, 2).
		Background(ColorSidebarBg)

	box := boxStyle.Render(content)

	return lipgloss.Place(p.width, p.height, lipgloss.Center, lipgloss.Center, box,
		lipgloss.WithWhitespaceBackground(ColorBg))
}

func (p ProfileSelect) viewForm(title string) string {
	titleStyle := lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true).
		MarginBottom(1)

	hintStyle := lipgloss.NewStyle().
		Foreground(ColorMuted).
		Italic(true)

	labelStyle := lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Bold(true).
		Width(16)

	var b strings.Builder
	b.WriteString(titleStyle.Render("  " + title))
	b.WriteString("\n\n")

	for i, label := range p.formLabels {
		b.WriteString("  ")
		b.WriteString(labelStyle.Render(label))
		b.WriteString(p.formInputs[i].View())
		b.WriteString("\n")

		if i == p.formFocus && i < len(p.formHints) {
			b.WriteString("  ")
			b.WriteString(strings.Repeat(" ", 16))
			b.WriteString(hintStyle.Render(p.formHints[i]))
			b.WriteString("\n")
		}
	}

	// TLS indicator
	b.WriteString("\n  ")
	b.WriteString(labelStyle.Render("TLS"))
	if p.tlsEnabled {
		b.WriteString(lipgloss.NewStyle().Foreground(ColorSuccess).Bold(true).Render("enabled"))
	} else {
		b.WriteString(lipgloss.NewStyle().Foreground(ColorMuted).Render("disabled"))
	}
	b.WriteString("\n")
	b.WriteString("  ")
	b.WriteString(strings.Repeat(" ", 16))
	b.WriteString(hintStyle.Render("Auto-enabled for cloud hosts or port 443"))
	b.WriteString("\n")

	host := p.formInputs[fieldHost].Value()
	if isCloudHost(host) {
		b.WriteString("\n")
		cloudNotice := lipgloss.NewStyle().Foreground(ColorSuccess).Bold(true)
		b.WriteString("  ")
		b.WriteString(cloudNotice.Render("Cloud host detected — port and TLS auto-configured"))
		b.WriteString("\n")
	}

	if p.formErr != "" {
		b.WriteString("\n")
		b.WriteString(StyleSearchError.Render("  " + p.formErr))
		b.WriteString("\n")
	}

	b.WriteString("\n")
	b.WriteString(hintStyle.Render("  tab next field  enter save  esc cancel"))

	content := b.String()

	boxStyle := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(ColorAccent).
		Padding(1, 2).
		Background(ColorSidebarBg)

	box := boxStyle.Render(content)

	return lipgloss.Place(p.width, p.height, lipgloss.Center, lipgloss.Center, box,
		lipgloss.WithWhitespaceBackground(ColorBg))
}
