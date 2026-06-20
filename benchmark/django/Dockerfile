FROM python:3.12-alpine

RUN apk add --no-cache postgresql-libs && \
    apk add --no-cache --virtual .build-deps gcc musl-dev postgresql-dev

WORKDIR /app
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt && \
    apk --purge del .build-deps

COPY . .

# We use gunicorn with gevent for max performance or standard sync workers.
# According to tech empower, gunicorn + meinheld or uvloop is best. We'll use gunicorn + gevent or standard workers.
RUN pip install gunicorn gevent

CMD ["gunicorn", "django_bench.wsgi:application", "--bind", "0.0.0.0:8000", "--workers", "4", "--worker-class", "gevent"]
EXPOSE 8000
