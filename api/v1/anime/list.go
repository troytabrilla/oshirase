package anime

import (
	"net/http"

	"github.com/gin-gonic/gin"
)

type List struct{}

// TODO Implement
func (l List) GET(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"message": "Anime List",
	})
}
