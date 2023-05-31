package v1

import (
	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/controllers"
	"go.mongodb.org/mongo-driver/mongo"
)

var AnimeList controllers.AnimeList
var MangaList controllers.MangaList
var AnimeSchedule controllers.AnimeSchedule
var AnimeEntry = controllers.AnimeEntry{}
var MangaEntry = controllers.MangaEntry{}

func AddRoutes(group *gin.RouterGroup, config *conf.Config, client *mongo.Client) {
	AnimeList = controllers.AnimeList{Config: config}
	AnimeSchedule = controllers.AnimeSchedule{Config: config}
	MangaList = controllers.MangaList{Config: config}

	animeGroup := group.Group("/anime")
	{
		animeGroup.GET("", AnimeList.GET)
		animeGroup.GET("/schedule", AnimeSchedule.GET)
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
