package anime

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
)

// TODO Move structs to models
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

type Data struct {
	MediaListCollection MediaListCollection
}

type Result struct {
	Data Data `json:"data"`
}

type AniListAPIErrorEntry struct {
	Message string `json:"message"`
	Status  int    `json:"status"`
}

type AniListAPIError struct {
	Errors []AniListAPIErrorEntry `json:"errors"`
}

func (a AniListAPIError) First() AniListAPIErrorEntry {
	if len(a.Errors) > 0 {
		return a.Errors[0]
	}

	return AniListAPIErrorEntry{
		Message: "Could not get AniList API error.",
		Status:  500,
	}
}

func (a AniListAPIError) Error() string {
	return a.First().Message
}

func (a AniListAPIError) GetStatus() int {
	return a.First().Status
}

func loadQuery() (string, error) {
	query, err := os.ReadFile("../graphql/anilist/list_query.graphql")
	if err != nil {
		return "", err
	}

	return string(query), nil
}

func handleError(c *gin.Context, e *error) {
	c.Error(*e)
	c.AbortWithError(http.StatusInternalServerError, *e)
}

type List struct {
	Config *conf.Config
}

func (list List) GET(context *gin.Context) {
	query, err := loadQuery()
	if err != nil {
		handleError(context, &err)
		return
	}

	payload := Payload{
		Query: query,
		Variables: Variables{
			UserID:   list.Config.AniListAPI.UserID,
			Type:     "ANIME",
			StatusIn: []string{"CURRENT", "PLANNING", "COMPLETED", "DROPPED", "PAUSED", "REPEATING"},
		},
	}

	body, err := json.Marshal(payload)
	if err != nil {
		handleError(context, &err)
		return
	}

	req, err := http.NewRequest("POST", list.Config.AniListAPI.URL, bytes.NewBuffer(body))
	if err != nil {
		handleError(context, &err)
		return
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")

	client := http.Client{}
	res, err := client.Do(req)
	if err != nil {
		handleError(context, &err)
		return
	}

	defer res.Body.Close()

	body, err = io.ReadAll(res.Body)
	if err != nil {
		handleError(context, &err)
		return
	}

	if res.Status != "200 OK" {
		var apiErr AniListAPIError
		err = json.Unmarshal(body, &apiErr)
		if err != nil {
			handleError(context, &err)
			return
		}

		context.AbortWithError(apiErr.GetStatus(), apiErr)
		return
	}

	var result Result
	err = json.Unmarshal(body, &result)
	if err != nil {
		handleError(context, &err)
		return
	}

	// TODO Flatten lists
	// TODO Refactor into controllers, models, and sources
	// TODO Controllers need to decide which models to use, models need to decide which sources to use
	// TODO Write transformer/aggregator in C++ to combine source data into a single list,
	// figure out best way to optimize, extra data can vary, but base is always anilist source + alt titles
	// TODO Move configs and graphql to root level, make config and graphql paths environment variables or cli args
	// TODO Add tests
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data":   result.Data.MediaListCollection.Lists,
	})
}
