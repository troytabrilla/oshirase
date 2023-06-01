package v1

import (
	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/controllers"
	"go.mongodb.org/mongo-driver/mongo"
)

var Anime controllers.Anime
var Manga controllers.Manga

func AddRoutes(group *gin.RouterGroup, config *conf.Config, client *mongo.Client) {
	Anime = controllers.Anime{Config: config}
	Manga = controllers.Manga{Config: config}

	animeGroup := group.Group("/anime")
	{
		animeGroup.GET("", Anime.GetList)
		animeGroup.GET("/schedule", Anime.GetSchedule)
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
}
