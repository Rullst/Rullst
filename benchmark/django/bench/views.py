from django.http import HttpResponse, JsonResponse
from django.shortcuts import render
from .models import World

def text(request):
    return HttpResponse("Hello World", content_type="text/plain")

def json(request):
    return JsonResponse({"message": "Hello World"})

def db_single(request):
    world = World.objects.get(id=1)
    return JsonResponse({"id": world.id, "text": world.text})

def html(request):
    return render(request, "bench/index.html", {"message": "Hello World"})
