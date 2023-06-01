package models

import (
	"testing"

	"github.com/troytabrilla/oshirase/api/api/conf"
	_ "github.com/troytabrilla/oshirase/api/api/test"
)

func TestAnimeListInvalid(test *testing.T) {
	config := conf.LoadConfig()
	anilist := AniList{Config: &config}

	res, err := anilist.FetchList(0, "ANIME", []string{})

	if len(res) != 0 || err == nil {
		test.Fatalf("FetchList should return an empty list and an error for an invalid user: %v, %v.", res, err)
	}
}

func TestAnimeListDefault(test *testing.T) {
	config := conf.LoadConfig()
	anilist := AniList{Config: &config}

	res, err := anilist.FetchList(config.Sources.AniList.API.UserID, "ANIME", []string{})
	if len(res) == 0 || err != nil {
		test.Fatalf("FetchList should return a non-empty anime list for the default user: %v, %v.", res, err)
	}
}

func TestMangaListDefault(test *testing.T) {
	config := conf.LoadConfig()
	anilist := AniList{Config: &config}

	res, err := anilist.FetchList(config.Sources.AniList.API.UserID, "MANGA", []string{})
	if len(res) == 0 || err != nil {
		test.Fatalf("FetchList should return a non-empty manga list for the default user: %v, %v.", res, err)
	}
}
