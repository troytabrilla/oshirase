package sources

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

type AniListAPI struct {
	Config *conf.Config
}

func (api *AniListAPI) Fetch(userId int, mediaType string, status []string) ([]byte, error) {
	query, err := os.ReadFile("../graphql/anilist/list_query.graphql")
	if err != nil {
		return []byte{}, err
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
		return []byte{}, err
	}

	req, err := http.NewRequest("POST", api.Config.AniList.API.URL, bytes.NewBuffer(body))
	if err != nil {
		return []byte{}, err
	}

	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Accept", "application/json")

	client := http.Client{}
	res, err := client.Do(req)
	if err != nil {
		return []byte{}, err
	}

	defer res.Body.Close()

	body, err = io.ReadAll(res.Body)
	if err != nil {
		return []byte{}, err
	}

	if res.Status != "200 OK" {
		var apiErr AniListAPIError
		err = json.Unmarshal(body, &apiErr)
		if err != nil {
			return []byte{}, err
		}

		return []byte{}, apiErr
	}

	return body, nil
}
