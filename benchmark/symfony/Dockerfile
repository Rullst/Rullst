FROM dunglas/frankenphp:php8.3-alpine

RUN apk add --no-cache \
    postgresql-dev \
    && docker-php-ext-install pdo pdo_pgsql opcache

COPY --from=composer:latest /usr/bin/composer /usr/bin/composer

WORKDIR /app

COPY . .
RUN composer install --optimize-autoloader --no-dev --classmap-authoritative

RUN mv "$PHP_INI_DIR/php.ini-production" "$PHP_INI_DIR/php.ini"

# Enable OPcache and JIT
RUN echo "opcache.enable=1" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini && \
    echo "opcache.jit=tracing" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini && \
    echo "opcache.jit_buffer_size=128M" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini && \
    echo "opcache.preload=/app/config/preload.php" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini && \
    echo "opcache.preload_user=root" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini

# Start frankenphp as worker for best perf
ENV SERVER_NAME=":8000"
CMD ["frankenphp", "php-server", "-r", "public"]
EXPOSE 8000
