package config

import (
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/viper"
)

type Profile struct {
	Host     string `mapstructure:"host"`
	Port     int    `mapstructure:"port"`
	Username string `mapstructure:"username"`
	Password string `mapstructure:"password"`
	Vhost    string `mapstructure:"vhost"`
	TLS      bool   `mapstructure:"tls"`
	TLSCert  string `mapstructure:"tls_cert"`
	TLSKey   string `mapstructure:"tls_key"`
	TLSCA    string `mapstructure:"tls_ca"`
}

func (p Profile) BaseURL() string {
	scheme := "http"
	if p.TLS {
		scheme = "https"
	}
	return fmt.Sprintf("%s://%s:%d", scheme, p.Host, p.Port)
}

func (p Profile) TLSConfig() (*tls.Config, error) {
	if !p.TLS {
		return nil, nil
	}
	tlsCfg := &tls.Config{}

	if p.TLSCert != "" && p.TLSKey != "" {
		cert, err := tls.LoadX509KeyPair(p.TLSCert, p.TLSKey)
		if err != nil {
			return nil, fmt.Errorf("loading client cert: %w", err)
		}
		tlsCfg.Certificates = []tls.Certificate{cert}
	}

	if p.TLSCA != "" {
		caCert, err := os.ReadFile(p.TLSCA)
		if err != nil {
			return nil, fmt.Errorf("reading CA cert: %w", err)
		}
		pool := x509.NewCertPool()
		pool.AppendCertsFromPEM(caCert)
		tlsCfg.RootCAs = pool
	}

	return tlsCfg, nil
}

type Config struct {
	Profiles       map[string]Profile `mapstructure:"profiles"`
	DefaultProfile string             `mapstructure:"default_profile"`
}

func (c *Config) GetProfile(name string) (Profile, error) {
	p, ok := c.Profiles[name]
	if !ok {
		return Profile{}, fmt.Errorf("profile %q not found in config", name)
	}
	return p, nil
}

func (c *Config) ProfileNames() []string {
	names := make([]string, 0, len(c.Profiles))
	for k := range c.Profiles {
		names = append(names, k)
	}
	return names
}

// Load reads config from the given path, or the default ~/.config/rabbitpeek/config.toml
func Load(path string) (*Config, error) {
	v := viper.New()
	v.SetConfigType("toml")

	if path != "" {
		v.SetConfigFile(path)
	} else {
		home, err := os.UserHomeDir()
		if err != nil {
			return nil, fmt.Errorf("getting home dir: %w", err)
		}
		configDir := filepath.Join(home, ".config", "rabbitpeek")
		v.SetConfigName("config")
		v.AddConfigPath(configDir)
	}

	cfg := &Config{
		Profiles: make(map[string]Profile),
	}

	if err := v.ReadInConfig(); err != nil {
		if _, ok := err.(viper.ConfigFileNotFoundError); ok {
			// Return default config with a local profile
			cfg.DefaultProfile = "local"
			cfg.Profiles["local"] = Profile{
				Host:     "localhost",
				Port:     15672,
				Username: "guest",
				Password: "guest",
				Vhost:    "/",
			}
			return cfg, nil
		}
		return nil, fmt.Errorf("reading config: %w", err)
	}

	if err := v.Unmarshal(cfg); err != nil {
		return nil, fmt.Errorf("parsing config: %w", err)
	}

	return cfg, nil
}
