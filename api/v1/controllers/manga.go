package controllers

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	apierror "github.com/troytabrilla/oshirase/api/api/error"
	"github.com/troytabrilla/oshirase/api/api/v1/models"
)

type Manga struct {
	Config *conf.Config
}

// TODO Implement
func (manga *Manga) GetEntry(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Reading Manga %s", c.Param("id")),
		},
	})
}

// TODO Implement
// TODO Authenticate
func (manga *Manga) PutEntry(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Updating Manga %s", c.Param("id")),
		},
	})
}

func (manga *Manga) GetList(context *gin.Context) {
	anilist := models.AniList{Config: manga.Config}
	userId := manga.Config.Sources.AniList.API.UserID
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
