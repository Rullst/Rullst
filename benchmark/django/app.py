import os
import sys
from django.conf import settings
from django.core.asgi import get_asgi_application
from django.http import HttpResponse, JsonResponse
from django.urls import path

if not settings.configured:
    settings.configure(
        DEBUG=False,
        SECRET_KEY="django-insecure-benchmark-secret-key-to-not-use-in-production",
        ROOT_URLCONF=__name__,
        ALLOWED_HOSTS=["*"],
        MIDDLEWARE=[
            "django.middleware.common.CommonMiddleware",
        ],
    )

def plaintext(request):
    return HttpResponse("Hello, World!", content_type="text/plain")

def json_endpoint(request):
    return JsonResponse({"message": "Hello, World!"})

urlpatterns = [
    path("", plaintext),
    path("json", json_endpoint),
]

app = get_asgi_application()

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000, log_level="warning")
