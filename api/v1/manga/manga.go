package manga

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
)

type Manga struct{}

// TODO Implement
func (m Manga) GET(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Reading Manga %s", c.Param("id")),
		},
	})
}

// TODO Implement
// TODO Authenticate
func (m Manga) PUT(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status": 200,
		"data": gin.H{
			"message": fmt.Sprintf("Updating Manga %s", c.Param("id")),
		},
	})
}
