package com.benchmark.quarkus;

import io.quarkus.qute.Template;
import io.quarkus.qute.TemplateInstance;
import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;

import java.util.concurrent.ThreadLocalRandom;

@Path("/")
public class BenchmarkResource {

    private static final String HELLO_WORLD = "Hello World";

    @Inject
    Template hello;

    @GET
    @Path("/text")
    @Produces(MediaType.TEXT_PLAIN)
    public String text() {
        return HELLO_WORLD;
    }

    @GET
    @Path("/json")
    @Produces(MediaType.APPLICATION_JSON)
    public Message json() {
        return new Message(HELLO_WORLD);
    }

    @GET
    @Path("/db-single")
    @Produces(MediaType.APPLICATION_JSON)
    public World dbSingle() {
        int id = ThreadLocalRandom.current().nextInt(1, 10001);
        World world = World.findById(id);
        if (world == null) {
            return new World(id, 0);
        }
        return world;
    }

    @GET
    @Path("/html")
    @Produces(MediaType.TEXT_HTML)
    public TemplateInstance html() {
        return hello.data("message", HELLO_WORLD);
    }
}
