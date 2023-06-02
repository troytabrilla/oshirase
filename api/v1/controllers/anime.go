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

	// TODO Fetch list, alt titles, schedule, latest simultaneously
	// TODO Aggregate results
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data":   result,
	})
}
