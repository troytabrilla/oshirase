package v1

import (
	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/controllers"
	"go.mongodb.org/mongo-driver/mongo"
)

var Anime controllers.Anime
var Manga controllers.Manga
var Internal controllers.Internal

func AddRoutes(group *gin.RouterGroup, config *conf.Config, client *mongo.Client) {
	Anime = controllers.Anime{Config: config, Client: client}
	Manga = controllers.Manga{Config: config}
	Internal = controllers.Internal{Config: config, Client: client}

	animeGroup := group.Group("/anime")
	{
		animeGroup.GET("", Anime.GetList)
		idGroup := animeGroup.Group("/:id")
		{

			idGroup.Use(controllers.LoadMediaByID("anime"))
			idGroup.GET("", Anime.GetEntry)
			idGroup.PUT("", Anime.PutEntry)
		}
	}

	mangaGroup := group.Group("/manga")
	{
		mangaGroup.GET("", Manga.GetList)
		idGroup := mangaGroup.Group("/:id")
		{
			idGroup.Use(controllers.LoadMediaByID("manga"))
			idGroup.GET("", Manga.GetEntry)
			idGroup.PUT("", Manga.PutEntry)
		}
	}

	internalGroup := group.Group("/internal")
	{
		internalGroup.GET("/alt_titles", Internal.GetAltTitles)
		internalGroup.GET("/current", Internal.GetCurrent)
		internalGroup.GET("/latest", Internal.GetLatest)
		internalGroup.GET("/schedule", Internal.GetSchedule)
	}
}
