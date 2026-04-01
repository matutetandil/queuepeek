package ui

import (
	"github.com/charmbracelet/bubbles/help"
	"github.com/charmbracelet/bubbles/key"
)

// KeyMap implements help.KeyMap for the main app.
type KeyMap struct {
	Quit       key.Binding
	Help       key.Binding
	Tab        key.Binding
	Search     key.Binding
	Reload     key.Binding
	ReloadAll  key.Binding
	Profile    key.Binding
	Stats      key.Binding
	FetchUp    key.Binding
	FetchDown  key.Binding
}

var Keys = KeyMap{
	Quit:      key.NewBinding(key.WithKeys("q", "ctrl+c"), key.WithHelp("q", "quit")),
	Help:      key.NewBinding(key.WithKeys("?"), key.WithHelp("?", "help")),
	Tab:       key.NewBinding(key.WithKeys("tab"), key.WithHelp("tab", "switch panel")),
	Search:    key.NewBinding(key.WithKeys("/"), key.WithHelp("/", "filter queues")),
	Reload:    key.NewBinding(key.WithKeys("r"), key.WithHelp("r", "reload msgs")),
	ReloadAll: key.NewBinding(key.WithKeys("R"), key.WithHelp("R", "reload queues")),
	Profile:   key.NewBinding(key.WithKeys("p"), key.WithHelp("p", "profile")),
	Stats:     key.NewBinding(key.WithKeys("c"), key.WithHelp("c", "stats")),
	FetchUp:   key.NewBinding(key.WithKeys("+"), key.WithHelp("+/-", "fetch count")),
	FetchDown: key.NewBinding(key.WithKeys("-")),
}

func (k KeyMap) ShortHelp() []key.Binding {
	return []key.Binding{k.Tab, k.Search, k.Reload, k.Profile, k.Help, k.Quit}
}

func (k KeyMap) FullHelp() [][]key.Binding {
	return [][]key.Binding{
		{k.Tab, k.Search, k.Reload, k.ReloadAll},
		{k.Profile, k.Stats, k.FetchUp, k.Help, k.Quit},
	}
}

// Ensure it implements help.KeyMap
var _ help.KeyMap = KeyMap{}
