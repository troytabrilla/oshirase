package controllers

import (
	"fmt"
	"strconv"

	"github.com/gin-gonic/gin"
)

func LoadMediaByID(mediaType string) func(*gin.Context) {
	return func(context *gin.Context) {
		idParam := context.Param("id")
		id, err := strconv.Atoi(idParam)
		if err != nil {
			context.Error(err)
			return
		}

		// TODO Load media, return 404 if not found
		fmt.Println("Fetching media:", id)

		context.Next()
	}
}
