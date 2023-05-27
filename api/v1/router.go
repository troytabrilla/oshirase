package v1

import (
	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/anime"
	"github.com/troytabrilla/oshirase/api/api/v1/manga"
)

var Schedule = anime.Schedule{}
var AnimeList anime.List
var Anime = anime.Anime{}
var MangaList = manga.List{}
var Manga = manga.Manga{}

func AddRoutes(group *gin.RouterGroup, config *conf.Config) {
	AnimeList = anime.List{Config: config}

	animeGroup := group.Group("/anime")
	{
		// TODO Move this to anime controller
		animeGroup.GET("/list", AnimeList.GET)
		animeGroup.GET("/schedule", Schedule.GET)
		idGroup := animeGroup.Group("/:id")
		{

			idGroup.Use(LoadMediaByID("anime"))
			idGroup.GET("", Anime.GET)
			idGroup.PUT("", Anime.PUT)
		}
	}

	mangaGroup := group.Group("/manga")
	{
		// TODO Move this to manga controller
		mangaGroup.GET("/list", MangaList.GET)
		idGroup := mangaGroup.Group("/:id")
		{
			idGroup.Use(LoadMediaByID("manga"))
			idGroup.GET("", Manga.GET)
			idGroup.PUT("", Manga.PUT)
		}
	}
}