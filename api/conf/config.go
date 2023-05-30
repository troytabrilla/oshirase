package conf

import (
	"log"
	"os"

	"gopkg.in/yaml.v3"
)

type AniListAPIConfig struct {
	URL string `yaml:"url"`
	// TODO Temporary, remove later.
	UserID int `yaml:"user_id"`
}

type AniListConfig struct {
	API AniListAPIConfig `yaml:"api"`
}

type Config struct {
	AniList AniListConfig `yaml:"anilist"`
}

func LoadConfig() Config {
	config := Config{}

	file, err := os.ReadFile("../config/config.yaml")
	if err != nil {
		log.Fatalf("Could not load config file: %v", err)
		os.Exit(1)
	}
	err = yaml.Unmarshal(file, &config)
	if err != nil {
		log.Fatalf("Could not parse config file: %v", err)
		os.Exit(1)
	}

	return config
}
