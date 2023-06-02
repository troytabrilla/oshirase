package controllers

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/models"
	"go.mongodb.org/mongo-driver/mongo"
)

type Internal struct {
	Config *conf.Config
	Client *mongo.Client
}

// TODO Internal routes should limit access to workers only
func (internal *Internal) GetAltTitles(context *gin.Context) {
	alt_title := models.AltTitle{Config: internal.Config, Client: internal.Client}

	result, err := alt_title.FetchAltTitles()
	if err != nil {
		context.Error(err)
		return
	}

	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data":   result,
	})
}

func (internal *Internal) GetCurrent(context *gin.Context) {
	anilist := models.AniList{Config: internal.Config}
	userId := internal.Config.Sources.AniList.API.UserID
	status := []string{"CURRENT"}

	// TODO Fetch simultaneously
	anime, err := anilist.FetchList(userId, "ANIME", status)
	if err != nil {
		context.Error(err)
		return
	}

	manga, err := anilist.FetchList(userId, "MANGA", status)
	if err != nil {
		context.Error(err)
		return
	}

	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"anime": anime,
			"manga": manga,
		},
	})
}

func (internal *Internal) GetLatest(context *gin.Context) {
	subsplease := models.SubsPlease{Config: internal.Config}
	result, err := subsplease.FetchLatest()
	if err != nil {
		context.Error(err)
		return
	}

	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data":   result,
	})
}

func (internal *Internal) GetSchedule(context *gin.Context) {
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": "Anime Schedule",
		},
	})
}
