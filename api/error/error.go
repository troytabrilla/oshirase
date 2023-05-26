package error

import (
	"fmt"
	"log"

	"github.com/gin-gonic/gin"
)

type NotFoundError struct {
	MediaType string
	Message   string
}

func (n NotFoundError) Error() string {
	return fmt.Sprintf("Could not find %s %s", n.MediaType, n.Message)
}

func HandleErrors(c *gin.Context) {
	c.Next()

	logger := log.Default()
	for _, err := range c.Errors {
		logger.Println(err)
	}

	last := c.Errors.Last()
	if last == nil {
		return
	}

	switch c.Writer.Status() {
	case 404:
		c.JSON(404, gin.H{
			"status":  404,
			"message": last.Error(),
		})
	default:
		c.JSON(500, gin.H{
			"status":  500,
			"message": "Whoops...",
		})
	}
}
