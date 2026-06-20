package com.benchmark.quarkus;

import io.quarkus.hibernate.orm.panache.PanacheEntityBase;
import jakarta.persistence.Entity;
import jakarta.persistence.Id;
import jakarta.persistence.Table;

@Entity
@Table(name = "World")
public class World extends PanacheEntityBase {
    @Id
    public int id;
    public int randomNumber;

    public World() {
    }

    public World(int id, int randomNumber) {
        this.id = id;
        this.randomNumber = randomNumber;
    }
}
