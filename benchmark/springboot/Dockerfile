FROM maven:3.9-eclipse-temurin-21-alpine AS builder
WORKDIR /app
COPY pom.xml ./
# Resolve dependencies to leverage caching
RUN mvn dependency:go-offline
COPY src ./src
RUN mvn package -DskipTests

FROM eclipse-temurin:17-jre-alpine
WORKDIR /app
COPY --from=builder /app/target/bench-springboot-0.0.1-SNAPSHOT.jar app.jar
EXPOSE 8080
# Use production settings for JVM to optimize performance
CMD ["java", "-XX:+UseG1GC", "-jar", "app.jar"]
