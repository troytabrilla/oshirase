package controllers

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/models"
)

type MangaEntry struct{}

// TODO Implement
func (entry MangaEntry) GET(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Reading Manga %s", c.Param("id")),
		},
	})
}

// TODO Implement
// TODO Authenticate
func (entry MangaEntry) PUT(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Updating Manga %s", c.Param("id")),
		},
	})
}

type MangaList struct {
	Config *conf.Config
}

func (list MangaList) GET(context *gin.Context) {
	api := models.AniListAPI{Config: list.Config}
	userId := list.Config.AniListAPI.UserID
	status := []string{}

	result, err := api.Fetch(userId, "MANGA", status)
	if err != nil {
		context.Error(err)

		var status int
		switch err := err.(type) {
		case models.AniListAPIError:
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
