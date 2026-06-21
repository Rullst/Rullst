using System.ComponentModel.DataAnnotations.Schema;
using System.Text.Json;
using System.Text;
using Microsoft.EntityFrameworkCore;
using Npgsql.EntityFrameworkCore.PostgreSQL;

var builder = WebApplication.CreateBuilder(args);

// Configure Logging
builder.Logging.ClearProviders();

// Configure DB Context
var dsn = Environment.GetEnvironmentVariable("DATABASE_URL") ?? "Host=db;Username=benchmark;Password=benchmark;Database=benchmark;Port=5432;Include Error Detail=true";
builder.Services.AddDbContextPool<AppDbContext>(options =>
    options.UseNpgsql(dsn));

var app = builder.Build();

app.MapGet("/text", () => "Hello World");

app.MapGet("/json", () => new { message = "Hello World" });

app.MapGet("/db-single", async (AppDbContext db) =>
{
    // Fetch ID 1 for simplicity
    var world = await db.Worlds.FindAsync(1u);
    return world;
});

app.MapGet("/html", () =>
{
    return Results.Content("<!DOCTYPE html><html><head><title>Benchmark</title></head><body><h1>Hello World</h1></body></html>", "text/html", Encoding.UTF8);
});

var port = Environment.GetEnvironmentVariable("PORT") ?? "8080";
app.Run($"http://0.0.0.0:{port}");


public class AppDbContext : DbContext
{
    public AppDbContext(DbContextOptions<AppDbContext> options) : base(options) { }
    public DbSet<World> Worlds { get; set; }

    protected override void OnModelCreating(ModelBuilder modelBuilder)
    {
        modelBuilder.Entity<World>().ToTable("world");
    }
}

[Table("world")]
public class World
{
    [Column("id")]
    public uint Id { get; set; }

    [Column("randomnumber")]
    public uint RandomNumber { get; set; }
}
