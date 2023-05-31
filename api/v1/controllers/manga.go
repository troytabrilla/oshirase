package controllers

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	apierror "github.com/troytabrilla/oshirase/api/api/error"
	"github.com/troytabrilla/oshirase/api/api/v1/models"
)

type MangaEntry struct{}

// TODO Implement
func (entry *MangaEntry) GET(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Reading Manga %s", c.Param("id")),
		},
	})
}

// TODO Implement
// TODO Authenticate
func (entry *MangaEntry) PUT(c *gin.Context) {
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

func (list *MangaList) GET(context *gin.Context) {
	anilist := models.AniList{Config: list.Config}
	userId := list.Config.AniList.API.UserID
	status := []string{}

	result, err := anilist.FetchList(userId, "MANGA", status)
	if err != nil {
		context.AbortWithError(apierror.GetStatusFromError(err), err)
		return
	}

	context.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data":   result,
	})
}
