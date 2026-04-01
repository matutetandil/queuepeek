package ui

import "github.com/charmbracelet/lipgloss"

// Theme holds all colors for the UI.
type Theme struct {
	Name       string
	Bg         lipgloss.Color
	SidebarBg  lipgloss.Color
	Selected   lipgloss.Color
	Primary    lipgloss.Color
	Muted      lipgloss.Color
	Accent     lipgloss.Color
	Error      lipgloss.Color
	Success    lipgloss.Color
	White      lipgloss.Color
	Divider    lipgloss.Color
	StatusBg   lipgloss.Color
	SearchBg   lipgloss.Color
	HighlightBg lipgloss.Color
}

var Themes = map[string]Theme{
	"slack": {
		Name:        "slack",
		Bg:          lipgloss.Color("#1A1D21"),
		SidebarBg:   lipgloss.Color("#19171D"),
		Selected:    lipgloss.Color("#1164A3"),
		Primary:     lipgloss.Color("#D1D2D3"),
		Muted:       lipgloss.Color("#696B72"),
		Accent:      lipgloss.Color("#ECB22E"),
		Error:       lipgloss.Color("#E01E5A"),
		Success:     lipgloss.Color("#2BAC76"),
		White:       lipgloss.Color("#FFFFFF"),
		Divider:     lipgloss.Color("#3D3F45"),
		StatusBg:    lipgloss.Color("#1E2126"),
		SearchBg:    lipgloss.Color("#2C2D31"),
		HighlightBg: lipgloss.Color("#252729"),
	},
	"dracula": {
		Name:        "dracula",
		Bg:          lipgloss.Color("#282A36"),
		SidebarBg:   lipgloss.Color("#21222C"),
		Selected:    lipgloss.Color("#6272A4"),
		Primary:     lipgloss.Color("#F8F8F2"),
		Muted:       lipgloss.Color("#6272A4"),
		Accent:      lipgloss.Color("#BD93F9"),
		Error:       lipgloss.Color("#FF5555"),
		Success:     lipgloss.Color("#50FA7B"),
		White:       lipgloss.Color("#F8F8F2"),
		Divider:     lipgloss.Color("#44475A"),
		StatusBg:    lipgloss.Color("#21222C"),
		SearchBg:    lipgloss.Color("#44475A"),
		HighlightBg: lipgloss.Color("#44475A"),
	},
	"gruvbox": {
		Name:        "gruvbox",
		Bg:          lipgloss.Color("#282828"),
		SidebarBg:   lipgloss.Color("#1D2021"),
		Selected:    lipgloss.Color("#458588"),
		Primary:     lipgloss.Color("#EBDBB2"),
		Muted:       lipgloss.Color("#928374"),
		Accent:      lipgloss.Color("#FABD2F"),
		Error:       lipgloss.Color("#FB4934"),
		Success:     lipgloss.Color("#B8BB26"),
		White:       lipgloss.Color("#FBF1C7"),
		Divider:     lipgloss.Color("#504945"),
		StatusBg:    lipgloss.Color("#1D2021"),
		SearchBg:    lipgloss.Color("#3C3836"),
		HighlightBg: lipgloss.Color("#3C3836"),
	},
	"catppuccin": {
		Name:        "catppuccin",
		Bg:          lipgloss.Color("#1E1E2E"),
		SidebarBg:   lipgloss.Color("#181825"),
		Selected:    lipgloss.Color("#89B4FA"),
		Primary:     lipgloss.Color("#CDD6F4"),
		Muted:       lipgloss.Color("#6C7086"),
		Accent:      lipgloss.Color("#F9E2AF"),
		Error:       lipgloss.Color("#F38BA8"),
		Success:     lipgloss.Color("#A6E3A1"),
		White:       lipgloss.Color("#CDD6F4"),
		Divider:     lipgloss.Color("#45475A"),
		StatusBg:    lipgloss.Color("#181825"),
		SearchBg:    lipgloss.Color("#313244"),
		HighlightBg: lipgloss.Color("#313244"),
	},
	"tokyo-night": {
		Name:        "tokyo-night",
		Bg:          lipgloss.Color("#1A1B26"),
		SidebarBg:   lipgloss.Color("#16161E"),
		Selected:    lipgloss.Color("#7AA2F7"),
		Primary:     lipgloss.Color("#A9B1D6"),
		Muted:       lipgloss.Color("#565F89"),
		Accent:      lipgloss.Color("#E0AF68"),
		Error:       lipgloss.Color("#F7768E"),
		Success:     lipgloss.Color("#9ECE6A"),
		White:       lipgloss.Color("#C0CAF5"),
		Divider:     lipgloss.Color("#3B4261"),
		StatusBg:    lipgloss.Color("#16161E"),
		SearchBg:    lipgloss.Color("#24283B"),
		HighlightBg: lipgloss.Color("#24283B"),
	},
}

