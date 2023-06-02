package controllers

import (
	"fmt"
	"strconv"

	"github.com/gin-gonic/gin"
	apierror "github.com/troytabrilla/oshirase/api/api/error"
)

func LoadMediaByID(mediaType string) func(*gin.Context) {
	return func(context *gin.Context) {
		idParam := context.Param("id")
		id, err := strconv.Atoi(idParam)
		if err != nil {
			context.AbortWithError(404, apierror.NotFoundError{
				MediaType: mediaType,
				Message:   idParam,
			})
			return
		}

		// TODO Load media, return 404 if not found
		fmt.Println("Fetching media:", id)

		context.Next()
	}
}
