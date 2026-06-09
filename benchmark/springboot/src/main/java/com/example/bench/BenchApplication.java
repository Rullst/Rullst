package com.example.bench;

import org.springframework.boot.SpringApplication;
import org.springframework.boot.autoconfigure.SpringBootApplication;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.RestController;

import java.util.Collections;
import java.util.Map;

@SpringBootApplication
@RestController
public class BenchApplication {

	public static void main(String[] args) {
		SpringApplication.run(BenchApplication.class, args);
	}

	@GetMapping("/")
	public String plaintext() {
		return "Hello, World!";
	}

	@GetMapping("/json")
	public Map<String, String> json() {
		return Collections.singletonMap("message", "Hello, World!");
	}
}
