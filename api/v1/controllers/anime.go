package controllers

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/models"
	"go.mongodb.org/mongo-driver/mongo"
)

type Anime struct {
	Config *conf.Config
	Client *mongo.Client
}

// TODO Implement
func (anime *Anime) GetEntry(context *gin.Context) {
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Reading Anime %s", context.Param("id")),
		},
	})
}

// TODO Implement
// TODO Authenticate
func (anime *Anime) PutEntry(context *gin.Context) {
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Updating Anime %s", context.Param("id")),
		},
	})
}

func (anime *Anime) GetList(context *gin.Context) {
	anilist := models.AniList{Config: anime.Config}
	userId := anime.Config.Sources.AniList.API.UserID
	status := []string{}

	result, err := anilist.FetchList(userId, "ANIME", status)
	if err != nil {
		context.Error(err)
		return
	}

	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data":   result,
	})
}

func (anime *Anime) GetSchedule(context *gin.Context) {
	anilist := models.AniList{Config: anime.Config}
	subsplease := models.SubsPlease{Config: anime.Config}
	alt_title := models.AltTitle{Config: anime.Config, Client: anime.Client}

	userId := anime.Config.Sources.AniList.API.UserID
	status := []string{"CURRENT"}

	anilist_ch := make(chan []models.FlatMedia)
	subsplease_ch := make(chan map[string]models.SubsPleaseLatest)
	alt_title_ch := make(chan map[int]models.AltTitles)
	err_ch := make(chan error)

	go func() {
		result, err := anilist.FetchList(userId, "ANIME", status)
		if err != nil {
			err_ch <- err
			return
		}

		anilist_ch <- result
	}()

	go func() {
		result, err := subsplease.FetchLatest()
		if err != nil {
			err_ch <- err
			return
		}

		subsplease_ch <- result
	}()

	go func() {
		result, err := alt_title.FetchAltTitles()
		if err != nil {
			err_ch <- err
			return
		}

		alt_title_ch <- result
	}()

	var list []models.FlatMedia
	var latest map[string]models.SubsPleaseLatest
	var alt_titles map[int]models.AltTitles
	var err error

	for i := 0; i < 3; i++ {
		select {
		case list = <-anilist_ch:
		case latest = <-subsplease_ch:
		case alt_titles = <-alt_title_ch:
		case err = <-err_ch:
			context.Error(err)
			return
		}
	}

	// TODO Fetch schedule
	// TODO Aggregate results (simulate microservice for exp)
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"list":       list,
			"latest":     latest,
			"alt_titles": alt_titles,
		},
	})
}
