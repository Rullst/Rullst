FROM public.ecr.aws/docker/library/ruby:3.2-bullseye

RUN apt-get update -y && apt-get install -y build-essential libpq-dev tzdata nodejs

WORKDIR /app

COPY Gemfile Gemfile.lock ./
RUN bundle config set --local without 'development test' && bundle install

COPY . .

ENV RAILS_ENV=production
ENV RAILS_LOG_TO_STDOUT=true
ENV SECRET_KEY_BASE=dummy_secret

CMD ["bundle", "exec", "rails", "server", "-b", "0.0.0.0"]
