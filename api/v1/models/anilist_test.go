package models

import (
	"testing"

	"github.com/troytabrilla/oshirase/api/api/conf"
	_ "github.com/troytabrilla/oshirase/api/api/test"
)

func TestAnimeListInvalid(test *testing.T) {
	config := conf.LoadConfig()
	api := AniListAPI{Config: &config}

	res, err := api.Fetch(0, "ANIME", []string{})

	if len(res) != 0 || err == nil {
		test.Fatalf(`Fetch should return an empty list and an error for an invalid user.`)
	}
}

func TestAnimeListDefault(test *testing.T) {
	config := conf.LoadConfig()
	api := AniListAPI{Config: &config}

	res, err := api.Fetch(config.AniListAPI.UserID, "ANIME", []string{})
	if len(res) == 0 || err != nil {
		test.Fatalf(`Fetch should return a non-empty anime list for the default user.`)
	}
}

func TestMangaListDefault(test *testing.T) {
	config := conf.LoadConfig()
	api := AniListAPI{Config: &config}

	res, err := api.Fetch(config.AniListAPI.UserID, "MANGA", []string{})
	if len(res) == 0 || err != nil {
		test.Fatalf(`Fetch should return a non-empty manga list for the default user.`)
	}
}
