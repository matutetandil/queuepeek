package cmd

import (
	"fmt"
	"os"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/spf13/cobra"

	"github.com/matutedenda/rabbitpeek/internal/config"
	"github.com/matutedenda/rabbitpeek/internal/rabbit"
	"github.com/matutedenda/rabbitpeek/internal/ui"
)

var (
	profileFlag string
	configPath  string
)

var rootCmd = &cobra.Command{
	Use:   "rabbitpeek",
	Short: "A TUI tool for peeking into RabbitMQ queues",
	Run: func(cmd *cobra.Command, args []string) {
		cfg, err := config.Load(configPath)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error loading config: %v\n", err)
			os.Exit(1)
		}

		profileName := profileFlag
		if profileName == "" {
			profileName = cfg.DefaultProfile
		}
		if profileName == "" {
			profileName = "local"
		}

		profile, err := cfg.GetProfile(profileName)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error: %v\n", err)
			os.Exit(1)
		}

		client, err := rabbit.NewClient(profile)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error creating client: %v\n", err)
			os.Exit(1)
		}
		model := ui.NewApp(client, profileName, cfg)

		p := tea.NewProgram(model, tea.WithAltScreen(), tea.WithMouseCellMotion())
		if _, err := p.Run(); err != nil {
			fmt.Fprintf(os.Stderr, "Error running program: %v\n", err)
			os.Exit(1)
		}
	},
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func init() {
	rootCmd.Flags().StringVarP(&profileFlag, "profile", "p", "", "Connection profile to use")
	rootCmd.Flags().StringVarP(&configPath, "config", "c", "", "Path to config file")
}
