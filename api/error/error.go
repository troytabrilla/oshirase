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

	for _, err := range c.Errors {
		log.Println(err)
	}

	last := c.Errors.Last()
	if last == nil {
		return
	}

	switch c.Writer.Status() {
	case 400:
		c.JSON(400, gin.H{
			"status": 400,
			"data": gin.H{
				"message": last.Error(),
			},
		})
	case 404:
		c.JSON(404, gin.H{
			"status": 404,
			"data": gin.H{
				"message": last.Error(),
			},
		})
	default:
		c.JSON(500, gin.H{
			"status": 500,
			"data": gin.H{
				"message": "Whoops...",
			},
		})
	}
}
