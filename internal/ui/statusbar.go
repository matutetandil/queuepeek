package ui

import (
	"fmt"
)

type StatusBar struct {
	width      int
	message    string
	isError    bool
	profile    string
	fetchCount int
}

func NewStatusBar(profile string) StatusBar {
	return StatusBar{
		profile:    profile,
		fetchCount: 50,
	}
}

func (s *StatusBar) SetWidth(width int) {
	s.width = width
}

func (s *StatusBar) SetMessage(msg string, isError bool) {
	s.message = msg
	s.isError = isError
}

func (s *StatusBar) SetProfile(profile string) {
	s.profile = profile
}

func (s *StatusBar) FetchCount() int {
	return s.fetchCount
}

func (s *StatusBar) IncreaseFetchCount() {
	if s.fetchCount < 500 {
		s.fetchCount += 10
	}
}

func (s *StatusBar) DecreaseFetchCount() {
	if s.fetchCount > 1 {
		s.fetchCount -= 10
		if s.fetchCount < 1 {
			s.fetchCount = 1
		}
	}
}

func (s StatusBar) View() string {
	if s.width == 0 {
		return ""
	}

	left := fmt.Sprintf(" Profile: %s | Fetch: %d | +/- fetch count | ? help",
		s.profile, s.fetchCount)

	if s.message != "" {
		line := fmt.Sprintf("%s | %s", left, s.message)
		if s.isError {
			return StyleStatusBarError.Width(s.width).Render(line)
		}
		return StyleStatusBar.Width(s.width).Render(line)
	}

	return StyleStatusBar.Width(s.width).Render(left)
}
