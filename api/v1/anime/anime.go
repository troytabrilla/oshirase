package anime

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
)

type Anime struct{}

// TODO Implement
func (a Anime) GET(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Reading Anime %s", c.Param("id")),
		},
	})
}

// TODO Implement
// TODO Authenticate
func (a Anime) PUT(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Updating Anime %s", c.Param("id")),
		},
	})
}
