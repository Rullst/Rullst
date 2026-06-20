FROM php:8.3-fpm-alpine

RUN apk add --no-cache \
    postgresql-dev \
    && docker-php-ext-install pdo pdo_pgsql opcache

COPY --from=composer:latest /usr/bin/composer /usr/bin/composer

WORKDIR /var/www/html

COPY . .
RUN composer install --optimize-autoloader --no-dev

RUN mv "$PHP_INI_DIR/php.ini-production" "$PHP_INI_DIR/php.ini"

# Enable OPcache and JIT
RUN echo "opcache.enable=1" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini && \
    echo "opcache.jit=tracing" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini && \
    echo "opcache.jit_buffer_size=128M" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini && \
    echo "opcache.enable_cli=1" >> $PHP_INI_DIR/conf.d/docker-php-ext-opcache.ini

# Start octance with swoole if we were to use it, but standard php-fpm is default or artisan serve
# Since we want max perf, Laravel Octane with swoole is best.
RUN apk add --no-cache autoconf gcc g++ make linux-headers && \
    pecl install swoole && docker-php-ext-enable swoole && \
    apk del autoconf gcc g++ make linux-headers

RUN composer require laravel/octane
RUN php artisan octane:install --server=swoole

EXPOSE 8000
CMD ["php", "artisan", "octane:start", "--server=swoole", "--host=0.0.0.0", "--port=8000", "--workers=4", "--task-workers=2"]
