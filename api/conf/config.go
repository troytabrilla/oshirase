package conf

import (
	"log"
	"os"

	"gopkg.in/yaml.v3"
)

type APIConfig struct {
	Port int `yaml:"port"`
}

type AniListAPIConfig struct {
	URL string `yaml:"url"`
	// TODO Temporary, remove later.
	UserID int `yaml:"user_id"`
}

type AniListConfig struct {
	API AniListAPIConfig `yaml:"api"`
}

type SubsPleaseRSSConfig struct {
	URL string `yaml:"url"`
}

type SubsPleaseConfig struct {
	RSS SubsPleaseRSSConfig `yaml:"rss"`
}

type MongoDBConfig struct {
	URI      string `yaml:"uri"`
	Database string `yaml:"database"`
}

type DBConfig struct {
	MongoDB MongoDBConfig `yaml:"mongodb"`
}

type SourcesConfig struct {
	AniList    AniListConfig    `yaml:"anilist"`
	SubsPlease SubsPleaseConfig `yaml:"subsplease"`
}

type Config struct {
	API     APIConfig     `yaml:"api"`
	DB      DBConfig      `yaml:"db"`
	Sources SourcesConfig `yaml:"sources"`
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
