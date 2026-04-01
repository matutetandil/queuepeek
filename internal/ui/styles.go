package ui

import "github.com/charmbracelet/lipgloss"

// Color palette — Slack dark theme inspired
var (
	ColorBg        = lipgloss.Color("#1A1D21")
	ColorSidebarBg = lipgloss.Color("#19171D")
	ColorSelected  = lipgloss.Color("#1164A3")
	ColorPrimary   = lipgloss.Color("#D1D2D3")
	ColorMuted     = lipgloss.Color("#696B72")
	ColorAccent    = lipgloss.Color("#ECB22E")
	ColorError     = lipgloss.Color("#E01E5A")
	ColorSuccess   = lipgloss.Color("#2BAC76")
	ColorWhite     = lipgloss.Color("#FFFFFF")
	ColorDivider   = lipgloss.Color("#3D3F45")
)

// Base styles — no hardcoded Width/Height; dimensions applied at render time.
var (
	StyleSidebarHeader = lipgloss.NewStyle().
				Foreground(ColorWhite).
				Bold(true).
				Background(ColorSidebarBg)

	StyleQueueItem = lipgloss.NewStyle().
			Foreground(ColorPrimary).
			Background(ColorSidebarBg).
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
			Background(lipgloss.Color("#1E2126")).
			Padding(0, 1)

	StyleStatusBarError = lipgloss.NewStyle().
				Foreground(ColorError).
				Background(lipgloss.Color("#1E2126")).
				Bold(true).
				Padding(0, 1)

	StyleSearchBar = lipgloss.NewStyle().
			Foreground(ColorPrimary).
			Background(lipgloss.Color("#2C2D31")).
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
			Background(ColorSidebarBg).
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
				Background(ColorSidebarBg).
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
)

const (
	MinTermWidth  = 80
	MinTermHeight = 24
)
