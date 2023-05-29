package models

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"os"

	"github.com/troytabrilla/oshirase/api/api/conf"
)

type AniListAPIErrorEntry struct {
	Message string `json:"message"`
	Status  int    `json:"status"`
}

type AniListAPIError struct {
	Errors []AniListAPIErrorEntry `json:"errors"`
}

func (err AniListAPIError) First() AniListAPIErrorEntry {
	if len(err.Errors) > 0 {
		return err.Errors[0]
	}

	return AniListAPIErrorEntry{
		Message: "Could not get AniList API error.",
		Status:  500,
	}
}

func (err AniListAPIError) Error() string {
	return err.First().Message
}

func (err AniListAPIError) GetStatus() int {
	return err.First().Status
}

type Variables struct {
	UserID   int      `json:"user_id"`
	Type     string   `json:"type"`
	StatusIn []string `json:"status_in"`
}

type Payload struct {
	Query     string    `json:"query"`
	Variables Variables `json:"variables"`
}

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

type AltTitles struct {
	MediaID   int
	AltTitles []string
}

type FlatMedia struct {
	MediaID      int
	MediaType    string
	Status       string
	Format       string
	Season       string
	SeasonYear   int
	Title        string
	EnglishTitle string
	Image        string
	Episodes     int
	Score        int
	Progress     int
	Schedule     Schedule
	Latest       Latest
	AltTitles    map[string]AltTitles
}

type AniListAPI struct {
	Config *conf.Config
}

func (api AniListAPI) Fetch(userId int, mediaType string, status []string) ([]FlatMedia, error) {
	query, err := os.ReadFile("../graphql/anilist/list_query.graphql")
	if err != nil {
		return []FlatMedia{}, err
	}

	if len(status) == 0 {
		status = []string{"CURRENT", "PLANNING", "COMPLETED", "DROPPED", "PAUSED", "REPEATING"}
	}

	payload := Payload{
		Query: string(query),
		Variables: Variables{
			UserID:   userId,
			Type:     mediaType,
			StatusIn: status,
		},
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return []FlatMedia{}, err
	}

	req, err := http.NewRequest("POST", api.Config.AniListAPI.URL, bytes.NewBuffer(body))
	if err != nil {
		return []FlatMedia{}, err
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")

	client := http.Client{}
	res, err := client.Do(req)
	if err != nil {
		return []FlatMedia{}, err
	}

	defer res.Body.Close()

	body, err = io.ReadAll(res.Body)
	if err != nil {
		return []FlatMedia{}, err
	}

	if res.Status != "200 OK" {
		var apiErr AniListAPIError
		err = json.Unmarshal(body, &apiErr)
		if err != nil {
			return []FlatMedia{}, err
		}

		return []FlatMedia{}, apiErr
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
				MediaID:      entry.Media.ID,
				MediaType:    entry.Media.Type,
				Status:       entry.Status,
				Format:       entry.Media.Format,
				Season:       entry.Media.Season,
				SeasonYear:   entry.Media.SeasonYear,
				Title:        entry.Media.Title.Romaji,
				EnglishTitle: entry.Media.Title.English,
				Image:        entry.Media.CoverImage.Large,
				Episodes:     entry.Media.Episodes,
				Score:        entry.Score,
				Progress:     entry.Progress,
			}
			flattened = append(flattened, flat)
		}
	}

	return flattened, nil
}
