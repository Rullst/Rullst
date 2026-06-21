package com.benchmark.springboot;

import org.springframework.beans.factory.annotation.Autowired;
import org.springframework.boot.CommandLineRunner;
import org.springframework.stereotype.Component;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ThreadLocalRandom;

@Component
public class DatabaseSeeder implements CommandLineRunner {

    @Autowired
    private WorldRepository worldRepository;

    @Override
    public void run(String... args) throws Exception {
        if (worldRepository.count() == 0) {
            System.out.println("Seeding database...");
            List<World> worlds = new ArrayList<>();
            for (int i = 1; i <= 10000; i++) {
                worlds.add(new World(i, ThreadLocalRandom.current().nextInt(1, 10001)));
            }
            worldRepository.saveAll(worlds);
            System.out.println("Database seeded with 10000 records.");
        }
    }
}
