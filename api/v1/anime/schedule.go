package anime

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

type Schedule struct{}

// TODO Implement
func (s Schedule) GET(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"message": "Anime Schedule",
	})
}
