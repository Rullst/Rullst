package com.benchmark.springboot;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.http.MediaType;
import org.springframework.stereotype.Controller;
import org.springframework.ui.Model;
import org.springframework.web.bind.annotation.GetMapping;
import org.springframework.web.bind.annotation.ResponseBody;

import java.util.Optional;
import java.util.concurrent.ThreadLocalRandom;

@Controller
public class BenchmarkController {

    private static final String HELLO_WORLD = "Hello World";

    @Autowired
    private WorldRepository worldRepository;

    @GetMapping(value = "/text", produces = MediaType.TEXT_PLAIN_VALUE)
    @ResponseBody
    public String text() {
        return HELLO_WORLD;
    }

    @GetMapping(value = "/json", produces = MediaType.APPLICATION_JSON_VALUE)
    @ResponseBody
    public Message json() {
        return new Message(HELLO_WORLD);
    }

    @GetMapping(value = "/db-single", produces = MediaType.APPLICATION_JSON_VALUE)
    @ResponseBody
    public World dbSingle() {
        int id = ThreadLocalRandom.current().nextInt(1, 10001);
        return worldRepository.findById(id).orElse(new World(id, 0));
    }

    @GetMapping("/html")
    public String html(Model model) {
        model.addAttribute("message", HELLO_WORLD);
        return "hello";
    }
}
