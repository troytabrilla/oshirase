package v1

import (
	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/controllers"
)

var AnimeList controllers.AnimeList
var MangaList controllers.MangaList
var Schedule = controllers.AnimeSchedule{}
var AnimeEntry = controllers.AnimeEntry{}
var MangaEntry = controllers.MangaEntry{}

func AddRoutes(group *gin.RouterGroup, config *conf.Config) {
	AnimeList = controllers.AnimeList{Config: config}
	MangaList = controllers.MangaList{Config: config}

	animeGroup := group.Group("/anime")
	{
		animeGroup.GET("", AnimeList.GET)
		animeGroup.GET("/schedule", Schedule.GET)
		idGroup := animeGroup.Group("/:id")
		{

			idGroup.Use(controllers.LoadMediaByID("anime"))
			idGroup.GET("", AnimeEntry.GET)
			idGroup.PUT("", AnimeEntry.PUT)
		}
	}

	mangaGroup := group.Group("/manga")
	{
		mangaGroup.GET("", MangaList.GET)
		idGroup := mangaGroup.Group("/:id")
		{
			idGroup.Use(controllers.LoadMediaByID("manga"))
			idGroup.GET("", MangaEntry.GET)
			idGroup.PUT("", MangaEntry.PUT)
		}
	}
}
