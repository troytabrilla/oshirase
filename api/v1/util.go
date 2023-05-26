package v1

import (
	"fmt"
	"net/http"
	"strconv"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/error"
)

func LoadMediaByID(mediaType string) func(*gin.Context) {
	return func(c *gin.Context) {
		idParam := c.Param("id")
		id, err := strconv.Atoi(idParam)
		if err != nil {
			c.Error(err)
			c.AbortWithError(http.StatusNotFound, error.NotFoundError{MediaType: mediaType, Message: idParam})
			return
		}

		// TODO Load media, return 404 if not found
		fmt.Println("Fetching media:", id)

		c.Next()
	}
}
