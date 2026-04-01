package ui

import "fmt"

func formatCount(count int) string {
	s := fmt.Sprintf("(%d)", count)
	switch {
	case count == 0:
		return StyleQueueCountZero.Render(s)
	case count > 1000:
		return StyleQueueCountHigh.Render(s)
	default:
		return StyleQueueCount.Render(s)
	}
}

func truncateStr(s string, maxLen int) string {
	if maxLen <= 0 {
		return ""
	}
	if len(s) <= maxLen {
		return s
	}
	if maxLen <= 3 {
		return s[:maxLen]
	}
	return s[:maxLen-3] + "..."
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
