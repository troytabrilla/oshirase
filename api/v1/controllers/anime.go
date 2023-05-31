package controllers

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/models"
	"github.com/troytabrilla/oshirase/api/api/v1/sources"
)

type AnimeEntry struct{}

// TODO Implement
func (entry *AnimeEntry) GET(context *gin.Context) {
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Reading Anime %s", context.Param("id")),
		},
	})
}

// TODO Implement
// TODO Authenticate
func (entry *AnimeEntry) PUT(context *gin.Context) {
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Updating Anime %s", context.Param("id")),
		},
	})
}

type AnimeList struct {
	Config *conf.Config
}

func (list *AnimeList) GET(context *gin.Context) {
	anilist := models.AniList{Config: list.Config}
	userId := list.Config.AniList.API.UserID
	status := []string{}

	result, err := anilist.FetchList(userId, "ANIME", status)
	if err != nil {
		context.Error(err)

		var status int
		switch err := err.(type) {
		case sources.AniListAPIError:
			status = err.GetStatus()
		default:
			status = -1
		}

		context.AbortWithError(status, err)
		return
	}

	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data":   result,
	})
}

type AnimeSchedule struct{}

// TODO Implement
func (schedule *AnimeSchedule) GET(context *gin.Context) {
	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": "Anime Schedule",
		},
	})
}