// ThemeNames returns theme names in a stable display order.
func ThemeNames() []string {
	return []string{"slack", "dracula", "gruvbox", "catppuccin", "tokyo-night"}
}

// Active theme colors — set via ApplyTheme.
var (
	ColorBg         lipgloss.Color
	ColorSidebarBg  lipgloss.Color
	ColorSelected   lipgloss.Color
	ColorPrimary    lipgloss.Color
	ColorMuted      lipgloss.Color
	ColorAccent     lipgloss.Color
	ColorError      lipgloss.Color
	ColorSuccess    lipgloss.Color
	ColorWhite      lipgloss.Color
	ColorDivider    lipgloss.Color
	ColorStatusBg   lipgloss.Color
	ColorSearchBg   lipgloss.Color
	ColorHighlightBg lipgloss.Color
)

// Styles — rebuilt after every theme change.
var (
	StyleSidebarHeader   lipgloss.Style
	StyleQueueItem       lipgloss.Style
	StyleQueueItemSelected lipgloss.Style
	StyleQueueCount      lipgloss.Style
	StyleQueueCountHigh  lipgloss.Style
	StyleQueueCountZero  lipgloss.Style
	StyleMessageHeader   lipgloss.Style
	StyleMessageMeta     lipgloss.Style
	StyleMessageBody     lipgloss.Style
	StyleMessageIndex    lipgloss.Style
	StyleStatusBar       lipgloss.Style
	StyleStatusBarError  lipgloss.Style
	StyleSearchBar       lipgloss.Style
	StyleSearchLabel     lipgloss.Style
	StyleSearchError     lipgloss.Style
	StyleHelpOverlay     lipgloss.Style
	StyleHelpKey         lipgloss.Style
	StyleHelpDesc        lipgloss.Style
	StyleProfileOverlay  lipgloss.Style
	StyleProfileSelected lipgloss.Style
	StyleProfileItem     lipgloss.Style
	StyleSpinner         lipgloss.Style
)

func init() {
	ApplyTheme("slack")
}

// ApplyTheme sets all color variables and rebuilds all styles.
func ApplyTheme(name string) {
	t, ok := Themes[name]
	if !ok {
		t = Themes["slack"]
	}

	ColorBg = t.Bg
	ColorSidebarBg = t.SidebarBg
	ColorSelected = t.Selected
	ColorPrimary = t.Primary
	ColorMuted = t.Muted
	ColorAccent = t.Accent
	ColorError = t.Error
	ColorSuccess = t.Success
	ColorWhite = t.White
	ColorDivider = t.Divider
	ColorStatusBg = t.StatusBg
	ColorSearchBg = t.SearchBg
	ColorHighlightBg = t.HighlightBg

	rebuildStyles()
}

func rebuildStyles() {
	StyleSidebarHeader = lipgloss.NewStyle().
		Foreground(ColorWhite).
		Bold(true)

	StyleQueueItem = lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Padding(0, 1)

	StyleQueueItemSelected = lipgloss.NewStyle().
		Foreground(ColorWhite).
		Background(ColorSelected).
		Bold(true).
		Padding(0, 1)

	StyleQueueCount = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true)

	StyleQueueCountHigh = lipgloss.NewStyle().
		Foreground(ColorError).
		Bold(true)

	StyleQueueCountZero = lipgloss.NewStyle().
		Foreground(ColorSuccess)

	StyleMessageHeader = lipgloss.NewStyle().
		Foreground(ColorWhite).
		Bold(true)

	StyleMessageMeta = lipgloss.NewStyle().
		Foreground(ColorMuted)

	StyleMessageBody = lipgloss.NewStyle().
		Foreground(ColorPrimary)

	StyleMessageIndex = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true)

	StyleStatusBar = lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Padding(0, 1)

	StyleStatusBarError = lipgloss.NewStyle().
		Foreground(ColorError).
		Bold(true).
		Padding(0, 1)

	StyleSearchBar = lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Padding(0, 1)

	StyleSearchLabel = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true)

	StyleSearchError = lipgloss.NewStyle().
		Foreground(ColorError).
		Italic(true)

	StyleHelpOverlay = lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(ColorSelected).
		Foreground(ColorPrimary).
		Padding(1, 2).
		Align(lipgloss.Center)

	StyleHelpKey = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Bold(true)

	StyleHelpDesc = lipgloss.NewStyle().
		Foreground(ColorPrimary)

	StyleProfileOverlay = lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(ColorAccent).
		Foreground(ColorPrimary).
		Padding(1, 2)

	StyleProfileSelected = lipgloss.NewStyle().
		Foreground(ColorWhite).
		Background(ColorSelected).
		Bold(true).
		Padding(0, 1)

	StyleProfileItem = lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Padding(0, 1)

	StyleSpinner = lipgloss.NewStyle().
		Foreground(ColorAccent)
}

const (
	MinTermWidth  = 80
	MinTermHeight = 24
)
