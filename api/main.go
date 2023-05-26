package main

import (
	"github.com/gin-gonic/gin"
	error "github.com/troytabrilla/oshirase/api/api/error"
	v1 "github.com/troytabrilla/oshirase/api/api/v1"
)

func main() {
	router := gin.Default()

	router.Use(error.HandleErrors)

	router.StaticFile("favicon.ico", "./public/favicon.ico")

	apiGroup := router.Group("/api")
	{
		v1.AddRoutes(apiGroup.Group("/v1"))
	}

	router.Run()
}
