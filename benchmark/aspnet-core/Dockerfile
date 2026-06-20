FROM mcr.microsoft.com/dotnet/sdk:10.0-alpine AS build

WORKDIR /source

COPY AspNetCoreBench.csproj .
RUN dotnet restore

COPY . .
RUN dotnet publish -c Release -o /app

FROM mcr.microsoft.com/dotnet/aspnet:10.0-alpine

WORKDIR /app
COPY --from=build /app .

EXPOSE 8080
ENV DOTNET_EnableDiagnostics=0
ENV DOTNET_TieredPGO=1
ENV DOTNET_TieredCompilation=1

CMD ["./AspNetCoreBench"]
