package error

import (
	"fmt"
	"log"

	"github.com/gin-gonic/gin"
	"github.com/troytabrilla/oshirase/api/api/sources"
)

type NotFoundError struct {
	MediaType string
	Message   string
}

func (n NotFoundError) Error() string {
	return fmt.Sprintf("Could not find %s %s", n.MediaType, n.Message)
}

func UnwrapError(err error) error {
	switch err := err.(type) {
	case *gin.Error:
		return err.Unwrap()
	default:
		return err
	}
}

// TODO Use interface for errors with status codes?
func GetStatusFromError(err error) int {
	var status int
	switch err := UnwrapError(err).(type) {
	case sources.AniListAPIError:
		status = err.GetStatus()
	case NotFoundError:
		status = 404
	default:
		status = 500
	}

	return status
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

	status := GetStatusFromError(last)
	var message string
	if status == 500 {
		message = "Whoops..."
	} else {
		message = last.Error()
	}

	c.JSON(status, gin.H{
		"status": status,
		"data": gin.H{
			"message": message,
		},
	})
}
