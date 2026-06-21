package main

import (
	"log"
	"os"

	"github.com/gofiber/fiber/v2"
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
	app := fiber.New(fiber.Config{
		DisableStartupMessage: true,
	})

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

	app.Get("/text", func(c *fiber.Ctx) error {
		return c.SendString("Hello World")
	})

	app.Get("/json", func(c *fiber.Ctx) error {
		return c.JSON(fiber.Map{"message": "Hello World"})
	})

	app.Get("/db-single", func(c *fiber.Ctx) error {
		var world World
		// Fetch ID 1 for simplicity to measure ORM overhead
		if err := db.First(&world, 1).Error; err != nil {
			return c.Status(fiber.StatusInternalServerError).SendString("error")
		}
		return c.JSON(world)
	})

	app.Get("/html", func(c *fiber.Ctx) error {
		c.Set(fiber.HeaderContentType, fiber.MIMETextHTMLCharsetUTF8)
		return c.SendString(`<!DOCTYPE html><html><head><title>Benchmark</title></head><body><h1>Hello World</h1></body></html>`)
	})

	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}
	app.Listen(":" + port)
}
