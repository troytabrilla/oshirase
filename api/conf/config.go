package conf

import (
	"fmt"
	"log"
	"os"

	"gopkg.in/yaml.v3"
)

type AniListAPIConfig struct {
	URL string `yaml:"url"`
	// TODO Temporary, remove later.
	UserID int `yaml:"user_id"`
}

type Config struct {
	AniListAPI AniListAPIConfig `yaml:"anilist_api"`
}

func LoadConfig() Config {
	config := Config{}

	file, err := os.ReadFile("../config/config.yaml")
	if err != nil {
		fmt.Println("what")
		log.Fatalf("Could not load config file: %v", err)
	}
	err = yaml.Unmarshal(file, &config)
	if err != nil {
		fmt.Println("hello")
		log.Fatalf("Could not parse config file: %v", err)
	}

	return config
}
