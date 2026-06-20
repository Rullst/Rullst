package com.benchmark.quarkus;

import io.quarkus.runtime.StartupEvent;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.transaction.Transactional;

import java.util.concurrent.ThreadLocalRandom;

@ApplicationScoped
public class DatabaseSeeder {

    @Transactional
    void onStart(@Observes StartupEvent ev) {
        if (World.count() == 0) {
            System.out.println("Seeding database...");
            for (int i = 1; i <= 10000; i++) {
                World world = new World(i, ThreadLocalRandom.current().nextInt(1, 10001));
                world.persist();
            }
            System.out.println("Database seeded with 10000 records.");
        }
    }
}
