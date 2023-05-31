package main

import (
	"fmt"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/db"
	"github.com/troytabrilla/oshirase/api/api/error"
	v1 "github.com/troytabrilla/oshirase/api/api/v1"
)

func main() {
	router := gin.Default()

	router.Use(error.HandleErrors)

	router.StaticFile("favicon.ico", "./public/favicon.ico")

	config := conf.LoadConfig()

	mongodb := db.MongoDB{Config: &config}
	client := mongodb.GetClient()

	defer mongodb.CloseClient(client)

	apiGroup := router.Group("/api")
	{
		v1.AddRoutes(apiGroup.Group("/v1"), &config, client)
	}

	router.Run(fmt.Sprintf(":%d", config.API.Port))
}
