package main

import (
	"log"
	"net/http"
	"os"

	"github.com/gin-gonic/gin"
	"gorm.io/driver/postgres"
	"gorm.io/gorm"
)

type World struct {
	ID           uint32 `gorm:"primaryKey"`
	RandomNumber uint32
}

func (World) TableName() string {
	return "world"
}

func main() {
	gin.SetMode(gin.ReleaseMode)

	r := gin.New()

	dsn := os.Getenv("DATABASE_URL")
	if dsn == "" {
		dsn = "host=db user=benchmark password=benchmark dbname=benchmark port=5432 sslmode=disable"
	}
	db, err := gorm.Open(postgres.Open(dsn), &gorm.Config{
		PrepareStmt: true,
	})
	if err != nil {
		log.Fatalf("failed to connect database: %v", err)
	}

	sqlDB, err := db.DB()
	if err == nil {
		sqlDB.SetMaxOpenConns(500)
		sqlDB.SetMaxIdleConns(500)
	}

	r.GET("/text", func(c *gin.Context) {
		c.String(http.StatusOK, "Hello World")
	})

	r.GET("/json", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"message": "Hello World"})
	})

	r.GET("/db-single", func(c *gin.Context) {
		var world World
		// Just fetching ID 1 for simplicity in load testing, to measure ORM overhead without random generation overhead
		if err := db.First(&world, 1).Error; err != nil {
			c.String(http.StatusInternalServerError, "error")
			return
		}
		c.JSON(http.StatusOK, world)
	})

	r.GET("/html", func(c *gin.Context) {
		c.Data(http.StatusOK, "text/html; charset=utf-8", []byte(`<!DOCTYPE html><html><head><title>Benchmark</title></head><body><h1>Hello World</h1></body></html>`))
	})

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	r.Run(":" + port)
}
