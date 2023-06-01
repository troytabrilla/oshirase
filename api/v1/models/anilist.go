package models

import (
	"encoding/json"

	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/sources"
)

type Title struct {
	Romaji  string `json:"romaji"`
	English string `json:"english"`
}

type CoverImage struct {
	Large string `json:"large"`
}

type Media struct {
	ID         int        `json:"id"`
	Type       string     `json:"type"`
	Format     string     `json:"format"`
	Season     string     `json:"season"`
	SeasonYear int        `json:"seasonYear"`
	Title      Title      `json:"title"`
	CoverImage CoverImage `json:"coverImage"`
	Episodes   int        `json:"episodes"`
}

type Entry struct {
	Media    Media  `json:"media"`
	Status   string `json:"status"`
	Score    int    `json:"score"`
	Progress int    `json:"progress"`
}

type MediaList struct {
	Name    string  `json:"name"`
	Status  string  `json:"status"`
	Entries []Entry `json:"entries"`
}

type MediaListCollection struct {
	Lists []MediaList `json:"lists"`
}

type AniListAPIData struct {
	MediaListCollection MediaListCollection
}

type AniListAPIResult struct {
	Data AniListAPIData `json:"data"`
}

type Schedule struct {
	Title string
	Day   string
	Time  string
}

type Latest struct {
	Title  string
	Latest int
	URL    string
}

type FlatMedia struct {
	MediaID    int
	MediaType  string
	Status     string
	Format     string
	Season     string
	SeasonYear int
	Title      string
	Image      string
	Episodes   int
	Score      int
	Progress   int
	Schedule   Schedule
	Latest     Latest
	AltTitles  []string
}

type AniList struct {
	Config *conf.Config
}

func (anilist *AniList) FetchList(userId int, mediaType string, status []string) ([]FlatMedia, error) {
	api := sources.AniListAPI{Config: anilist.Config}

	body, err := api.Fetch(userId, mediaType, status)
	if err != nil {
		return []FlatMedia{}, err
	}

	var result AniListAPIResult
	err = json.Unmarshal(body, &result)
	if err != nil {
		return []FlatMedia{}, err
	}

	lists := result.Data.MediaListCollection.Lists

	flattened := make([]FlatMedia, 0)
	for _, list := range lists {
		for _, entry := range list.Entries {
			flat := FlatMedia{
				MediaID:    entry.Media.ID,
				MediaType:  entry.Media.Type,
				Status:     entry.Status,
				Format:     entry.Media.Format,
				Season:     entry.Media.Season,
				SeasonYear: entry.Media.SeasonYear,
				Title:      entry.Media.Title.Romaji,
				Image:      entry.Media.CoverImage.Large,
				Episodes:   entry.Media.Episodes,
				Score:      entry.Score,
				Progress:   entry.Progress,
				AltTitles:  []string{entry.Media.Title.English},
			}
			flattened = append(flattened, flat)
		}
	}

	return flattened, nil
}
