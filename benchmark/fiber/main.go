package main

import (
	"github.com/gofiber/fiber/v2"
)

type Message struct {
	Message string `json:"message"`
}

func main() {
	app := fiber.New(fiber.Config{
		DisableStartupMessage: true,
	})

	app.Get("/", func(c *fiber.Ctx) error {
		return c.SendString("Hello, World!")
	})

	app.Get("/json", func(c *fiber.Ctx) error {
		return c.JSON(Message{Message: "Hello, World!"})
	})

	app.Listen(":3000")
}
