package main

import (
	"github.com/gin-gonic/gin"
)

type Message struct {
	Message string `json:"message"`
}

func main() {
	gin.SetMode(gin.ReleaseMode)
	r := gin.New()
	r.Use(gin.Recovery())

	r.GET("/", func(c *gin.Context) {
		c.String(200, "Hello, World!")
	})

	r.GET("/json", func(c *gin.Context) {
		c.JSON(200, Message{Message: "Hello, World!"})
	})

	r.Run(":3000")
}
