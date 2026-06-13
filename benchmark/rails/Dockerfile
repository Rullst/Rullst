FROM ruby:3.3-alpine

# Install essential Linux packages
RUN apk add --no-cache build-base

WORKDIR /app

COPY Gemfile Gemfile.lock ./
RUN bundle install --without development test

COPY . .

EXPOSE 3000

ENV RAILS_ENV=production
ENV RACK_ENV=production
ENV RAILS_LOG_TO_STDOUT=true
ENV SECRET_KEY_BASE=dummysecretkeybaseforbenchmarkingpurposes

# Use Puma as the application server
CMD ["bundle", "exec", "puma", "-C", "config/puma.rb"]
